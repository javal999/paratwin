//! Exact wave routing: min-cost assignment of orders to feasible nodes with
//! per-node wave capacity, via successive-shortest-path min-cost flow on the
//! bipartite graph (Theorem 5 of the report: the transportation polytope is
//! integral, so the LP optimum is attained at an integral assignment).

/// candidates[i] = list of (node_index, cost) feasible for order i.
/// capacity[j] = remaining wave slots of node j.
/// Returns assignment[i] = Some(node_index) or None (infeasible / dropped).
pub fn assign_min_cost(
    candidates: &[Vec<(usize, f64)>],
    capacity: &[u32],
) -> Vec<Option<usize>> {
    let n_orders = candidates.len();
    let n_nodes = capacity.len();
    if n_orders == 0 {
        return Vec::new();
    }
    // Exact integer costs (milli-units): floating-point costs in a residual
    // graph can create phantom negative cycles from rounding residue, which
    // livelocks Bellman-Ford. Integers make SSP provably terminating.
    let cand_i: Vec<Vec<(usize, i64)>> = candidates
        .iter()
        .map(|l| {
            l.iter()
                .map(|&(j, c)| (j, (c.max(0.0) * 1000.0).round() as i64))
                .collect()
        })
        .collect();

    // Node indexing: 0 = source, 1..=n_orders = orders,
    // n_orders+1..=n_orders+n_nodes = fulfillment nodes, last = sink.
    let s = 0usize;
    let t = 1 + n_orders + n_nodes;
    let vcount = t + 1;

    #[derive(Clone)]
    struct Edge {
        to: usize,
        cap: i64,
        cost: i64,
        flow: i64,
    }
    let mut edges: Vec<Edge> = Vec::new();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); vcount];
    let add = |edges: &mut Vec<Edge>, adj: &mut Vec<Vec<usize>>, a: usize, b: usize, cap: i64, cost: i64| {
        adj[a].push(edges.len());
        edges.push(Edge { to: b, cap, cost, flow: 0 });
        adj[b].push(edges.len());
        edges.push(Edge { to: a, cap: 0, cost: -cost, flow: 0 });
    };

    for (i, cands) in cand_i.iter().enumerate() {
        add(&mut edges, &mut adj, s, 1 + i, 1, 0);
        for &(j, c) in cands {
            add(&mut edges, &mut adj, 1 + i, 1 + n_orders + j, 1, c);
        }
    }
    for (j, &cap) in capacity.iter().enumerate() {
        add(&mut edges, &mut adj, 1 + n_orders + j, t, cap as i64, 0);
    }

    // Successive shortest paths (SPFA); exact i64 arithmetic guarantees
    // termination, and each augmentation pushes exactly one order.
    const INF: i64 = i64::MAX / 4;
    for _ in 0..n_orders {
        let mut dist = vec![INF; vcount];
        let mut in_q = vec![false; vcount];
        let mut prev: Vec<Option<usize>> = vec![None; vcount];
        dist[s] = 0;
        let mut q = std::collections::VecDeque::new();
        q.push_back(s);
        in_q[s] = true;
        while let Some(u) = q.pop_front() {
            in_q[u] = false;
            for &ei in &adj[u] {
                let e = &edges[ei];
                if e.cap - e.flow > 0 && dist[u] + e.cost < dist[e.to] {
                    dist[e.to] = dist[u] + e.cost;
                    prev[e.to] = Some(ei);
                    if !in_q[e.to] {
                        q.push_back(e.to);
                        in_q[e.to] = true;
                    }
                }
            }
        }
        if dist[t] >= INF {
            break; // no more augmenting paths
        }
        // augment by 1 unit along the path
        let mut v = t;
        while let Some(ei) = prev[v] {
            edges[ei].flow += 1;
            edges[ei ^ 1].flow -= 1;
            v = edges[ei ^ 1].to;
            if v == s {
                break;
            }
        }
    }

    // read assignment
    let mut out = vec![None; n_orders];
    for i in 0..n_orders {
        for &ei in &adj[1 + i] {
            let e = &edges[ei];
            if ei % 2 == 0 && e.flow > 0 && e.to > n_orders && e.to < t {
                out[i] = Some(e.to - 1 - n_orders);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// brute force optimal assignment cost for tiny instances
    fn brute(candidates: &[Vec<(usize, f64)>], capacity: &[u32]) -> f64 {
        fn rec(
            i: usize,
            candidates: &[Vec<(usize, f64)>],
            used: &mut Vec<u32>,
            capacity: &[u32],
        ) -> f64 {
            if i == candidates.len() {
                return 0.0;
            }
            // option: leave order unassigned at high penalty
            let mut best = 1e6 + rec(i + 1, candidates, used, capacity);
            for &(j, c) in &candidates[i] {
                if used[j] < capacity[j] {
                    used[j] += 1;
                    best = best.min(c + rec(i + 1, candidates, used, capacity));
                    used[j] -= 1;
                }
            }
            best
        }
        rec(0, candidates, &mut vec![0; capacity.len()], capacity)
    }

    #[test]
    fn matches_brute_force() {
        // 5 orders, 3 nodes, mixed feasibility
        let cands: Vec<Vec<(usize, f64)>> = vec![
            vec![(0, 4.0), (1, 2.0)],
            vec![(0, 3.0), (2, 6.0)],
            vec![(1, 1.0), (2, 2.5)],
            vec![(0, 5.0), (1, 2.2), (2, 2.4)],
            vec![(2, 1.5)],
        ];
        let cap = vec![1, 2, 2];
        let assign = assign_min_cost(&cands, &cap);
        let mut cost = 0.0;
        let mut assigned = 0;
        let mut used = vec![0u32; 3];
        for (i, a) in assign.iter().enumerate() {
            if let Some(j) = a {
                assigned += 1;
                used[*j] += 1;
                cost += cands[i].iter().find(|(n, _)| n == j).unwrap().1;
            }
        }
        for j in 0..3 {
            assert!(used[j] <= cap[j], "capacity violated");
        }
        // brute force: all 5 assignable
        let b = brute(&cands, &cap);
        assert_eq!(assigned, 5, "all orders should be assigned");
        assert!((cost - b).abs() < 1e-9, "SSP cost {} != brute {}", cost, b);
    }

    #[test]
    fn respects_capacity_and_drops() {
        let cands: Vec<Vec<(usize, f64)>> =
            vec![vec![(0, 1.0)], vec![(0, 1.0)], vec![(0, 1.0)]];
        let cap = vec![2];
        let assign = assign_min_cost(&cands, &cap);
        let n = assign.iter().flatten().count();
        assert_eq!(n, 2);
    }
}
