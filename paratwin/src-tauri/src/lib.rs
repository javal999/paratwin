//! ParaTwin desktop app: Tauri v2 shell over the shared host layer.
//! Every command mirrors the devserver dispatch table one-to-one.

use paratwin_engine as engine;
use paratwin_host::{classify_open_error, Host, OpenFailure, RunRow, ScenarioRow};
use engine::model::{Frame, RunResult, Scenario};
use serde::Serialize;
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

type H<'a> = State<'a, Arc<Host>>;

fn e2s(e: anyhow::Error) -> String {
    e.to_string()
}

#[derive(Serialize)]
struct SaveOut {
    id: i64,
}

#[derive(Serialize)]
struct RunOut {
    run_id: i64,
    result: RunResult,
}

#[tauri::command(rename_all = "snake_case")]
async fn list_scenarios(h: H<'_>) -> Result<Vec<ScenarioRow>, String> {
    h.list_scenarios().map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn get_scenario(h: H<'_>, id: i64) -> Result<Scenario, String> {
    h.get_scenario(id).map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn save_scenario(h: H<'_>, id: Option<i64>, scenario: Scenario) -> Result<SaveOut, String> {
    h.save_scenario(id, &scenario).map(|id| SaveOut { id }).map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn delete_scenario(h: H<'_>, id: i64) -> Result<(), String> {
    h.delete_scenario(id).map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn preset_scenario(name: String) -> Result<Scenario, String> {
    Ok(engine::preset(&name))
}

#[tauri::command(rename_all = "snake_case")]
async fn auto_layout(
    metro_km: f64,
    n_dark: u32,
    n_store: u32,
    flagship: bool,
    orders_per_day: f64,
) -> Result<Vec<engine::model::FulfillNode>, String> {
    Ok(engine::auto_layout(metro_km, n_dark, n_store, flagship, orders_per_day))
}

#[tauri::command(rename_all = "snake_case")]
async fn run_simulation(h: H<'_>, scenario_id: i64) -> Result<RunOut, String> {
    let host = h.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        host.run_simulation(scenario_id)
            .map(|(run_id, result)| RunOut { run_id, result })
            .map_err(e2s)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command(rename_all = "snake_case")]
async fn list_runs(h: H<'_>) -> Result<Vec<RunRow>, String> {
    h.list_runs().map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn get_run(h: H<'_>, run_id: i64) -> Result<RunResult, String> {
    h.get_run(run_id).map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn get_run_scenario(h: H<'_>, run_id: i64) -> Result<Scenario, String> {
    h.get_run_scenario(run_id).map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn get_frames(h: H<'_>, run_id: i64, from_min: u32, to_min: u32) -> Result<Vec<Frame>, String> {
    h.get_frames(run_id, from_min, to_min).map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn delete_run(h: H<'_>, run_id: i64) -> Result<(), String> {
    h.delete_run(run_id).map_err(e2s)
}

#[tauri::command(rename_all = "snake_case")]
async fn validate_math() -> Result<Vec<engine::ValidationRow>, String> {
    tauri::async_runtime::spawn_blocking(|| Ok(engine::validate_math()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command(rename_all = "snake_case")]
async fn network_sweep(
    h: H<'_>,
    scenario_id: i64,
    n_from: u32,
    n_to: u32,
) -> Result<Vec<engine::SweepPoint>, String> {
    let host = h.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let sc = host.get_scenario(scenario_id).map_err(e2s)?;
        Ok(engine::network_sweep(&sc, n_from, n_to))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command(rename_all = "snake_case")]
async fn compare_partnerships(
    h: H<'_>,
    scenario_id: i64,
) -> Result<Vec<engine::PartnershipOption>, String> {
    let host = h.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let sc = host.get_scenario(scenario_id).map_err(e2s)?;
        Ok(engine::compare_partnerships(&sc))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command(rename_all = "snake_case")]
async fn rank_stores(
    h: H<'_>,
    scenario_id: i64,
    take: usize,
) -> Result<Vec<engine::StoreRankRow>, String> {
    let sc = h.get_scenario(scenario_id).map_err(e2s)?;
    Ok(engine::rank_store_activations(&sc, take))
}

/// Show a native error dialog and exit cleanly. Never panics: a panic here
/// would unwind into an Objective-C callback and abort the process.
fn fatal(app: &tauri::AppHandle, title: &str, body: &str) {
    eprintln!("ParaTwin fatal: {title} — {body}");
    app.dialog()
        .message(body)
        .kind(MessageDialogKind::Error)
        .title(title)
        .buttons(MessageDialogButtons::Ok)
        .blocking_show();
    app.exit(1);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // must be registered first: a second launch focuses the running
        // window instead of starting a process that would fail to take the
        // data-store lock.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.webview_windows().values().next() {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let dir = match app.path().app_data_dir() {
                Ok(d) => d,
                Err(e) => {
                    fatal(app.handle(), "ParaTwin cannot start",
                        &format!("macOS did not provide an application data folder.\n\nDetails: {e}"));
                    return Ok(());
                }
            };
            // a few short retries cover a previous instance still shutting down
            match Host::open_with_retry(&dir, 4) {
                Ok(h) => {
                    let host = Arc::new(h);
                    if let Err(e) = host.seed_presets() {
                        // non-fatal: the app is usable with an empty list
                        eprintln!("ParaTwin: could not seed preset scenarios: {e:#}");
                    }
                    app.manage(host);
                }
                Err(e) => {
                    let (title, body) = match classify_open_error(&e) {
                        OpenFailure::AlreadyRunning => (
                            "ParaTwin is already running",
                            format!(
                                "Another copy of ParaTwin has the simulation database open, so this \
                                 one cannot start.\n\nSwitch to the running ParaTwin window (check \
                                 the Dock), or quit it and open this copy again.\n\nData folder: {}",
                                dir.display()
                            ),
                        ),
                        OpenFailure::Other => (
                            "ParaTwin cannot open its data store",
                            format!(
                                "The scenario and simulation database could not be opened.\n\n\
                                 Details: {e:#}\n\nData folder: {}",
                                dir.display()
                            ),
                        ),
                    };
                    fatal(app.handle(), title, &body);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_scenarios,
            get_scenario,
            save_scenario,
            delete_scenario,
            preset_scenario,
            auto_layout,
            run_simulation,
            list_runs,
            get_run,
            get_run_scenario,
            get_frames,
            delete_run,
            validate_math,
            network_sweep,
            compare_partnerships,
            rank_stores
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("ParaTwin failed to start: {e}");
            std::process::exit(1);
        });
}
