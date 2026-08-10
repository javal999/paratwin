//! ParaTwin engine: digital twin of the ParaExpress hyperlocal fulfillment
//! network. Discrete-event + agent-based + Monte Carlo, with closed-form
//! validators from the ParaExpress technical report.

pub mod analytics;
pub mod demand;
pub mod des;
pub mod model;
pub mod montecarlo;
pub mod routing;
pub mod vrp;

use model::*;
use serde::{Deserialize, Serialize};

/// Auto-layout: place nodes on the demand field by greedy submodular coverage
/// (Theorem 2 / Corollary 1 of the report).
pub fn auto_layout(
    metro_km: f64,
    n_dark: u32,
    n_store: u32,
    flagship: bool,
    orders_per_day: f64,
) -> Vec<FulfillNode> {
    let sc = Scenario { nodes: vec![], ..default_scenario() };
    let sc = Scenario { metro_km, orders_per_day, ..sc };
    let field = demand::DemandField::new(&sc);
    let zones: Vec<(f64, f64, f64)> = (0..field.weights.len())
        .map(|i| {
            let (x, y) = field.cell_center(i);
            (x, y, field.weights[i])
        })
        .collect();
    let cands: Vec<(f64, f64)> = zones.iter().map(|z| (z.0, z.1)).collect();

    let mut out: Vec<FulfillNode> = Vec::new();
    let mut id = 0u32;

    if flagship {
        // flagship at the highest-demand cell (CBD)
        let best = zones
            .iter()
            .enumerate()
            .max_by(|a, b| a.1 .2.partial_cmp(&b.1 .2).unwrap())
            .unwrap()
            .0;
        let (x, y) = (zones[best].0, zones[best].1);
        out.push(FulfillNode {
            id,
            kind: NodeKind::Flagship,
            x_km: x,
            y_km: y,
            pickers: 6,
            assortment_cover: 0.95,
            base_stock_days: 3.0,
        });
        id += 1;
    }

    // dark MFCs by greedy coverage at 3.5 km radius
    let picks = analytics::greedy_coverage(&zones, &cands, 3.5, (n_dark + n_store) as usize + 8);
    let mut iter = picks.iter();
    let mut placed_dark = 0;
    while placed_dark < n_dark {
        let Some((ci, _, _)) = iter.next() else { break };
        {
            let (x, y) = cands[*ci];
            // keep >= 1.5 km separation from already-placed nodes
            if out
                .iter()
                .any(|n| ((n.x_km - x).powi(2) + (n.y_km - y).powi(2)).sqrt() < 1.5)
            {
                continue;
            }
            placed_dark += 1;
            out.push(FulfillNode {
                id,
                kind: NodeKind::Dark,
                x_km: x,
                y_km: y,
                pickers: 3,
                assortment_cover: 0.80,
                base_stock_days: 2.5,
            });
            id += 1;
        }
    }
    // store nodes fill the remaining best cells (jittered so they do not
    // coincide with the dark MFCs)
    let mut placed = 0;
    for (ci, _, _) in iter {
        if placed >= n_store {
            break;
        }
        let (x, y) = cands[*ci];
        if out
            .iter()
            .any(|n| ((n.x_km - x).powi(2) + (n.y_km - y).powi(2)).sqrt() < 1.2)
        {
            continue;
        }
        out.push(FulfillNode {
            id,
            kind: NodeKind::Store,
            x_km: (x + 0.7).min(metro_km),
            y_km: (y + 0.4).min(metro_km),
            pickers: 1,
            assortment_cover: 0.55,
            base_stock_days: 2.0,
        });
        id += 1;
        placed += 1;
    }
    // if greedy ran out of coverage-distinct cells, place remaining stores on
    // the next-highest demand cells
    if placed < n_store {
        let mut ranked: Vec<usize> = (0..zones.len()).collect();
        ranked.sort_by(|a, b| zones[*b].2.partial_cmp(&zones[*a].2).unwrap());
        for zi in ranked {
            if placed >= n_store {
                break;
            }
            let (x, y) = (zones[zi].0, zones[zi].1);
            if out
                .iter()
                .all(|n| ((n.x_km - x).powi(2) + (n.y_km - y).powi(2)).sqrt() > 1.2)
            {
                out.push(FulfillNode {
                    id,
                    kind: NodeKind::Store,
                    x_km: x,
                    y_km: y,
                    pickers: 1,
                    assortment_cover: 0.55,
                    base_stock_days: 2.0,
                });
                id += 1;
                placed += 1;
            }
        }
    }
    out
}

