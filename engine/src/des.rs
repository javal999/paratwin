//! Discrete-event simulation kernel with agent-based couriers.
//!
//! Implements the operational layer of the ParaExpress report:
//! order arrivals (spatial Poisson, Def. 2), wave routing (P3, Theorem 5),
//! M/M/c-style pick queues (Prop. 3), courier pre-matching, base-stock
//! replenishment (eq. 7), and the store-node ATP/oversell mechanics (Prop. 4).

use crate::demand::DemandField;
use crate::model::*;
use crate::routing::assign_min_cost;
use crate::vrp;
use rand::prelude::*;
use rand_distr::{Distribution, LogNormal, Poisson};
use rand_pcg::Pcg64;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

const HANDOVER_MIN: f64 = 0.5;
const SAFETY_MIN: f64 = 3.0;
const MAX_PENDING_AGE: f64 = 10.0;
const QUEUE_CAP_FACTOR: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Outcome {
    Pending,
    ServedOnTime,
    ServedLate,
    Downgraded,
    Lost,
    Oversell,
}

#[derive(Debug, Clone)]
struct Order {
    t_click: f64,
    x: f64,
    y: f64,
    zone: usize,
    basket_u: f64,
    node: Option<usize>,
    t_enqueue: f64,
    t_pick_start: f64,
    t_pick_end: f64,
    t_courier: f64,
    t_deliver: f64,
    dist_km: f64,
    courier_booked: bool,
    dispatched: bool,
    outcome: Outcome,
    match_delay: f64,
}

struct NodeState {
    spec: FulfillNode,
    stock: f64,
    ledger: f64,
    base_stock: f64,
    queue: VecDeque<usize>,
    busy: u32,
    served: u64,
    on_time: u64,
    stockouts: u64,
    busy_minutes: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Ev {
    Arrival(u32),
    Wave,
    PickEnd(u32),
    CourierArrive(u32),
    Deliver(u32),
    Replenish,
    Px2Dispatch,
    PosSale(u32),
    LedgerPost(u32),
    MinuteTick,
}

#[derive(Debug, Clone, Copy)]
struct Trip {
    t0: f64,
    t1: f64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    phase: u8,
}

/// Test hook: run one replication and return (px2 drops batched, px2 accepted).
pub fn px2_conservation_probe(sc: &crate::model::Scenario, seed: u64) -> (u64, u64) {
    let out = simulate(sc, seed, false);
    (out.px2_drops_total, out.px2_accepted_total)
}

pub struct SimOutput {
    pub kpi: KpiSummary,
    pub hourly: Vec<HourPoint>,
    pub nodes: Vec<NodeStat>,
    pub zones: Vec<ZoneStat>,
    pub frames: Vec<Frame>,
    /// internal conservation counters (test hooks, not serialized)
    pub px2_drops_total: u64,
    pub px2_accepted_total: u64,
}

fn dist(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    // Manhattan-flavored road metric (grid factor 1.18 on Euclidean)
    1.18 * ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

fn lognormal(mean: f64, cv: f64) -> LogNormal<f64> {
    let sigma2 = (1.0 + cv * cv).ln();
    let mu = mean.max(1e-9).ln() - sigma2 / 2.0;
    LogNormal::new(mu, sigma2.sqrt()).unwrap()
}

pub fn simulate(sc: &Scenario, rep_seed: u64, capture_frames: bool) -> SimOutput {
    let mut rng = Pcg64::seed_from_u64(rep_seed);
    let field = DemandField::new(sc);
    let cdf = field.cdf();
    let horizon = sc.days as f64 * 1440.0;

    // ---- generate arrivals ----
    let mut orders: Vec<Order> = Vec::new();
    for day in 0..sc.days {
        for (hour, share) in HOURLY.iter().enumerate() {
            let lam = sc.orders_per_day * share;
            let n = if lam > 0.0 {
                Poisson::new(lam).map(|p| p.sample(&mut rng) as u64).unwrap_or(0)
            } else {
                0
            };
            for _ in 0..n {
                let t = day as f64 * 1440.0 + hour as f64 * 60.0 + rng.gen::<f64>() * 60.0;
                let u: f64 = rng.gen();
                let zone = cdf.partition_point(|&c| c < u).min(cdf.len() - 1);
                let (cx, cy) = field.cell_center(zone);
                let s = field.metro_km / field.cells as f64;
                let x = cx + (rng.gen::<f64>() - 0.5) * s;
                let y = cy + (rng.gen::<f64>() - 0.5) * s;
                orders.push(Order {
                    t_click: t,
                    x,
                    y,
                    zone,
                    basket_u: rng.gen(),
                    node: None,
                    t_enqueue: 0.0,
                    t_pick_start: 0.0,
                    t_pick_end: 0.0,
                    t_courier: 0.0,
                    t_deliver: 0.0,
                    dist_km: 0.0,
                    courier_booked: false,
                    dispatched: false,
                    outcome: Outcome::Pending,
                    match_delay: 0.0,
                });
            }
        }
    }
    orders.sort_by(|a, b| a.t_click.partial_cmp(&b.t_click).unwrap());

    // ---- node init: demand share by nearest-node rule for base stock ----
    let mut share = vec![0.0f64; sc.nodes.len()];
    for (zi, w) in field.weights.iter().enumerate() {
        let (zx, zy) = field.cell_center(zi);
        let nearest = sc
            .nodes
            .iter()
            .enumerate()
            .min_by(|a, b| {
                dist(a.1.x_km, a.1.y_km, zx, zy)
                    .partial_cmp(&dist(b.1.x_km, b.1.y_km, zx, zy))
                    .unwrap()
            })
            .map(|(i, _)| i)
            .unwrap();
        share[nearest] += w;
    }
    let mut nodes: Vec<NodeState> = sc
        .nodes
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            let base = (share[i] * sc.orders_per_day * spec.base_stock_days).max(4.0);
            NodeState {
                spec: spec.clone(),
                stock: base,
                ledger: base,
                base_stock: base,
                queue: VecDeque::new(),
                busy: 0,
                served: 0,
                on_time: 0,
                stockouts: 0,
                busy_minutes: 0.0,
            }
        })
        .collect();

