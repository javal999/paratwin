//! ParaTwin host layer: persistence + service functions shared by the Tauri
//! desktop app and the browser dev server.
//!
//! Storage split per the ParaExpress spec:
//! - SQLite  : scenarios, runs, KPI summaries (relational, queryable)
//! - RocksDB : high-volume animation frames + per-run time series
//!             (fast sequential writes, range scans by key prefix)

use anyhow::{anyhow, Context, Result};
use paratwin_engine as engine;
use engine::model::{Frame, RunResult, Scenario};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Host {
    db: Mutex<Connection>,
    frames: rocksdb::DB,
    #[allow(dead_code)]
    data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRow {
    pub id: i64,
    pub name: String,
    pub updated_at: String,
    pub scenario: Scenario,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRow {
    pub id: i64,
    pub scenario_id: i64,
    pub scenario_name: String,
    pub created_at: String,
    pub sla: f64,
    pub contribution_idr: f64,
    pub orders_per_day: f64,
    pub n_nodes: i64,
    pub frame_count: i64,
}

/// Why the data store could not be opened — lets the app show a useful
/// message instead of crashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFailure {
    /// Another ParaTwin process holds the frame-store lock.
    AlreadyRunning,
    /// Anything else (permissions, disk, corruption).
    Other,
}

/// Classify an open error; RocksDB reports a held lock as an IO/lock error.
pub fn classify_open_error(e: &anyhow::Error) -> OpenFailure {
    let msg = format!("{e:#}").to_lowercase();
    if msg.contains("lock") || msg.contains("temporarily unavailable") {
        OpenFailure::AlreadyRunning
    } else {
        OpenFailure::Other
    }
}

impl Host {
    /// Open with a few short retries — covers the race where a previous
    /// instance is still releasing its lock during shutdown.
    pub fn open_with_retry(data_dir: &Path, attempts: u32) -> Result<Self> {
        let mut last = None;
        for i in 0..attempts.max(1) {
            match Self::open(data_dir) {
                Ok(h) => return Ok(h),
                Err(e) => {
                    if classify_open_error(&e) != OpenFailure::AlreadyRunning {
                        return Err(e); // not a lock problem: fail fast
                    }
                    last = Some(e);
                    if i + 1 < attempts {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                    }
                }
            }
        }
        Err(last.unwrap_or_else(|| anyhow!("failed to open data store")))
    }

    pub fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db = Connection::open(data_dir.join("paratwin.sqlite"))?;
        db.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS scenarios(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                json TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS runs(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                scenario_id INTEGER NOT NULL,
                scenario_name TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                result_json TEXT NOT NULL,
                sla REAL NOT NULL,
                contribution REAL NOT NULL,
                orders_per_day REAL NOT NULL,
                n_nodes INTEGER NOT NULL,
                frame_count INTEGER NOT NULL
            );
            "#,
        )?;
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        let frames = rocksdb::DB::open(&opts, data_dir.join("frames.rocks"))
            .context("open rocksdb")?;
        Ok(Host { db: Mutex::new(db), frames, data_dir: data_dir.to_path_buf() })
    }

    // ---------- scenarios ----------

    pub fn seed_presets(&self) -> Result<()> {
        let n: i64 = self
            .db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM scenarios", [], |r| r.get(0))?;
        if n == 0 {
            for p in ["baseline", "pilot", "scaleout"] {
                let sc = engine::preset(p);
                self.save_scenario(None, &sc)?;
            }
        }
        Ok(())
    }

    pub fn list_scenarios(&self) -> Result<Vec<ScenarioRow>> {
        let db = self.db.lock().unwrap();
        let mut st =
            db.prepare("SELECT id, name, json, updated_at FROM scenarios ORDER BY id")?;
        let rows = st
            .query_map([], |r| {
                let json: String = r.get(2)?;
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, json, r.get::<_, String>(3)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(id, name, json, updated_at)| {
                Ok(ScenarioRow {
                    id,
                    name,
                    updated_at,
                    scenario: serde_json::from_str(&json)?,
                })
            })
            .collect()
    }

    pub fn get_scenario(&self, id: i64) -> Result<Scenario> {
        let db = self.db.lock().unwrap();
        let json: String = db
            .query_row("SELECT json FROM scenarios WHERE id=?1", [id], |r| r.get(0))
            .map_err(|_| anyhow!("scenario {id} not found"))?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn save_scenario(&self, id: Option<i64>, sc: &Scenario) -> Result<i64> {
        sc.validate().map_err(|e| anyhow!(e))?;
        let json = serde_json::to_string(sc)?;
        let db = self.db.lock().unwrap();
        match id {
            Some(id) => {
                db.execute(
                    "UPDATE scenarios SET name=?1, json=?2, updated_at=datetime('now') WHERE id=?3",
                    rusqlite::params![sc.name, json, id],
                )?;
                Ok(id)
            }
            None => {
                db.execute(
                    "INSERT INTO scenarios(name, json) VALUES(?1, ?2)",
                    rusqlite::params![sc.name, json],
                )?;
                Ok(db.last_insert_rowid())
            }
        }
    }

    pub fn delete_scenario(&self, id: i64) -> Result<()> {
        self.db
            .lock()
            .unwrap()
            .execute("DELETE FROM scenarios WHERE id=?1", [id])?;
        Ok(())
    }

    // ---------- runs ----------

    pub fn run_simulation(&self, scenario_id: i64) -> Result<(i64, RunResult)> {
        let sc = self.get_scenario(scenario_id)?;
        let mc = engine::montecarlo::run_monte_carlo(&sc).map_err(|e| anyhow!(e))?;
        let result_json = serde_json::to_string(&mc.result)?;
        let run_id: i64 = {
            let db = self.db.lock().unwrap();
            db.execute(
                "INSERT INTO runs(scenario_id, scenario_name, result_json, sla, contribution,
                 orders_per_day, n_nodes, frame_count) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                rusqlite::params![
                    scenario_id,
                    sc.name,
                    result_json,
                    mc.result.kpi.sla_attainment,
                    mc.result.kpi.contribution_idr,
                    sc.orders_per_day,
                    sc.nodes.len() as i64,
                    mc.frames.len() as i64
                ],
            )?;
            db.last_insert_rowid()
        };
        // frames to RocksDB: key = run_id (8 bytes BE) + minute (4 bytes BE)
        let mut batch = rocksdb::WriteBatch::default();
        for f in &mc.frames {
            let mut key = Vec::with_capacity(12);
            key.extend_from_slice(&run_id.to_be_bytes());
            key.extend_from_slice(&f.minute.to_be_bytes());
            batch.put(key, serde_json::to_vec(f)?);
        }
        self.frames.write(batch)?;
        Ok((run_id, mc.result))
    }

    pub fn list_runs(&self) -> Result<Vec<RunRow>> {
        let db = self.db.lock().unwrap();
        let mut st = db.prepare(
            "SELECT id, scenario_id, scenario_name, created_at, sla, contribution,
             orders_per_day, n_nodes, frame_count FROM runs ORDER BY id DESC LIMIT 100",
        )?;
        let rows = st
            .query_map([], |r| {
                Ok(RunRow {
                    id: r.get(0)?,
                    scenario_id: r.get(1)?,
                    scenario_name: r.get(2)?,
                    created_at: r.get(3)?,
                    sla: r.get(4)?,
                    contribution_idr: r.get(5)?,
                    orders_per_day: r.get(6)?,
                    n_nodes: r.get(7)?,
                    frame_count: r.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_run(&self, run_id: i64) -> Result<RunResult> {
        let db = self.db.lock().unwrap();
        let json: String = db
            .query_row("SELECT result_json FROM runs WHERE id=?1", [run_id], |r| r.get(0))
            .map_err(|_| anyhow!("run {run_id} not found"))?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn get_run_scenario(&self, run_id: i64) -> Result<Scenario> {
        let sid: i64 = self
            .db
            .lock()
            .unwrap()
            .query_row("SELECT scenario_id FROM runs WHERE id=?1", [run_id], |r| r.get(0))?;
        self.get_scenario(sid)
    }

    pub fn get_frames(&self, run_id: i64, from_min: u32, to_min: u32) -> Result<Vec<Frame>> {
        let mut lo = Vec::with_capacity(12);
        lo.extend_from_slice(&run_id.to_be_bytes());
        lo.extend_from_slice(&from_min.to_be_bytes());
        let mut out = Vec::new();
        let iter = self.frames.iterator(rocksdb::IteratorMode::From(
            &lo,
            rocksdb::Direction::Forward,
        ));
        for item in iter {
            let (k, v) = item?;
            if k.len() != 12 || &k[0..8] != run_id.to_be_bytes().as_slice() {
                break;
            }
            let minute = u32::from_be_bytes([k[8], k[9], k[10], k[11]]);
            if minute > to_min {
                break;
            }
            out.push(serde_json::from_slice(&v)?);
        }
        Ok(out)
    }

    pub fn delete_run(&self, run_id: i64) -> Result<()> {
        self.db
            .lock()
            .unwrap()
            .execute("DELETE FROM runs WHERE id=?1", [run_id])?;
        let mut lo = Vec::with_capacity(12);
        lo.extend_from_slice(&run_id.to_be_bytes());
        lo.extend_from_slice(&0u32.to_be_bytes());
        let mut hi = Vec::with_capacity(12);
        hi.extend_from_slice(&run_id.to_be_bytes());
        hi.extend_from_slice(&u32::MAX.to_be_bytes());
        let mut batch = rocksdb::WriteBatch::default();
        batch.delete_range(lo, hi);
        self.frames.write(batch)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_persistence() {
        let dir = std::env::temp_dir().join(format!("paratwin-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let host = Host::open(&dir).unwrap();
        host.seed_presets().unwrap();
        let scs = host.list_scenarios().unwrap();
        assert_eq!(scs.len(), 3);

        // fast run: shrink the baseline preset
        let mut sc = scs[0].scenario.clone();
        sc.days = 1;
        sc.replications = 2;
        let sid = host.save_scenario(Some(scs[0].id), &sc).unwrap();
        let (run_id, result) = host.run_simulation(sid).unwrap();
        assert!(result.kpi.orders_total > 0.0);

        let runs = host.list_runs().unwrap();
        assert_eq!(runs.len(), 1);
        let frames = host.get_frames(run_id, 0, 200).unwrap();
        assert!(!frames.is_empty(), "frames should be stored in rocksdb");
        assert!(frames.windows(2).all(|w| w[0].minute <= w[1].minute));

        let back = host.get_run(run_id).unwrap();
        assert_eq!(back.kpi.orders_total, result.kpi.orders_total);

        // delete_run must remove both the SQLite row and the RocksDB frames
        host.delete_run(run_id).unwrap();
        assert!(host.get_run(run_id).is_err());
        assert!(host.get_frames(run_id, 0, 100000).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