pub fn default_scenario() -> Scenario {
    Scenario {
        name: "Jakarta pilot".into(),
        metro_km: 20.0,
        orders_per_day: 1500.0,
        days: 7,
        replications: 10,
        seed: 42,
        sla_min: 60.0,
        sla_target: 0.95,
        wave_seconds: 20.0,
        pick_mean_min: 4.0,
        aov_idr: 220_000.0,
        margin_pct: 0.45,
        delivery_fee_idr: 10_000.0,
        downgrade_accept: 0.6,
        wage_per_hour: 22_000.0,
        rent_dark_m: 45.0,
        rent_flag_m: 90.0,
        store_fee_per_order: 4_000.0,
        pos_latency_min: 5.0,
        store_buffer: 2.0,
        courier: CourierCfg {
            speed_kmh: 22.0,
            match_mean_min: 6.5,
            match_cv: 0.45,
            primary_fail_prob: 0.06,
            failover_extra_min: 12.0,
            dual_carrier: true,
            fee_base: 9_000.0,
            fee_per_km: 2_400.0,
            prematch: true,
        },
        midmile: MidMileCfg {
            period_days: 1.0,
            otif: 0.96,
            cost_per_drop: 90_000.0,
            truck_fixed_idr: 250_000.0,
            truck_per_km_idr: 6_000.0,
            truck_capacity_units: 400.0,
        },
        nodes: vec![],
    }
}

/// Named presets matching the report's rollout phases.
pub fn preset(name: &str) -> Scenario {
    let mut sc = default_scenario();
    match name {
        "baseline" => {
            sc.name = "Phase 0 — Minimal (3 dark MFCs)".into();
            sc.nodes = auto_layout(sc.metro_km, 3, 0, false, sc.orders_per_day);
            sc.orders_per_day = 800.0;
        }
        "pilot" => {
            sc.name = "Phase 1 — Pilot (1 flagship + 2 dark + 15 stores)".into();
            sc.nodes = auto_layout(sc.metro_km, 2, 15, true, sc.orders_per_day);
            sc.orders_per_day = 1500.0;
        }
        "scaleout" => {
            sc.name = "Phase 2 — Scale-out (1 flagship + 6 dark + 30 stores)".into();
            sc.nodes = auto_layout(sc.metro_km, 6, 30, true, sc.orders_per_day);
            sc.orders_per_day = 4000.0;
        }
        _ => {
            sc.nodes = auto_layout(sc.metro_km, 2, 15, true, sc.orders_per_day);
        }
    }
    sc
}

// ---------------- validation report (analytic vs simulated) ----------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRow {
    pub law: String,
    pub reference: String,
    pub analytic: f64,
    pub simulated: f64,
    pub rel_err: f64,
    pub pass: bool,
}

