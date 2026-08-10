//! Scenario and result data model for the ParaTwin digital twin.
//! Mirrors the formal model of the ParaExpress technical report:
//! demand field (Def. 2), promise classes (Def. 3), fulfillment graph (Def. 1).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Store,    // consignment store node (Exagon), ~0 fixed cost
    Dark,     // dark MFC
    Flagship, // flagship MFC
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillNode {
    pub id: u32,
    pub kind: NodeKind,
    pub x_km: f64,
    pub y_km: f64,
    pub pickers: u32,
    /// share of the instant-basket demand the node's forward assortment covers
    /// (Theorem 4: top-K by demand rate; coverage is concave in slots)
    pub assortment_cover: f64,
    /// base stock in units of "basket-equivalents" per day of local demand
    pub base_stock_days: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourierCfg {
    pub speed_kmh: f64,
    /// mean minutes from booking to rider at door (lognormal)
    pub match_mean_min: f64,
    pub match_cv: f64,
    /// probability primary carrier fails/surges per order; failover adds delay
    pub primary_fail_prob: f64,
    pub failover_extra_min: f64,
    pub dual_carrier: bool,
    /// pricing (IDR)
    pub fee_base: f64,
    pub fee_per_km: f64,
    /// courier booked at pick start (true = ParaExpress pre-matching)
    pub prematch: bool,
}

fn d_truck_fixed() -> f64 { 250_000.0 }
fn d_truck_per_km() -> f64 { 6_000.0 }
fn d_truck_cap() -> f64 { 400.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidMileCfg {
    /// replenishment period in days (MOSTRANS daily wave = 1.0)
    pub period_days: f64,
    /// on-time-in-full probability of the wave
    pub otif: f64,
    /// legacy flat cost per node-drop (kept for old scenarios; routing
    /// economics below supersede it)
    pub cost_per_drop: f64,
    /// CVRP truck economics: fixed cost per truck-wave and cost per routed km
    #[serde(default = "d_truck_fixed")]
    pub truck_fixed_idr: f64,
    #[serde(default = "d_truck_per_km")]
    pub truck_per_km_idr: f64,
    /// truck capacity in basket-equivalent units per wave
    #[serde(default = "d_truck_cap")]
    pub truck_capacity_units: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    /// metro square side (km); demand is a Gaussian-mixture field on it
    pub metro_km: f64,
    /// total instant-channel orders per day (Λ)
    pub orders_per_day: f64,
    /// simulated horizon
    pub days: u32,
    /// Monte Carlo replications
    pub replications: u32,
    pub seed: u64,
    /// promise τ minutes and target α
    pub sla_min: f64,
    pub sla_target: f64,
    /// routing wave (seconds)
    pub wave_seconds: f64,
    /// mean pick-pack minutes per order (lognormal, cv 0.35)
    pub pick_mean_min: f64,
    /// average order value and gross margin
    pub aov_idr: f64,
    pub margin_pct: f64,
    /// delivery fee charged to the consumer (IDR)
    pub delivery_fee_idr: f64,
    /// share of PX1-infeasible orders that accept PX2 (rest lost)
    pub downgrade_accept: f64,
    /// picker wage per hour (IDR)
    pub wage_per_hour: f64,
    /// monthly rent by node kind (IDR M) -> converted internally per day
    pub rent_dark_m: f64,
    pub rent_flag_m: f64,
    /// store-node economics: per-order fee to partner (IDR)
    pub store_fee_per_order: f64,
    /// POS posting latency at store nodes (minutes) -> oversell risk (Prop. 4)
    pub pos_latency_min: f64,
    /// protection buffer at store nodes (basket-equivalents)
    pub store_buffer: f64,
    pub courier: CourierCfg,
    pub midmile: MidMileCfg,
    pub nodes: Vec<FulfillNode>,
}

impl Scenario {
    pub fn validate(&self) -> Result<(), String> {
        if self.metro_km <= 0.0 || self.orders_per_day <= 0.0 {
            return Err("metro size and demand must be positive".into());
        }
        if self.days == 0 || self.replications == 0 {
            return Err("days and replications must be >= 1".into());
        }
        if self.nodes.is_empty() {
            return Err("at least one fulfillment node is required".into());
        }
        if !(0.5..=0.999).contains(&self.sla_target) {
            return Err("sla_target must be in [0.5, 0.999]".into());
        }
        Ok(())
    }
}

/// PTW-02 / C-01: distribution of distinct SKUs per basket.
///
/// The PX1 promise is measured on the order, and an order fails if any SKU in
/// its basket is missing. `p[j]` is the probability a basket holds exactly `j`
/// distinct SKUs, so `p[0]` is unused and the generating function is
/// G(x) = sum_j p_j x^j.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketSkuDist {
    /// p[j] = P(basket has exactly j distinct SKUs); index 0 carries no mass
    pub p: Vec<f64>,
}

impl BasketSkuDist {
    /// Every basket holds exactly `k` distinct SKUs (the C-01 Check case).
    pub fn exactly(k: usize) -> Self {
        let mut p = vec![0.0; k + 1];
        p[k] = 1.0;
        BasketSkuDist { p }
    }

    /// G(x) = sum_j p_j x^j -- the order fill rate at item fill rate x.
    pub fn generating_fn(&self, x: f64) -> f64 {
        self.p
            .iter()
            .enumerate()
            .map(|(j, pj)| pj * x.powi(j as i32))
            .sum()
    }

    /// beta* = G-inverse(alpha): the item fill rate that delivers order fill
    /// rate `alpha`. Reading Phi-inverse(alpha) directly is the C-01 defect.
    ///
    /// G is continuous and non-decreasing on [0,1] with G(0) = 0 and G(1) = 1
    /// whenever every basket holds at least one SKU, so bisection converges.
    pub fn inverse_g(&self, alpha: f64) -> f64 {
        if alpha <= 0.0 {
            return 0.0;
        }
        if alpha >= 1.0 {
            return 1.0;
        }
        let (mut lo, mut hi) = (0.0f64, 1.0f64);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            // `<` survives mutation to `<=`: the two differ only when
            // G(mid) == alpha exactly, and both keep the root bracketed and
            // converge to it. Equivalent mutant, not a coverage gap.
            if self.generating_fn(mid) < alpha {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    /// Mean distinct SKUs per basket.
    pub fn mean(&self) -> f64 {
        self.p.iter().enumerate().map(|(j, pj)| j as f64 * pj).sum()
    }
}

/// Hourly demand profile for Indonesian e-commerce (share of daily demand
/// per hour; two peaks: lunchtime and the 19:00-21:00 live-commerce prime).
pub const HOURLY: [f64; 24] = [
    0.010, 0.006, 0.004, 0.003, 0.004, 0.008, 0.018, 0.030, 0.042, 0.050, 0.058, 0.066, 0.070,
    0.060, 0.052, 0.048, 0.050, 0.055, 0.062, 0.075, 0.082, 0.070, 0.048, 0.029,
];

/// Traffic slowdown factor by hour (peak-hour congestion in Jakarta).
pub fn traffic_factor(hour: usize) -> f64 {
    match hour {
        7..=9 => 1.45,
        16..=19 => 1.55,
        10..=15 => 1.20,
        20..=21 => 1.15,
        _ => 1.0,
    }
}

// ---------------- results ----------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KpiSummary {
    pub orders_total: f64,
    pub px1_served: f64,
    pub px1_on_time: f64,
    pub px2_downgraded: f64,
    pub lost_sales: f64,
    pub oversell_events: f64,
    /// SLA attainment α̂ = on-time / served (PX1)
    pub sla_attainment: f64,
    pub p50_click_to_door_min: f64,
    pub p95_click_to_door_min: f64,
    pub avg_pick_wait_min: f64,
    pub delay_prob: f64, // P(queue wait > 0), Halfin-Whitt check
    pub avg_courier_match_min: f64,
    pub avg_travel_min: f64,
    pub avg_distance_km: f64,
    pub fill_rate: f64,
    /// economics per day (IDR)
    pub revenue_margin_idr: f64,
    pub courier_cost_idr: f64,
    pub labor_cost_idr: f64,
    pub rent_cost_idr: f64,
    pub midmile_cost_idr: f64,
    pub store_fees_idr: f64,
    pub contribution_idr: f64,
    pub contribution_per_order: f64,
    /// promise-shown rate: share of zone-hours with PX1 feasible
    pub promise_shown_rate: f64,
    pub picker_utilization: f64,
    /// routed mid-mile km per day (CVRP: savings + 2-opt + Or-opt)
    #[serde(default)]
    pub midmile_km_per_day: f64,
    #[serde(default)]
    pub midmile_trucks_per_day: f64,
    /// PX2 same-day rider batching (multi-pickup multi-delivery)
    #[serde(default)]
    pub px2_rider_km_per_day: f64,
    #[serde(default)]
    pub px2_drops_per_batch: f64,
    /// cost saved by batching vs one solo trip per PX2 order
    #[serde(default)]
    pub px2_batch_savings_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourPoint {
    pub hour: u32,
    pub orders: f64,
    pub on_time_rate: f64,
    pub avg_ctd_min: f64,
    pub queue_len: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStat {
    pub id: u32,
    pub kind: NodeKind,
    pub x_km: f64,
    pub y_km: f64,
    pub orders: f64,
    pub utilization: f64,
    pub on_time_rate: f64,
    pub stockout_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneStat {
    pub zx: u32,
    pub zy: u32,
    pub demand: f64,
    pub on_time_rate: f64,
    pub covered: bool,
}

/// One animation frame (per simulated minute of replication 0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub minute: u32,
    /// couriers: [x, y, state] where state 0=to-pickup 1=to-drop
    pub couriers: Vec<(f32, f32, u8)>,
    /// per node: queue length (waiting + in service)
    pub queues: Vec<u16>,
    /// deliveries completed this minute: consumer (x, y, on_time)
    pub drops: Vec<(f32, f32, u8)>,
    pub orders_so_far: u32,
    pub on_time_so_far: u32,
    // ---- cumulative stats for live dashboard metrics ----
    #[serde(default)]
    pub arrived_so_far: u32,
    #[serde(default)]
    pub lost_so_far: u32,
    #[serde(default)]
    pub px2_so_far: u32,
    #[serde(default)]
    pub waited_so_far: u32,
    #[serde(default)]
    pub ctd_sum_min: f32,
    /// cumulative contribution so far (margin - variable - prorated fixed)
    #[serde(default)]
    pub contrib_idr: f64,
    /// latest promise-shown sample (share of demand able to see Instant)
    #[serde(default)]
    pub promise_now: f32,
    #[serde(default)]
    pub busy_pickers: u16,
    #[serde(default)]
    pub total_pickers: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiStat {
    pub mean: f64,
    pub half_width: f64, // 95% CI half width across replications
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub kpi: KpiSummary,
    /// 95% confidence intervals for headline KPIs across replications
    pub ci: std::collections::BTreeMap<String, CiStat>,
    pub hourly: Vec<HourPoint>,
    pub nodes: Vec<NodeStat>,
    pub zones: Vec<ZoneStat>,
    pub replication_kpis: Vec<KpiSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// PTW-02 / C-01. Expected values are the correction's Check line, not
    /// anything read back out of this implementation.
    #[test]
    fn basket_generating_function_matches_check() {
        let d = BasketSkuDist::exactly(3);
        // C-01 Check: with beta = 0.98 the order fill rate is 0.941192
        assert!((d.generating_fn(0.98) - 0.941192).abs() < 1e-6, "G(0.98)");
        // and beta* = G-inverse(0.95) = 0.983048
        assert!((d.inverse_g(0.95) - 0.983048).abs() < 1e-6, "beta*");
        assert!((d.mean() - 3.0).abs() < 1e-12);
        // a one-SKU basket is the degenerate case where order == item fill
        let single = BasketSkuDist::exactly(1);
        assert!((single.generating_fn(0.93) - 0.93).abs() < 1e-12);
        assert!((single.inverse_g(0.93) - 0.93).abs() < 1e-9);
    }

    proptest! {
        /// G and its inverse must round-trip, and multi-SKU baskets must
        /// always demand a *higher* item fill rate than the order target --
        /// that gap is the whole point of C-01.
        #[test]
        fn inverse_g_round_trips_and_exceeds_target(
            k in 1usize..8, alpha in 0.50f64..0.999
        ) {
            let d = BasketSkuDist::exactly(k);
            let beta = d.inverse_g(alpha);
            prop_assert!((d.generating_fn(beta) - alpha).abs() < 1e-9, "round trip");
            prop_assert!(beta >= alpha - 1e-12, "item fill must not be below order fill");
            if k > 1 {
                prop_assert!(beta > alpha, "multi-SKU baskets need a strictly higher item fill");
            }
        }

        /// G is a probability generating function: increasing on [0,1] with
        /// G(0) = 0 for baskets that always hold at least one SKU, G(1) = 1.
        #[test]
        fn generating_fn_is_a_cdf_like_map(k in 1usize..8, x in 0.0f64..1.0) {
            let d = BasketSkuDist::exactly(k);
            prop_assert!((d.generating_fn(1.0) - 1.0).abs() < 1e-12);
            prop_assert!(d.generating_fn(0.0).abs() < 1e-12);
            prop_assert!(d.generating_fn(x) <= d.generating_fn((x + 1e-4).min(1.0)) + 1e-12);
            prop_assert!(d.generating_fn(x) <= x + 1e-12, "G(x) <= x for k >= 1");
        }
    }
}