    // ---- event heap ----
    // key: (time in microseconds, sequence) for deterministic ordering
    let mut heap: BinaryHeap<Reverse<(u64, u64, Ev)>> = BinaryHeap::new();
    let mut seq: u64 = 0;
    macro_rules! push {
        ($t:expr, $e:expr) => {{
            seq += 1;
            heap.push(Reverse((($t * 1e6) as u64, seq, $e)));
        }};
    }
    for (i, o) in orders.iter().enumerate() {
        push!(o.t_click, Ev::Arrival(i as u32));
    }
    push!(sc.wave_seconds / 60.0, Ev::Wave);
    push!(sc.midmile.period_days * 1440.0, Ev::Replenish);
    push!(30.0, Ev::Px2Dispatch);
    push!(1.0, Ev::MinuteTick);
    // POS sales at store nodes (9:00-21:00, ~25 baskets/day)
    let pos_rate_per_min = 25.0 / (12.0 * 60.0);
    for (i, n) in nodes.iter().enumerate() {
        if n.spec.kind == NodeKind::Store {
            let dt = -rng.gen::<f64>().max(1e-12).ln() / pos_rate_per_min;
            push!(540.0 + dt, Ev::PosSale(i as u32));
        }
    }

    let pick_dist = lognormal(sc.pick_mean_min, 0.35);
    let travel_noise = lognormal(1.0, 0.15);

    let mut pending: Vec<u32> = Vec::new();
    let mut trips: Vec<(u32, Trip)> = Vec::new(); // (order, trip)
    let mut frames: Vec<Frame> = Vec::new();
    let mut drops_buf: Vec<(f32, f32, u8)> = Vec::new();
    let mut served_ctr: u32 = 0;
    let mut ontime_ctr: u32 = 0;

    // stats accumulators
    let mut ctds: Vec<f64> = Vec::new();
    let mut pick_waits: Vec<f64> = Vec::new();
    let mut match_delays: Vec<f64> = Vec::new();
    let mut travels: Vec<f64> = Vec::new();
    let mut dists: Vec<f64> = Vec::new();
    let mut waited_pos = 0u64;
    let mut hourly_orders = vec![0.0f64; 24];
    let mut hourly_ontime = vec![0.0f64; 24];
    let mut hourly_served = vec![0.0f64; 24];
    let mut hourly_ctd = vec![0.0f64; 24];
    let mut hourly_q = vec![0.0f64; 24];
    let mut hourly_q_n = vec![0.0f64; 24];
    let mut promise_samples = 0.0f64;
    let mut promise_shown = 0.0f64;
    let mut zone_orders = vec![0.0f64; field.weights.len()];
    let mut zone_ontime = vec![0.0f64; field.weights.len()];
    let mut stock_denied = 0u64;
    // live-frame + VRP accumulators
    let mut arrived_ctr: u32 = 0;
    let mut lost_ctr: u32 = 0;
    let mut px2_ctr: u32 = 0;
    let mut store_served_ctr: u64 = 0;
    let mut ctd_sum: f64 = 0.0;
    let mut courier_gross_cum: f64 = 0.0;
    let mut px2_pool: Vec<(f64, f64, usize)> = Vec::new();
    let mut px2_cost_cum: f64 = 0.0;
    let mut px2_naive_cum: f64 = 0.0;
    let mut px2_km_cum: f64 = 0.0;
    let mut px2_batches: u64 = 0;
    let mut px2_dropcnt: u64 = 0;
    let mut midmile_km_cum: f64 = 0.0;
    let mut midmile_trucks: u64 = 0;
    let mut midmile_cost_cum: f64 = 0.0;
    let mut promise_now_var: f64 = 1.0;
    let total_pickers: u32 = nodes
        .iter()
        .filter(|n| n.spec.kind != NodeKind::Store)
        .map(|n| n.spec.pickers)
        .sum::<u32>()
        + nodes.iter().filter(|n| n.spec.kind == NodeKind::Store).map(|n| n.spec.pickers).sum::<u32>();
    let margin_per_order = sc.aov_idr * sc.margin_pct;
    let labor_per_day: f64 = nodes
        .iter()
        .filter(|n| n.spec.kind != NodeKind::Store)
        .map(|n| n.spec.pickers as f64 * 14.0 * sc.wage_per_hour)
        .sum();
    let rent_per_day: f64 = nodes
        .iter()
        .map(|n| match n.spec.kind {
            NodeKind::Dark => sc.rent_dark_m * 1e6 / 30.0,
            NodeKind::Flagship => sc.rent_flag_m * 1e6 / 30.0,
            NodeKind::Store => 0.0,
        })
        .sum();
    let fixed_per_min = (labor_per_day + rent_per_day) / 1440.0;