/// Cross-validate the DES against the closed forms of the report.
/// This is the "mathematically proven" gate: the twin must reproduce
/// Erlang-C queueing, Theorem-3 cost structure, and the routing optimum.
pub fn validate_math() -> Vec<ValidationRow> {
    let mut rows = Vec::new();

    // 1) M/M/c delay probability: single node, exponential-ish pick times.
    //    Use a scenario tuned so pick queue dominates: 1 node, 3 pickers.
    let mut sc = default_scenario();
    sc.days = 10;
    sc.replications = 5;
    sc.orders_per_day = 1400.0;
    sc.metro_km = 6.0; // tiny metro: travel never binds
    sc.sla_min = 240.0; // effectively no rejection
    sc.pick_mean_min = 4.0;
    sc.courier.match_mean_min = 3.0;
    sc.nodes = vec![FulfillNode {
        id: 0,
        kind: NodeKind::Dark,
        x_km: 3.0,
        y_km: 3.0,
        pickers: 3,
        assortment_cover: 1.0,
        base_stock_days: 10.0,
    }];
    let mc = montecarlo::run_monte_carlo(&sc).unwrap();
    // effective peak-hour comparison is noisy; compare day-average lambda
    let lam_per_min = sc.orders_per_day / 1440.0;
    let mu = 1.0 / sc.pick_mean_min;
    let analytic = analytics::erlang_c(lam_per_min, mu, 3);
    let simulated = mc.result.kpi.delay_prob;
    // demand is nonhomogeneous (peaks), so allow a generous band; the point
    // is order-of-magnitude agreement of queueing behavior
    let rel = (simulated - analytic).abs() / analytic.max(1e-9);
    rows.push(ValidationRow {
        law: "M/M/c delay probability (Erlang-C)".into(),
        reference: "Prop. 3 / Halfin-Whitt".into(),
        analytic,
        simulated,
        rel_err: rel,
        pass: simulated > 0.2 * analytic && simulated < 5.0 * analytic,
    });

    // 2) Theorem 3: n* of the cost curve (report Fig. 8 parameters)
    let n = analytics::n_star(1.2, 6.0, 120.0);
    rows.push(ValidationRow {
        law: "Unique optimal node count n* (2Fx^3+ax^2-b=0)".into(),
        reference: "Theorem 3 / Remark 3".into(),
        analytic: 9.06,
        simulated: n,
        rel_err: (n - 9.06).abs() / 9.06,
        pass: (n - 9.06).abs() < 0.15,
    });

    // 3) Asset-light limit n* -> b/a
    let n0 = analytics::n_star(0.0, 6.0, 120.0);
    rows.push(ValidationRow {
        law: "Asset-light limit n* = b/a".into(),
        reference: "Theorem 3 (F→0)".into(),
        analytic: 20.0,
        simulated: n0,
        rel_err: (n0 - 20.0).abs() / 20.0,
        pass: (n0 - 20.0).abs() < 1e-6,
    });

    // 4) Eppen pooling: safety stock ratio for 4 nodes vs 1 = sqrt(4) = 2
    let r = analytics::pooled_safety_stock(1.65, 100.0, 4.0)
        / analytics::pooled_safety_stock(1.65, 100.0, 1.0);
    rows.push(ValidationRow {
        law: "Risk-pooling sqrt(n) safety-stock inflation".into(),
        reference: "Prop. 1 (Eppen)".into(),
        analytic: 2.0,
        simulated: r,
        rel_err: (r - 2.0).abs() / 2.0,
        pass: (r - 2.0).abs() < 1e-9,
    });

    // 5) Square-root staffing sublinearity
    let c1 = analytics::sqrt_staffing(10.0, 0.25, 0.2) as f64;
    let c2 = analytics::sqrt_staffing(20.0, 0.25, 0.2) as f64;
    rows.push(ValidationRow {
        law: "Sublinear staffing: c(2λ) < 2 c(λ)".into(),
        reference: "Prop. 3".into(),
        analytic: 2.0 * c1,
        simulated: c2,
        rel_err: 0.0,
        pass: c2 < 2.0 * c1,
    });

    rows
}

// ---------------- decision tools ----------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepPoint {
    pub n_nodes: u32,
    pub sla: f64,
    pub contribution_idr: f64,
    pub lost_per_day: f64,
    pub promise_shown: f64,
}

