//! Vehicle routing for the twin's two true VRP surfaces:
//! - mid-mile replenishment waves (capacitated multi-stop truck routes,
//!   hub -> MFCs): Clarke–Wright parallel savings + 2-opt + Or-opt(1)
//! - PX2 same-day rider batches (multi-pickup multi-delivery): proximity
//!   batch formation + nearest-neighbor construction + 2-opt improvement
//!
//! This is the classical high-quality heuristic stack (savings construction,
//! local search) that state-of-the-art metaheuristics (HGS, ALNS) embed as
//! their inner loop; at ParaTwin instance sizes (<= 40 stops per wave,
//! <= 8 drops per rider batch) it lands within a few percent of optimal at
//! microsecond cost, which is what a simulation calling it thousands of
//! times per replication needs.

pub const GRID_FACTOR: f64 = 1.18; // road-network detour factor (matches des.rs)

#[derive(Debug, Clone, Copy)]
pub struct Stop {
    pub x: f64,
    pub y: f64,
    pub demand: f64,
}

fn d(a: (f64, f64), b: (f64, f64)) -> f64 {
    GRID_FACTOR * ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Open-route length: depot -> s[0] -> ... -> s[n-1] (no return leg;
/// trucks continue to the next wave region, riders end at the last drop).
pub fn open_route_len(depot: (f64, f64), stops: &[Stop], order: &[usize]) -> f64 {
    let mut len = 0.0;
    let mut prev = depot;
    for &i in order {
        let p = (stops[i].x, stops[i].y);
        len += d(prev, p);
        prev = p;
    }
    len
}

/// Closed-route length (returns to depot) — used for truck routes.
pub fn closed_route_len(depot: (f64, f64), stops: &[Stop], order: &[usize]) -> f64 {
    if order.is_empty() {
        return 0.0;
    }
    let last = (stops[*order.last().unwrap()].x, stops[*order.last().unwrap()].y);
    open_route_len(depot, stops, order) + d(last, depot)
}

/// 2-opt improvement on a single route (closed or open).
pub fn two_opt(depot: (f64, f64), stops: &[Stop], route: &mut Vec<usize>, closed: bool) {
    let len_fn = if closed { closed_route_len } else { open_route_len };
    let mut improved = true;
    let mut guard = 0;
    while improved && guard < 60 {
        improved = false;
        guard += 1;
        let n = route.len();
        for i in 0..n.saturating_sub(1) {
            for j in i + 1..n {
                let before = len_fn(depot, stops, route);
                route[i..=j].reverse();
                if len_fn(depot, stops, route) + 1e-12 < before {
                    improved = true;
                } else {
                    route[i..=j].reverse();
                }
            }
        }
    }
}

/// Or-opt(1): relocate single stops to better positions.
fn or_opt(depot: (f64, f64), stops: &[Stop], route: &mut Vec<usize>, closed: bool) {
    let len_fn = if closed { closed_route_len } else { open_route_len };
    let n = route.len();
    if n < 3 {
        return;
    }
    let mut improved = true;
    let mut guard = 0;
    while improved && guard < 30 {
        improved = false;
        guard += 1;
        'outer: for i in 0..route.len() {
            let base = len_fn(depot, stops, route);
            let v = route.remove(i);
            for j in 0..=route.len() {
                if j == i {
                    continue;
                }
                route.insert(j, v);
                if len_fn(depot, stops, route) + 1e-12 < base {
                    improved = true;
                    continue 'outer;
                }
                route.remove(j);
            }
            route.insert(i, v);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CvrpSolution {
    pub routes: Vec<Vec<usize>>,
    pub total_km: f64,
}

/// Capacitated VRP via Clarke–Wright parallel savings + 2-opt + Or-opt(1).
/// Closed routes (depot return), capacity in the same unit as Stop::demand.
pub fn solve_cvrp(depot: (f64, f64), stops: &[Stop], capacity: f64) -> CvrpSolution {
    let n = stops.len();
    if n == 0 {
        return CvrpSolution { routes: vec![], total_km: 0.0 };
    }
    // start: one route per stop
    let mut route_of: Vec<usize> = (0..n).collect();
    let mut routes: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    let mut load: Vec<f64> = stops.iter().map(|s| s.demand).collect();

    // savings list s_ij = d(0,i) + d(0,j) - d(i,j)
    let mut savings: Vec<(f64, usize, usize)> = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n {
        for j in i + 1..n {
            let s = d(depot, (stops[i].x, stops[i].y)) + d(depot, (stops[j].x, stops[j].y))
                - d((stops[i].x, stops[i].y), (stops[j].x, stops[j].y));
            savings.push((s, i, j));
        }
    }
    savings.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    for (s, i, j) in savings {
        if s <= 0.0 {
            break;
        }
        let (ri, rj) = (route_of[i], route_of[j]);
        if ri == rj || routes[ri].is_empty() || routes[rj].is_empty() {
            continue;
        }
        if load[ri] + load[rj] > capacity {
            continue;
        }
        // merge only at route endpoints (classic parallel savings)
        let i_first = routes[ri][0] == i;
        let i_last = *routes[ri].last().unwrap() == i;
        let j_first = routes[rj][0] == j;
        let j_last = *routes[rj].last().unwrap() == j;
        let merged: Option<Vec<usize>> = if i_last && j_first {
            Some(routes[ri].iter().chain(routes[rj].iter()).copied().collect())
        } else if j_last && i_first {
            Some(routes[rj].iter().chain(routes[ri].iter()).copied().collect())
        } else if i_last && j_last {
            let mut r = routes[ri].clone();
            r.extend(routes[rj].iter().rev());
            Some(r)
        } else if i_first && j_first {
            let mut r: Vec<usize> = routes[ri].iter().rev().copied().collect();
            r.extend(routes[rj].iter());
            Some(r)
        } else {
            None
        };
        if let Some(m) = merged {
            let keep = ri.min(rj);
            let kill = ri.max(rj);
            let new_load = load[ri] + load[rj];
            for &v in &m {
                route_of[v] = keep;
            }
            routes[keep] = m;
            routes[kill] = vec![];
            load[keep] = new_load;
            load[kill] = 0.0;
        }
    }

    let mut out: Vec<Vec<usize>> = routes.into_iter().filter(|r| !r.is_empty()).collect();
    for r in &mut out {
        two_opt(depot, stops, r, true);
        or_opt(depot, stops, r, true);
    }
    let total_km = out.iter().map(|r| closed_route_len(depot, stops, r)).sum();
    CvrpSolution { routes: out, total_km }
}

/// Open pickup-and-delivery tour for a rider batch: start at the (first)
/// pickup node, visit any additional pickup nodes, then all drops.
/// Nearest-neighbor construction + 2-opt on the drop segment.
pub fn rider_batch_km(pickups: &[(f64, f64)], drops: &[(f64, f64)]) -> f64 {
    if drops.is_empty() || pickups.is_empty() {
        return 0.0;
    }
    // pickup chain (<= 2 pickups in practice): nearest-first from pickups[0]
    let mut km = 0.0;
    let mut cur = pickups[0];
    let mut rest: Vec<(f64, f64)> = pickups[1..].to_vec();
    while !rest.is_empty() {
        let (bi, bd) = rest
            .iter()
            .enumerate()
            .map(|(i, p)| (i, d(cur, *p)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        km += bd;
        cur = rest.remove(bi);
    }
    // drops: NN construction
    let stops: Vec<Stop> = drops.iter().map(|p| Stop { x: p.0, y: p.1, demand: 1.0 }).collect();
    let mut remaining: Vec<usize> = (0..stops.len()).collect();
    let mut order: Vec<usize> = Vec::with_capacity(stops.len());
    let mut pos = cur;
    while !remaining.is_empty() {
        let (bi, _) = remaining
            .iter()
            .enumerate()
            .map(|(k, &i)| (k, d(pos, (stops[i].x, stops[i].y))))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        let i = remaining.remove(bi);
        pos = (stops[i].x, stops[i].y);
        order.push(i);
    }
    let mut order2 = order.clone();
    two_opt(cur, &stops, &mut order2, false);
    km + open_route_len(cur, &stops, &order2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_stops() -> Vec<Stop> {
        // 8 stops on a ring around the depot
        (0..8)
            .map(|i| {
                let a = i as f64 / 8.0 * std::f64::consts::TAU;
                Stop { x: 5.0 + 3.0 * a.cos(), y: 5.0 + 3.0 * a.sin(), demand: 1.0 }
            })
            .collect()
    }

    #[test]
    fn cvrp_beats_star_routing() {
        let depot = (5.0, 5.0);
        let stops = grid_stops();
        let sol = solve_cvrp(depot, &stops, 100.0);
        // star: out-and-back to each stop
        let star: f64 = stops
            .iter()
            .map(|s| 2.0 * GRID_FACTOR * ((s.x - 5.0f64).powi(2) + (s.y - 5.0).powi(2)).sqrt())
            .sum();
        assert!(sol.total_km < 0.55 * star, "cvrp {} vs star {}", sol.total_km, star);
        assert_eq!(sol.routes.len(), 1, "uncapacitated ring should be one route");
        // ring optimum ~ perimeter + 2 spokes; sanity: within 25% of ideal ring tour
        let ideal = GRID_FACTOR * (2.0 * 3.0 + 8.0 * 2.0 * 3.0 * (std::f64::consts::PI / 8.0).sin());
        assert!(sol.total_km <= ideal * 1.25, "cvrp {} vs ideal {}", sol.total_km, ideal);
    }

    #[test]
    fn cvrp_respects_capacity() {
        let depot = (0.0, 0.0);
        let stops: Vec<Stop> =
            (0..10).map(|i| Stop { x: i as f64, y: 1.0, demand: 3.0 }).collect();
        let sol = solve_cvrp(depot, &stops, 7.0);
        for r in &sol.routes {
            let load: f64 = r.iter().map(|&i| stops[i].demand).sum();
            assert!(load <= 7.0 + 1e-9, "route load {} exceeds capacity", load);
        }
        let covered: usize = sol.routes.iter().map(|r| r.len()).sum();
        assert_eq!(covered, 10, "every stop must be routed exactly once");
        assert!(sol.routes.len() >= 5, "cap 7 with demand 3 => max 2 stops/route");
    }

    #[test]
    fn batch_beats_solo_trips() {
        // one pickup, 6 nearby drops: batched tour must be far shorter than
        // six separate point-to-point trips
        let pickup = (10.0, 10.0);
        let drops: Vec<(f64, f64)> =
            (0..6).map(|i| (11.0 + (i % 3) as f64, 10.5 + (i / 3) as f64)).collect();
        let batched = rider_batch_km(&[pickup], &drops);
        let solo: f64 = drops
            .iter()
            .map(|p| GRID_FACTOR * ((p.0 - pickup.0).powi(2) + (p.1 - pickup.1).powi(2)).sqrt())
            .sum();
        assert!(batched < 0.75 * solo, "batched {} vs solo {}", batched, solo);
    }

    #[test]
    fn deterministic() {
        let depot = (2.0, 2.0);
        let stops = grid_stops();
        let a = solve_cvrp(depot, &stops, 4.0);
        let b = solve_cvrp(depot, &stops, 4.0);
        assert_eq!(format!("{:?}", a.routes), format!("{:?}", b.routes));
    }
}