    // helper: start picks at a node while pickers are free
    macro_rules! try_start_picks {
        ($ni:expr, $now:expr, $rng:expr, $orders:expr, $nodes:expr) => {{
            let ni = $ni;
            loop {
                if $nodes[ni].busy >= $nodes[ni].spec.pickers || $nodes[ni].queue.is_empty() {
                    break;
                }
                let oi = $nodes[ni].queue.pop_front().unwrap() as usize;
                // ATP truth check at pick start (Prop. 4 oversell mechanics)
                if $nodes[ni].stock < 1.0 {
                    $orders[oi].outcome = Outcome::Oversell;
                    $nodes[ni].stockouts += 1;
                    lost_ctr += 1;
                    continue;
                }
                $nodes[ni].stock -= 1.0;
                $nodes[ni].busy += 1;
                $orders[oi].t_pick_start = $now;
                pick_waits.push($now - $orders[oi].t_enqueue);
                if $now - $orders[oi].t_enqueue > 0.05 {
                    waited_pos += 1;
                }
                let d = pick_dist.sample($rng).clamp(1.0, 30.0);
                $orders[oi].t_pick_end = $now + d;
                push!($now + d, Ev::PickEnd(oi as u32));
                // courier pre-match at pick start
                if sc.courier.prematch && !$orders[oi].courier_booked {
                    let (bt, be) = book_courier(oi, $now, $rng, $orders, $nodes, &mut trips, sc);
                    push!(bt, be);
                }
            }
        }};
    }

    fn book_courier(
        oi: usize,
        now: f64,
        rng: &mut Pcg64,
        orders: &mut [Order],
        nodes: &[NodeState],
        trips: &mut Vec<(u32, Trip)>,
        sc: &Scenario,
    ) -> (f64, Ev) {
        let hour = ((now / 60.0) as usize) % 24;
        let s = if traffic_factor(hour) > 1.4 { 1.35 } else { 1.0 };
        let ln = {
            let mean = sc.courier.match_mean_min * s;
            let cv = sc.courier.match_cv;
            let sigma2 = (1.0 + cv * cv).ln();
            LogNormal::new(mean.ln() - sigma2 / 2.0, sigma2.sqrt()).unwrap()
        };
        let mut delay = ln.sample(rng).clamp(1.0, 60.0);
        let fail_p = if sc.courier.dual_carrier {
            sc.courier.primary_fail_prob * 0.35
        } else {
            sc.courier.primary_fail_prob
        };
        if rng.gen::<f64>() < fail_p {
            delay += if sc.courier.dual_carrier {
                sc.courier.failover_extra_min * 0.4
            } else {
                sc.courier.failover_extra_min
            };
        }
        orders[oi].courier_booked = true;
        orders[oi].match_delay = delay;
        let ni = orders[oi].node.unwrap();
        let n = &nodes[ni];
        // animation: rider spawns 1-3 km away and rides to the node
        let ang = rng.gen::<f64>() * std::f64::consts::TAU;
        let r = 1.0 + 2.0 * rng.gen::<f64>();
        trips.push((
            oi as u32,
            Trip {
                t0: now,
                t1: now + delay,
                x0: (n.spec.x_km + r * ang.cos()).clamp(0.0, sc.metro_km),
                y0: (n.spec.y_km + r * ang.sin()).clamp(0.0, sc.metro_km),
                x1: n.spec.x_km,
                y1: n.spec.y_km,
                phase: 0,
            },
        ));
        (now + delay, Ev::CourierArrive(oi as u32))
    }