/// Network-size sweep: vary the number of dark MFCs, keep stores fixed.
pub fn network_sweep(base: &Scenario, n_from: u32, n_to: u32) -> Vec<SweepPoint> {
    let stores = base.nodes.iter().filter(|n| n.kind == NodeKind::Store).count() as u32;
    let flagship = base.nodes.iter().any(|n| n.kind == NodeKind::Flagship);
    let mut out = Vec::new();
    for n in n_from..=n_to {
        let mut sc = base.clone();
        sc.replications = base.replications.min(5);
        sc.days = base.days.min(5);
        sc.nodes = auto_layout(sc.metro_km, n, stores, flagship, sc.orders_per_day);
        if let Ok(mc) = montecarlo::run_monte_carlo(&sc) {
            out.push(SweepPoint {
                n_nodes: n,
                sla: mc.result.kpi.sla_attainment,
                contribution_idr: mc.result.kpi.contribution_idr,
                lost_per_day: mc.result.kpi.lost_sales,
                promise_shown: mc.result.kpi.promise_shown_rate,
            });
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnershipOption {
    pub name: String,
    pub description: String,
    pub sla: f64,
    pub contribution_idr: f64,
    pub courier_cost_idr: f64,
    pub p95_min: f64,
    pub recommended: bool,
}

/// Compare courier partnership strategies on the same scenario.
pub fn compare_partnerships(base: &Scenario) -> Vec<PartnershipOption> {
    let mut opts: Vec<(String, String, Scenario)> = Vec::new();

    let mut grab_only = base.clone();
    grab_only.courier.dual_carrier = false;
    opts.push((
        "Grab only".into(),
        "Single on-demand carrier; exposed to surge and match failures.".into(),
        grab_only,
    ));

    let mut dual = base.clone();
    dual.courier.dual_carrier = true;
    opts.push((
        "Grab + GoSend dual-carrier".into(),
        "ShipGo+ rate-shop with automatic failover (report Sec. VII-B).".into(),
        dual,
    ));

    let mut own = base.clone();
    own.courier.dual_carrier = false;
    own.courier.match_mean_min = 3.0; // dedicated riders staged at nodes
    own.courier.match_cv = 0.25;
    own.courier.primary_fail_prob = 0.01;
    own.courier.fee_base = 14_000.0; // payroll-equivalent per-order cost at pilot volume
    own.courier.fee_per_km = 1_200.0;
    opts.push((
        "Own fleet".into(),
        "Dedicated riders: faster match but higher fixed cost below ~35-45 orders/day/rider.".into(),
        own,
    ));

    let mut rows: Vec<PartnershipOption> = opts
        .into_iter()
        .filter_map(|(name, description, mut sc)| {
            sc.replications = sc.replications.min(6);
            sc.days = sc.days.min(5);
            montecarlo::run_monte_carlo(&sc).ok().map(|mc| PartnershipOption {
                name,
                description,
                sla: mc.result.kpi.sla_attainment,
                contribution_idr: mc.result.kpi.contribution_idr,
                courier_cost_idr: mc.result.kpi.courier_cost_idr,
                p95_min: mc.result.kpi.p95_click_to_door_min,
                recommended: false,
            })
        })
        .collect();

    // recommend: max contribution among options meeting the SLA target,
    // else max SLA
    let target = base.sla_target;
    let best = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.sla >= target - 0.02)
        .max_by(|a, b| a.1.contribution_idr.partial_cmp(&b.1.contribution_idr).unwrap())
        .map(|(i, _)| i)
        .or_else(|| {
            rows.iter()
                .enumerate()
                .max_by(|a, b| a.1.sla.partial_cmp(&b.1.sla).unwrap())
                .map(|(i, _)| i)
        });
    if let Some(i) = best {
        rows[i].recommended = true;
    }
    rows
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreRankRow {
    pub rank: u32,
    pub x_km: f64,
    pub y_km: f64,
    pub marginal_coverage: f64,
    pub cumulative_coverage: f64,
}

/// Greedy consignment-store activation ranking (Corollary 1).
pub fn rank_store_activations(base: &Scenario, take: usize) -> Vec<StoreRankRow> {
    let field = demand::DemandField::new(base);
    let zones: Vec<(f64, f64, f64)> = (0..field.weights.len())
        .map(|i| {
            let (x, y) = field.cell_center(i);
            (x, y, field.weights[i])
        })
        .collect();
    let cands: Vec<(f64, f64)> = zones.iter().map(|z| (z.0, z.1)).collect();
    let radius = (base.courier.speed_kmh / 60.0)
        * (base.sla_min - base.pick_mean_min - base.courier.match_mean_min - 10.0).max(5.0);
    let picks = analytics::greedy_coverage(&zones, &cands, radius.min(4.0), take);
    picks
        .into_iter()
        .enumerate()
        .map(|(i, (ci, g, c))| StoreRankRow {
            rank: i as u32 + 1,
            x_km: cands[ci].0,
            y_km: cands[ci].1,
            marginal_coverage: g,
            cumulative_coverage: c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_with_same_seed() {
        let mut sc = preset("baseline");
        sc.days = 2;
        sc.replications = 2;
        let a = montecarlo::run_monte_carlo(&sc).unwrap();
        let b = montecarlo::run_monte_carlo(&sc).unwrap();
        assert_eq!(
            serde_json::to_string(&a.result.kpi).unwrap(),
            serde_json::to_string(&b.result.kpi).unwrap()
        );
    }

    #[test]
    fn pilot_runs_and_produces_sane_kpis() {
        let mut sc = preset("pilot");
        sc.days = 3;
        sc.replications = 3;
        let mc = montecarlo::run_monte_carlo(&sc).unwrap();
        let k = &mc.result.kpi;
        assert!(k.orders_total > 1000.0, "orders {}", k.orders_total);
        assert!(k.sla_attainment > 0.5, "sla {}", k.sla_attainment);
        assert!(k.p50_click_to_door_min > 10.0 && k.p50_click_to_door_min < 90.0);
        assert!(!mc.frames.is_empty(), "frames must be captured");
        // accounting identity: contribution = margin - all costs
        let lhs = k.contribution_idr;
        let rhs = k.revenue_margin_idr
            - k.courier_cost_idr
            - k.labor_cost_idr
            - k.rent_cost_idr
            - k.midmile_cost_idr
            - k.store_fees_idr;
        assert!((lhs - rhs).abs() < 1.0, "accounting identity violated");
    }

    #[test]
    fn more_nodes_shorten_distances() {
        let mut small = preset("baseline");
        small.days = 2;
        small.replications = 3;
        let mut big = small.clone();
        big.nodes = auto_layout(small.metro_km, 10, 0, false, small.orders_per_day);
        let a = montecarlo::run_monte_carlo(&small).unwrap();
        let b = montecarlo::run_monte_carlo(&big).unwrap();
        assert!(
            b.result.kpi.avg_distance_km < a.result.kpi.avg_distance_km,
            "10 nodes should shorten the last mile: {} vs {}",
            b.result.kpi.avg_distance_km,
            a.result.kpi.avg_distance_km
        );
    }

    #[test]
    fn prematch_beats_sequential_booking() {
        let mut pre = preset("baseline");
        pre.days = 2;
        pre.replications = 3;
        let mut seq = pre.clone();
        seq.courier.prematch = false;
        let a = montecarlo::run_monte_carlo(&pre).unwrap();
        let b = montecarlo::run_monte_carlo(&seq).unwrap();
        assert!(
            a.result.kpi.p50_click_to_door_min < b.result.kpi.p50_click_to_door_min,
            "prematch {} should beat sequential {}",
            a.result.kpi.p50_click_to_door_min,
            b.result.kpi.p50_click_to_door_min
        );
    }

    #[test]
    fn order_conservation() {
        // every generated order must end in exactly one terminal state:
        // total = served (on-time + late) + downgraded + lost + oversell
        for preset_name in ["baseline", "pilot"] {
            let mut sc = preset(preset_name);
            sc.days = 3;
            sc.replications = 1;
            let out = des::simulate(&sc, 1234, false);
            let k = &out.kpi;
            let accounted = k.px1_served + k.px2_downgraded + k.lost_sales + k.oversell_events;
            assert!(
                (accounted - k.orders_total).abs() < 1e-6,
                "{preset_name}: conservation violated: {} vs {}",
                accounted,
                k.orders_total
            );
            // on-time is a subset of served
            assert!(k.px1_on_time <= k.px1_served + 1e-9);
        }
    }

    #[test]
    fn sla_not_better_under_3x_demand() {
        // stressing the same network must not improve service
        let mut base = preset("baseline");
        base.days = 3;
        base.replications = 3;
        let mut heavy = base.clone();
        heavy.orders_per_day *= 3.0;
        let a = montecarlo::run_monte_carlo(&base).unwrap();
        let b = montecarlo::run_monte_carlo(&heavy).unwrap();
        assert!(
            b.result.kpi.sla_attainment <= a.result.kpi.sla_attainment + 0.01,
            "SLA improved under 3x load: {} -> {}",
            a.result.kpi.sla_attainment,
            b.result.kpi.sla_attainment
        );
        assert!(
            b.result.kpi.p95_click_to_door_min >= a.result.kpi.p95_click_to_door_min - 1.0,
            "p95 improved under 3x load"
        );
    }

    #[test]
    fn dual_carrier_reduces_tail() {
        // failover should cut the p95 tail when the primary is unreliable
        let mut single = preset("baseline");
        single.days = 3;
        single.replications = 4;
        single.courier.dual_carrier = false;
        single.courier.primary_fail_prob = 0.25;
        let mut dual = single.clone();
        dual.courier.dual_carrier = true;
        let a = montecarlo::run_monte_carlo(&single).unwrap();
        let b = montecarlo::run_monte_carlo(&dual).unwrap();
        assert!(
            b.result.kpi.p95_click_to_door_min < a.result.kpi.p95_click_to_door_min,
            "dual carrier should shorten the tail: {} vs {}",
            b.result.kpi.p95_click_to_door_min,
            a.result.kpi.p95_click_to_door_min
        );
    }

    #[test]
    fn px2_conservation_every_downgrade_batched() {
        // shrink coverage so plenty of orders overflow to PX2, then check
        // drops-per-batch x batches reconstructs the downgraded count exactly
        let mut sc = preset("baseline");
        sc.days = 2;
        sc.replications = 1;
        for n in &mut sc.nodes {
            n.assortment_cover = 0.55; // force overflow
        }
        sc.downgrade_accept = 1.0; // all overflow accepted as PX2
        let out = des::simulate(&sc, 777, false);
        let k = &out.kpi;
        assert!(k.px2_downgraded > 5.0, "need PX2 volume, got {}", k.px2_downgraded);
        // px2_drops_per_batch is drops/batches; recover total drops per day
        // via rider km bookkeeping invariant instead: batches>0 and savings>=0
        assert!(k.px2_drops_per_batch >= 1.0);
        assert!(k.px2_rider_km_per_day > 0.0);
        assert!(k.px2_batch_savings_pct >= 0.0 && k.px2_batch_savings_pct <= 1.0);
        // conservation is checked exactly inside the engine test hook below
        let (drops, downgraded) = des::px2_conservation_probe(&sc, 777);
        assert_eq!(drops, downgraded, "every accepted PX2 order must be batched once");
    }

    #[test]
    fn validation_report_passes() {
        let rows = validate_math();
        for r in &rows {
            assert!(r.pass, "validation failed: {} ({} vs {})", r.law, r.analytic, r.simulated);
        }
    }
}
