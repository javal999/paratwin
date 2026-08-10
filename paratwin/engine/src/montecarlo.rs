//! Monte Carlo replication layer: runs R independent replications with
//! common-random-number seeding, aggregates KPI means and 95% confidence
//! intervals (Student-t) as required by the ParaTwin output contract Y.

use crate::des::simulate;
use crate::model::*;
use std::collections::BTreeMap;

/// two-sided 95% Student-t critical values for df = 1..30 (df>30 -> 1.96)
fn t95(df: usize) -> f64 {
    const T: [f64; 30] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179,
        2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064,
        2.060, 2.056, 2.052, 2.048, 2.045, 2.042,
    ];
    if df == 0 {
        f64::NAN
    } else if df <= 30 {
        T[df - 1]
    } else {
        1.96
    }
}

fn ci(vals: &[f64]) -> CiStat {
    let n = vals.len() as f64;
    let mean = vals.iter().sum::<f64>() / n;
    if vals.len() < 2 {
        return CiStat { mean, half_width: 0.0 };
    }
    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    CiStat { mean, half_width: t95(vals.len() - 1) * (var / n).sqrt() }
}

pub struct McOutput {
    pub result: RunResult,
    pub frames: Vec<Frame>,
}

pub fn run_monte_carlo(sc: &Scenario) -> Result<McOutput, String> {
    sc.validate()?;
    let mut reps: Vec<crate::des::SimOutput> = Vec::new();
    for r in 0..sc.replications {
        let capture = r == 0;
        reps.push(simulate(sc, sc.seed.wrapping_add(r as u64 * 7919), capture));
    }

    // mean KPI across replications (field-wise)
    let kpis: Vec<KpiSummary> = reps.iter().map(|r| r.kpi.clone()).collect();
    let mean_kpi = mean_kpis(&kpis);

    let mut cim: BTreeMap<String, CiStat> = BTreeMap::new();
    let pull = |f: fn(&KpiSummary) -> f64| kpis.iter().map(f).collect::<Vec<_>>();
    cim.insert("sla_attainment".into(), ci(&pull(|k| k.sla_attainment)));
    cim.insert("p95_click_to_door_min".into(), ci(&pull(|k| k.p95_click_to_door_min)));
    cim.insert("contribution_idr".into(), ci(&pull(|k| k.contribution_idr)));
    cim.insert("contribution_per_order".into(), ci(&pull(|k| k.contribution_per_order)));
    cim.insert("fill_rate".into(), ci(&pull(|k| k.fill_rate)));
    cim.insert("lost_sales".into(), ci(&pull(|k| k.lost_sales)));
    cim.insert("promise_shown_rate".into(), ci(&pull(|k| k.promise_shown_rate)));
    cim.insert("picker_utilization".into(), ci(&pull(|k| k.picker_utilization)));

    // hourly / node / zone series averaged across replications
    let hourly = (0..24)
        .map(|h| {
            let n = reps.len() as f64;
            HourPoint {
                hour: h as u32,
                orders: reps.iter().map(|r| r.hourly[h].orders).sum::<f64>() / n,
                on_time_rate: reps.iter().map(|r| r.hourly[h].on_time_rate).sum::<f64>() / n,
                avg_ctd_min: reps.iter().map(|r| r.hourly[h].avg_ctd_min).sum::<f64>() / n,
                queue_len: reps.iter().map(|r| r.hourly[h].queue_len).sum::<f64>() / n,
            }
        })
        .collect();
    let n_nodes = reps[0].nodes.len();
    let nodes = (0..n_nodes)
        .map(|i| {
            let n = reps.len() as f64;
            let base = &reps[0].nodes[i];
            NodeStat {
                id: base.id,
                kind: base.kind,
                x_km: base.x_km,
                y_km: base.y_km,
                orders: reps.iter().map(|r| r.nodes[i].orders).sum::<f64>() / n,
                utilization: reps.iter().map(|r| r.nodes[i].utilization).sum::<f64>() / n,
                on_time_rate: reps.iter().map(|r| r.nodes[i].on_time_rate).sum::<f64>() / n,
                stockout_rate: reps.iter().map(|r| r.nodes[i].stockout_rate).sum::<f64>() / n,
            }
        })
        .collect();
    let zones = reps[0].zones.clone();
    let frames = reps[0].frames.clone();

    Ok(McOutput {
        result: RunResult {
            kpi: mean_kpi,
            ci: cim,
            hourly,
            nodes,
            zones,
            replication_kpis: kpis,
        },
        frames,
    })
}

macro_rules! avg_field {
    ($kpis:expr, $($f:ident),+ $(,)?) => {{
        let n = $kpis.len() as f64;
        KpiSummary {
            $($f: $kpis.iter().map(|k| k.$f).sum::<f64>() / n,)+
        }
    }};
}

fn mean_kpis(kpis: &[KpiSummary]) -> KpiSummary {
    avg_field!(
        kpis,
        orders_total, px1_served, px1_on_time, px2_downgraded, lost_sales, oversell_events,
        sla_attainment, p50_click_to_door_min, p95_click_to_door_min, avg_pick_wait_min,
        delay_prob, avg_courier_match_min, avg_travel_min, avg_distance_km, fill_rate,
        revenue_margin_idr, courier_cost_idr, labor_cost_idr, rent_cost_idr, midmile_cost_idr,
        store_fees_idr, contribution_idr, contribution_per_order, promise_shown_rate,
        picker_utilization, midmile_km_per_day, midmile_trucks_per_day, px2_rider_km_per_day,
        px2_drops_per_batch, px2_batch_savings_pct,
    )
}