    // main loop
    while let Some(Reverse((tk, _, ev))) = heap.pop() {
        let now = tk as f64 / 1e6;
        if now > horizon + 180.0 {
            break;
        }
        let hour = ((now / 60.0) as usize) % 24;
        match ev {
            Ev::Arrival(oi) => {
                arrived_ctr += 1;
                hourly_orders[hour] += 1.0;
                zone_orders[orders[oi as usize].zone] += 1.0;
                pending.push(oi);
            }
            Ev::Wave => {
                // build candidates
                let mut cand: Vec<Vec<(usize, f64)>> = Vec::with_capacity(pending.len());
                let mut keep: Vec<u32> = Vec::new();
                let mut wave_orders: Vec<u32> = Vec::new();
                for &oi in &pending {
                    let o = &orders[oi as usize];
                    let age = now - o.t_click;
                    let budget = sc.sla_min - age - SAFETY_MIN;
                    let mut list: Vec<(usize, f64)> = Vec::new();
                    let mut any_geo = false;
                    for (ni, n) in nodes.iter().enumerate() {
                        if o.basket_u >= n.spec.assortment_cover {
                            continue;
                        }
                        let d = dist(n.spec.x_km, n.spec.y_km, o.x, o.y);
                        let travel = d / sc.courier.speed_kmh * 60.0 * traffic_factor(hour);
                        let q_est = (n.queue.len() as f64 / n.spec.pickers as f64)
                            * sc.pick_mean_min;
                        let match_overhead = if sc.courier.prematch {
                            (sc.courier.match_mean_min - sc.pick_mean_min).max(0.0)
                        } else {
                            sc.courier.match_mean_min
                        };
                        let eta = travel + sc.pick_mean_min + q_est + match_overhead + HANDOVER_MIN;
                        if eta > budget {
                            continue;
                        }
                        any_geo = true;
                        let buffer = if n.spec.kind == NodeKind::Store {
                            sc.store_buffer
                        } else {
                            0.0
                        };
                        if n.ledger - buffer < 1.0 {
                            continue;
                        }
                        if (n.queue.len() as u32 + n.busy)
                            >= n.spec.pickers * QUEUE_CAP_FACTOR
                        {
                            continue;
                        }
                        let fee = sc.courier.fee_base + sc.courier.fee_per_km * d;
                        let congestion = 600.0 * (n.queue.len() as f64);
                        let displacement =
                            if n.ledger < 0.2 * n.base_stock { 2000.0 } else { 0.0 };
                        list.push((ni, fee + congestion + displacement));
                    }
                    if list.is_empty() {
                        if age > MAX_PENDING_AGE || !any_geo {
                            // decide now: downgrade or lost
                            let o = &mut orders[oi as usize];
                            if any_geo {
                                stock_denied += 1;
                            }
                            if rng.gen::<f64>() < sc.downgrade_accept {
                                o.outcome = Outcome::Downgraded;
                                px2_ctr += 1;
                                // nearest node fulfills the same-day batch
                                let (ox, oy) = (o.x, o.y);
                                let ni = nodes
                                    .iter()
                                    .enumerate()
                                    .min_by(|a, b| {
                                        dist(a.1.spec.x_km, a.1.spec.y_km, ox, oy)
                                            .partial_cmp(&dist(b.1.spec.x_km, b.1.spec.y_km, ox, oy))
                                            .unwrap()
                                    })
                                    .map(|(i, _)| i)
                                    .unwrap();
                                px2_pool.push((ox, oy, ni));
                            } else {
                                o.outcome = Outcome::Lost;
                                lost_ctr += 1;
                            }
                        } else {
                            keep.push(oi);
                        }
                    } else {
                        cand.push(list);
                        wave_orders.push(oi);
                    }
                }
                let caps: Vec<u32> = nodes
                    .iter()
                    .map(|n| {
                        (n.spec.pickers * QUEUE_CAP_FACTOR)
                            .saturating_sub(n.queue.len() as u32 + n.busy)
                    })
                    .collect();
                let assign = assign_min_cost(&cand, &caps);
                for (k, a) in assign.iter().enumerate() {
                    let oi = wave_orders[k];
                    match a {
                        Some(ni) => {
                            let o = &mut orders[oi as usize];
                            o.node = Some(*ni);
                            o.t_enqueue = now;
                            o.dist_km =
                                dist(nodes[*ni].spec.x_km, nodes[*ni].spec.y_km, o.x, o.y);
                            nodes[*ni].ledger -= 1.0;
                            nodes[*ni].queue.push_back(oi as usize);
                            try_start_picks!(*ni, now, &mut rng, &mut orders, &mut nodes);
                        }
                        None => keep.push(oi),
                    }
                }
                pending = keep;
                if now < horizon {
                    push!(now + sc.wave_seconds / 60.0, Ev::Wave);
                }
            }
            Ev::PickEnd(oi) => {
                let ni = orders[oi as usize].node.unwrap();
                nodes[ni].busy = nodes[ni].busy.saturating_sub(1);
                if !sc.courier.prematch && !orders[oi as usize].courier_booked {
                    let (t, e) = book_courier(
                        oi as usize,
                        now,
                        &mut rng,
                        &mut orders,
                        &nodes,
                        &mut trips,
                        sc,
                    );
                    push!(t, e);
                }
                // handover happens when both done
                if orders[oi as usize].t_courier > 0.0
                    && orders[oi as usize].t_courier <= now
                {
                    schedule_delivery(
                        oi as usize, now, &mut orders, &nodes, &mut trips, sc, &travel_noise,
                        &mut rng, &mut |t, e| {
                            seq += 1;
                            heap.push(Reverse(((t * 1e6) as u64, seq, e)));
                        },
                    );
                }
                try_start_picks!(ni, now, &mut rng, &mut orders, &mut nodes);
            }
            Ev::CourierArrive(oi) => {
                orders[oi as usize].t_courier = now;
                match_delays.push(orders[oi as usize].match_delay);
                if orders[oi as usize].t_pick_end > 0.0
                    && orders[oi as usize].t_pick_end <= now
                {
                    schedule_delivery(
                        oi as usize, now, &mut orders, &nodes, &mut trips, sc, &travel_noise,
                        &mut rng, &mut |t, e| {
                            seq += 1;
                            heap.push(Reverse(((t * 1e6) as u64, seq, e)));
                        },
                    );
                }
            }
            Ev::Deliver(oi) => {
                let o = &mut orders[oi as usize];
                o.t_deliver = now;
                let ctd = now - o.t_click;
                ctds.push(ctd);
                travels.push(now - o.t_pick_end.max(o.t_courier) - HANDOVER_MIN);
                dists.push(o.dist_km);
                ctd_sum += ctd;
                courier_gross_cum += sc.courier.fee_base + sc.courier.fee_per_km * o.dist_km;
                let ni = o.node.unwrap();
                if nodes[ni].spec.kind == NodeKind::Store {
                    store_served_ctr += 1;
                }
                nodes[ni].served += 1;
                served_ctr += 1;
                let on_time = ctd <= sc.sla_min;
                if on_time {
                    nodes[ni].on_time += 1;
                    ontime_ctr += 1;
                    hourly_ontime[hour] += 1.0;
                    zone_ontime[o.zone] += 1.0;
                }
                o.outcome = if on_time { Outcome::ServedOnTime } else { Outcome::ServedLate };
                hourly_served[hour] += 1.0;
                hourly_ctd[hour] += ctd;
                if capture_frames {
                    drops_buf.push((o.x as f32, o.y as f32, on_time as u8));
                }
            }
            Ev::Px2Dispatch => {
                dispatch_px2(
                    &mut px2_pool, &nodes, sc,
                    &mut px2_cost_cum, &mut px2_naive_cum, &mut px2_km_cum,
                    &mut px2_batches, &mut px2_dropcnt,
                );
                if now < horizon {
                    push!(now + 30.0, Ev::Px2Dispatch);
                }
            }
            Ev::Replenish => {
                // route the wave: hub gateway -> nodes with a deficit (CVRP)
                let stops: Vec<vrp::Stop> = nodes
                    .iter()
                    .filter(|n| n.base_stock - n.stock > 0.5)
                    .map(|n| vrp::Stop {
                        x: n.spec.x_km,
                        y: n.spec.y_km,
                        demand: n.base_stock - n.stock,
                    })
                    .collect();
                if !stops.is_empty() {
                    let depot = (0.06 * sc.metro_km, 0.06 * sc.metro_km);
                    let sol = vrp::solve_cvrp(depot, &stops, sc.midmile.truck_capacity_units.max(1.0));
                    midmile_km_cum += sol.total_km;
                    midmile_trucks += sol.routes.len() as u64;
                    midmile_cost_cum += sol.routes.len() as f64 * sc.midmile.truck_fixed_idr
                        + sol.total_km * sc.midmile.truck_per_km_idr;
                }
                for n in nodes.iter_mut() {
                    if rng.gen::<f64>() < sc.midmile.otif {
                        n.stock = n.base_stock;
                        n.ledger = n.base_stock
                            - n.queue.len() as f64
                            - n.busy as f64;
                    }
                    // OTIF miss: wave skipped, stock carries over
                }
                if now < horizon {
                    push!(now + sc.midmile.period_days * 1440.0, Ev::Replenish);
                }
            }
            Ev::PosSale(ni) => {
                let minute_of_day = now % 1440.0;
                if (540.0..1260.0).contains(&minute_of_day) {
                    let n = &mut nodes[ni as usize];
                    if n.stock > 0.0 {
                        n.stock -= 1.0;
                        push!(now + sc.pos_latency_min, Ev::LedgerPost(ni));
                    }
                }
                let dt = -rng.gen::<f64>().max(1e-12).ln() / pos_rate_per_min;
                let mut nxt = now + dt;
                if nxt % 1440.0 > 1260.0 {
                    nxt = (nxt / 1440.0).floor() * 1440.0 + 1440.0 + 540.0;
                }
                if nxt < horizon {
                    push!(nxt, Ev::PosSale(ni));
                }
            }
            Ev::LedgerPost(ni) => {
                let n = &mut nodes[ni as usize];
                n.ledger = (n.ledger - 1.0).max(0.0);
            }
            Ev::MinuteTick => {
                // utilization sampling
                for n in nodes.iter_mut() {
                    n.busy_minutes += n.busy as f64;
                }
                let q_tot: f64 = nodes.iter().map(|n| n.queue.len() as f64).sum();
                hourly_q[hour] += q_tot;
                hourly_q_n[hour] += 1.0;
                // hourly promise-shown sampling (every 60 min)
                if (now as u64).is_multiple_of(60) {
                    let mut samp_tot = 0.0f64;
                    let mut samp_shown = 0.0f64;
                    for (zi, w) in field.weights.iter().enumerate() {
                        let (zx, zy) = field.cell_center(zi);
                        let shown = nodes.iter().any(|n| {
                            let d = dist(n.spec.x_km, n.spec.y_km, zx, zy);
                            let travel =
                                d / sc.courier.speed_kmh * 60.0 * traffic_factor(hour);
                            let buffer = if n.spec.kind == NodeKind::Store {
                                sc.store_buffer
                            } else {
                                0.0
                            };
                            travel
                                + sc.pick_mean_min
                                + sc.courier.match_mean_min
                                + HANDOVER_MIN
                                < sc.sla_min - SAFETY_MIN
                                && n.ledger - buffer >= 1.0
                        });
                        promise_samples += w;
                        samp_tot += w;
                        if shown {
                            promise_shown += w;
                            samp_shown += w;
                        }
                    }
                    if samp_tot > 0.0 {
                        promise_now_var = samp_shown / samp_tot;
                    }
                }
                // frames
                if capture_frames && now <= 1440.0 * sc.days.min(2) as f64 {
                    trips.retain(|(_, t)| t.t1 >= now - 1.0);
                    let mut cs: Vec<(f32, f32, u8)> = Vec::new();
                    for (_, t) in trips.iter() {
                        if t.t0 <= now && now <= t.t1 {
                            let a = ((now - t.t0) / (t.t1 - t.t0).max(1e-9)).clamp(0.0, 1.0);
                            cs.push((
                                (t.x0 + a * (t.x1 - t.x0)) as f32,
                                (t.y0 + a * (t.y1 - t.y0)) as f32,
                                t.phase,
                            ));
                            if cs.len() >= 500 {
                                break;
                            }
                        }
                    }
                    let busy_now: u32 = nodes.iter().map(|n| n.busy).sum();
                    let contrib_now = margin_per_order * served_ctr as f64
                        + margin_per_order * 0.95 * px2_ctr as f64
                        - (courier_gross_cum - served_ctr as f64 * sc.delivery_fee_idr).max(0.0)
                        - px2_cost_cum
                        - midmile_cost_cum
                        - store_served_ctr as f64 * sc.store_fee_per_order
                        - fixed_per_min * now;
                    frames.push(Frame {
                        minute: now as u32,
                        couriers: cs,
                        queues: nodes
                            .iter()
                            .map(|n| n.queue.len() as u16 + n.busy as u16)
                            .collect(),
                        drops: std::mem::take(&mut drops_buf),
                        orders_so_far: served_ctr,
                        on_time_so_far: ontime_ctr,
                        arrived_so_far: arrived_ctr,
                        lost_so_far: lost_ctr,
                        px2_so_far: px2_ctr,
                        waited_so_far: waited_pos as u32,
                        ctd_sum_min: ctd_sum as f32,
                        contrib_idr: contrib_now,
                        promise_now: promise_now_var as f32,
                        busy_pickers: busy_now as u16,
                        total_pickers: total_pickers as u16,
                    });
                }
                if now < horizon {
                    push!(now + 1.0, Ev::MinuteTick);
                }
            }
        }
    }

    // ---------- aggregate ----------
    // flush PX2 orders that arrived after the last dispatch window so every
    // accepted same-day order is batched exactly once (conservation)
    dispatch_px2(
        &mut px2_pool, &nodes, sc,
        &mut px2_cost_cum, &mut px2_naive_cum, &mut px2_km_cum,
        &mut px2_batches, &mut px2_dropcnt,
    );
    let mut out = build_output(sc, &field, orders, nodes, SimAgg {
        ctds, pick_waits, match_delays, travels, dists, waited_pos,
        hourly_orders, hourly_ontime, hourly_served, hourly_ctd, hourly_q, hourly_q_n,
        promise_samples, promise_shown, zone_orders, zone_ontime,
        stock_denied: stock_denied as f64,
        frames,
        courier_gross: courier_gross_cum,
        px2_cost: px2_cost_cum,
        px2_naive: px2_naive_cum,
        px2_km: px2_km_cum,
        px2_batches: px2_batches as f64,
        px2_drops: px2_dropcnt as f64,
        midmile_km: midmile_km_cum,
        midmile_trucks: midmile_trucks as f64,
        midmile_cost: midmile_cost_cum,
    });
    out.px2_drops_total = px2_dropcnt;
    out.px2_accepted_total = px2_ctr as u64;
    out
}

#[allow(clippy::too_many_arguments)]
fn schedule_delivery(
    oi: usize,
    now: f64,
    orders: &mut [Order],
    _nodes: &[NodeState],
    trips: &mut Vec<(u32, Trip)>,
    sc: &Scenario,
    travel_noise: &LogNormal<f64>,
    rng: &mut Pcg64,
    push: &mut dyn FnMut(f64, Ev),
) {
    if orders[oi].dispatched {
        return; // tie between PickEnd and CourierArrive at the same tick
    }
    orders[oi].dispatched = true;
    let o = &orders[oi];
    let hour = ((now / 60.0) as usize) % 24;
    let travel =
        o.dist_km / sc.courier.speed_kmh * 60.0 * traffic_factor(hour) * travel_noise.sample(rng);
    let t_dep = now + HANDOVER_MIN;
    let t_arr = t_dep + travel.max(0.5);
    let ni = o.node.unwrap();
    let n = &_nodes[ni];
    trips.push((
        oi as u32,
        Trip {
            t0: t_dep,
            t1: t_arr,
            x0: n.spec.x_km,
            y0: n.spec.y_km,
            x1: o.x,
            y1: o.y,
            phase: 1,
        },
    ));
    push(t_arr, Ev::Deliver(oi as u32));
}


/// Form and cost PX2 batches from the pending pool (multi-pickup
/// multi-delivery; <= 8 drops; nearby nodes' small batches merge).
#[allow(clippy::too_many_arguments)]
fn dispatch_px2(
    pool: &mut Vec<(f64, f64, usize)>,
    nodes: &[NodeState],
    sc: &Scenario,
    px2_cost_cum: &mut f64,
    px2_naive_cum: &mut f64,
    px2_km_cum: &mut f64,
    px2_batches: &mut u64,
    px2_dropcnt: &mut u64,
) {
    if pool.is_empty() {
        return;
    }
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<usize, Vec<(f64, f64)>> = BTreeMap::new();
    for &(x, y, ni) in pool.iter() {
        groups.entry(ni).or_default().push((x, y));
        *px2_naive_cum += sc.courier.fee_base
            + sc.courier.fee_per_km * dist(nodes[ni].spec.x_km, nodes[ni].spec.y_km, x, y);
    }
    pool.clear();
    let mut open: Vec<(Vec<usize>, Vec<(f64, f64)>)> = Vec::new();
    for (ni, drops) in groups {
        for chunk in drops.chunks(8) {
            open.push((vec![ni], chunk.to_vec()));
        }
    }
    let mut merged: Vec<(Vec<usize>, Vec<(f64, f64)>)> = Vec::new();
    while let Some((nis, drops)) = open.pop() {
        if drops.len() <= 3 {
            if let Some(pos) = open.iter().position(|(onis, od)| {
                od.len() + drops.len() <= 8
                    && onis.len() == 1
                    && nis.len() == 1
                    && dist(
                        nodes[onis[0]].spec.x_km,
                        nodes[onis[0]].spec.y_km,
                        nodes[nis[0]].spec.x_km,
                        nodes[nis[0]].spec.y_km,
                    ) <= 3.0
            }) {
                let (mut onis, mut od) = open.remove(pos);
                onis.extend(nis);
                od.extend(drops);
                merged.push((onis, od));
                continue;
            }
        }
        merged.push((nis, drops));
    }
    for (nis, drops) in merged {
        let pickups: Vec<(f64, f64)> = nis
            .iter()
            .map(|&ni| (nodes[ni].spec.x_km, nodes[ni].spec.y_km))
            .collect();
        let km = vrp::rider_batch_km(&pickups, &drops);
        *px2_cost_cum += sc.courier.fee_base + sc.courier.fee_per_km * km;
        *px2_km_cum += km;
        *px2_batches += 1;
        *px2_dropcnt += drops.len() as u64;
    }
}

struct SimAgg {
    ctds: Vec<f64>,
    pick_waits: Vec<f64>,
    match_delays: Vec<f64>,
    travels: Vec<f64>,
    dists: Vec<f64>,
    waited_pos: u64,
    hourly_orders: Vec<f64>,
    hourly_ontime: Vec<f64>,
    hourly_served: Vec<f64>,
    hourly_ctd: Vec<f64>,
    hourly_q: Vec<f64>,
    hourly_q_n: Vec<f64>,
    promise_samples: f64,
    promise_shown: f64,
    zone_orders: Vec<f64>,
    zone_ontime: Vec<f64>,
    stock_denied: f64,
    frames: Vec<Frame>,
    courier_gross: f64,
    px2_cost: f64,
    px2_naive: f64,
    px2_km: f64,
    px2_batches: f64,
    px2_drops: f64,
    midmile_km: f64,
    midmile_trucks: f64,
    midmile_cost: f64,
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn build_output(
    sc: &Scenario,
    field: &DemandField,
    orders: Vec<Order>,
    nodes: Vec<NodeState>,
    agg: SimAgg,
) -> SimOutput {
    let days = sc.days as f64;
    let total = orders.len() as f64;
    let served: f64 = orders
        .iter()
        .filter(|o| matches!(o.outcome, Outcome::ServedOnTime | Outcome::ServedLate))
        .count() as f64;
    let on_time: f64 = orders.iter().filter(|o| o.outcome == Outcome::ServedOnTime).count() as f64;
    let downgraded =
        orders.iter().filter(|o| o.outcome == Outcome::Downgraded).count() as f64;
    let lost = orders
        .iter()
        .filter(|o| matches!(o.outcome, Outcome::Lost | Outcome::Pending))
        .count() as f64;
    let oversell = orders.iter().filter(|o| o.outcome == Outcome::Oversell).count() as f64;

    let mut ctds = agg.ctds.clone();
    ctds.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // economics (per day)
    let margin = sc.aov_idr * sc.margin_pct;
    let revenue_margin = (served * margin + downgraded * margin * 0.95) / days;
    // PX1 courier: actual accumulated fees net of the delivery fee charged;
    // PX2 courier: the routed multi-pickup multi-delivery batch cost (VRP)
    let courier_cost =
        ((agg.courier_gross - served * sc.delivery_fee_idr).max(0.0) + agg.px2_cost) / days;
    let labor: f64 = nodes
        .iter()
        .filter(|n| n.spec.kind != NodeKind::Store)
        .map(|n| n.spec.pickers as f64 * 14.0 * sc.wage_per_hour)
        .sum();
    let rent: f64 = nodes
        .iter()
        .map(|n| match n.spec.kind {
            NodeKind::Dark => sc.rent_dark_m * 1e6 / 30.0,
            NodeKind::Flagship => sc.rent_flag_m * 1e6 / 30.0,
            NodeKind::Store => 0.0,
        })
        .sum();
    let store_orders: f64 = nodes
        .iter()
        .filter(|n| n.spec.kind == NodeKind::Store)
        .map(|n| n.served as f64)
        .sum::<f64>()
        / days;
    let store_fees = store_orders * sc.store_fee_per_order;
    let midmile = agg.midmile_cost / days;
    let contribution = revenue_margin - courier_cost - labor - rent - store_fees - midmile;
    let served_per_day = (served + downgraded) / days;

    let minutes = days * 1440.0;
    let util_num: f64 = nodes.iter().map(|n| n.busy_minutes).sum();
    let util_den: f64 = nodes
        .iter()
        .map(|n| n.spec.pickers as f64 * minutes)
        .sum();

    let kpi = KpiSummary {
        orders_total: total / days,
        px1_served: served / days,
        px1_on_time: on_time / days,
        px2_downgraded: downgraded / days,
        lost_sales: lost / days,
        oversell_events: oversell / days,
        sla_attainment: if served > 0.0 { on_time / served } else { 0.0 },
        p50_click_to_door_min: pct(&ctds, 0.50),
        p95_click_to_door_min: pct(&ctds, 0.95),
        avg_pick_wait_min: mean(&agg.pick_waits),
        delay_prob: if agg.pick_waits.is_empty() {
            0.0
        } else {
            agg.waited_pos as f64 / agg.pick_waits.len() as f64
        },
        avg_courier_match_min: mean(&agg.match_delays),
        avg_travel_min: mean(&agg.travels),
        avg_distance_km: mean(&agg.dists),
        fill_rate: if total > 0.0 {
            1.0 - (agg.stock_denied + oversell) / total
        } else {
            1.0
        },
        revenue_margin_idr: revenue_margin,
        courier_cost_idr: courier_cost,
        labor_cost_idr: labor,
        rent_cost_idr: rent,
        midmile_cost_idr: midmile,
        store_fees_idr: store_fees,
        contribution_idr: contribution,
        contribution_per_order: if served_per_day > 0.0 {
            contribution / served_per_day
        } else {
            0.0
        },
        promise_shown_rate: if agg.promise_samples > 0.0 {
            agg.promise_shown / agg.promise_samples
        } else {
            0.0
        },
        picker_utilization: if util_den > 0.0 { util_num / util_den } else { 0.0 },
        midmile_km_per_day: agg.midmile_km / days,
        midmile_trucks_per_day: agg.midmile_trucks / days,
        px2_rider_km_per_day: agg.px2_km / days,
        px2_drops_per_batch: if agg.px2_batches > 0.0 { agg.px2_drops / agg.px2_batches } else { 0.0 },
        px2_batch_savings_pct: if agg.px2_naive > 0.0 {
            (1.0 - agg.px2_cost / agg.px2_naive).max(0.0)
        } else {
            0.0
        },
    };

    let hourly = (0..24)
        .map(|h| HourPoint {
            hour: h as u32,
            orders: agg.hourly_orders[h] / days,
            on_time_rate: if agg.hourly_served[h] > 0.0 {
                agg.hourly_ontime[h] / agg.hourly_served[h]
            } else {
                0.0
            },
            avg_ctd_min: if agg.hourly_served[h] > 0.0 {
                agg.hourly_ctd[h] / agg.hourly_served[h]
            } else {
                0.0
            },
            queue_len: if agg.hourly_q_n[h] > 0.0 {
                agg.hourly_q[h] / agg.hourly_q_n[h]
            } else {
                0.0
            },
        })
        .collect();

    let node_stats = nodes
        .iter()
        .map(|n| NodeStat {
            id: n.spec.id,
            kind: n.spec.kind,
            x_km: n.spec.x_km,
            y_km: n.spec.y_km,
            orders: n.served as f64 / days,
            utilization: n.busy_minutes / (n.spec.pickers as f64 * minutes),
            on_time_rate: if n.served > 0 {
                n.on_time as f64 / n.served as f64
            } else {
                0.0
            },
            stockout_rate: if n.served + n.stockouts > 0 {
                n.stockouts as f64 / (n.served + n.stockouts) as f64
            } else {
                0.0
            },
        })
        .collect();

    let radius_free =
        (sc.courier.speed_kmh / 60.0) * (sc.sla_min - sc.pick_mean_min - sc.courier.match_mean_min - 8.0).max(0.0);
    let zones = (0..field.weights.len())
        .map(|zi| {
            let (zx, zy) = field.cell_center(zi);
            let covered = sc
                .nodes
                .iter()
                .any(|n| dist(n.x_km, n.y_km, zx, zy) <= radius_free);
            ZoneStat {
                zx: (zi % field.cells) as u32,
                zy: (zi / field.cells) as u32,
                demand: field.weights[zi],
                on_time_rate: if agg.zone_orders[zi] > 0.0 {
                    agg.zone_ontime[zi] / agg.zone_orders[zi]
                } else {
                    -1.0
                },
                covered,
            }
        })
        .collect();

    SimOutput {
        kpi,
        hourly,
        nodes: node_stats,
        zones,
        frames: agg.frames,
        px2_drops_total: 0,
        px2_accepted_total: 0,
    }
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}
