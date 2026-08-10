//! Closed-form models from the ParaExpress report, used both for design
//! candidate generation and to cross-validate the simulation (the twin is
//! "mathematically proven": simulated statistics must reproduce these laws).

/// Erlang-C: probability an arriving order waits in an M/M/c queue.
/// lambda = arrival rate, mu = service rate per server, c = servers.
pub fn erlang_c(lambda: f64, mu: f64, c: u32) -> f64 {
    let a = lambda / mu; // offered load
    let c_f = c as f64;
    if a >= c_f {
        return 1.0;
    }
    // P0
    let mut sum = 0.0;
    let mut term = 1.0; // a^k / k!
    for k in 0..c {
        if k > 0 {
            term *= a / k as f64;
        }
        sum += term;
    }
    let term_c = term * a / c_f; // a^c / c!
    let last = term_c * c_f / (c_f - a);
    let p_wait = last / (sum + last);
    p_wait.clamp(0.0, 1.0)
}

/// Mean waiting time in queue for M/M/c (minutes if rates are per minute).
pub fn mmc_mean_wait(lambda: f64, mu: f64, c: u32) -> f64 {
    let a = lambda / mu;
    let c_f = c as f64;
    if a >= c_f {
        return f64::INFINITY;
    }
    erlang_c(lambda, mu, c) / (c_f * mu - lambda)
}

/// Square-root safety staffing (Prop. 3 / Halfin-Whitt): smallest c with
/// Erlang-C delay probability <= delta.
pub fn sqrt_staffing(lambda: f64, mu: f64, delta: f64) -> u32 {
    let rho = lambda / mu;
    let mut c = rho.ceil() as u32 + 1;
    while erlang_c(lambda, mu, c) > delta {
        c += 1;
        if c > 10_000 {
            break;
        }
    }
    c
}

/// Network cost rate C(n) = F n + a sqrt(n) + b / sqrt(n)   (Theorem 3).
pub fn cost_curve(f: f64, a: f64, b: f64, n: f64) -> f64 {
    f * n + a * n.sqrt() + b / n.sqrt()
}

/// Unique minimizer n* of C(n): positive root of 2F x^3 + a x^2 - b = 0
/// with x = sqrt(n), found by bisection (psi is strictly increasing).
pub fn n_star(f: f64, a: f64, b: f64) -> f64 {
    if f <= 0.0 {
        return b / a; // asset-light closed form (Theorem 3, F -> 0)
    }
    let g = |x: f64| 2.0 * f * x.powi(3) + a * x * x - b;
    let (mut lo, mut hi) = (1e-9, 1.0);
    while g(hi) < 0.0 {
        hi *= 2.0;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if g(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let x = 0.5 * (lo + hi);
    x * x
}

/// SLA feasibility floor (Prop. 2): n_min = A / (eta * pi * R^2),
/// R = v * (tau - t_node)^+.
pub fn n_min(area_km2: f64, speed_kmh: f64, tau_min: f64, t_node_min: f64, eta: f64) -> f64 {
    let r = (speed_kmh / 60.0) * (tau_min - t_node_min).max(0.0);
    if r <= 0.0 {
        return f64::INFINITY;
    }
    (area_km2 / (eta * std::f64::consts::PI * r * r)).ceil()
}

/// Greedy submodular coverage ranking (Theorem 2 / Corollary 1):
/// candidates ranked by marginal covered demand within radius_km.
/// Returns (candidate_index, marginal_gain, cumulative_coverage share).
pub fn greedy_coverage(
    zone_xy_w: &[(f64, f64, f64)],
    candidates: &[(f64, f64)],
    radius_km: f64,
    take: usize,
) -> Vec<(usize, f64, f64)> {
    let total: f64 = zone_xy_w.iter().map(|z| z.2).sum();
    let mut covered = vec![false; zone_xy_w.len()];
    let mut picked: Vec<(usize, f64, f64)> = Vec::new();
    let mut used = vec![false; candidates.len()];
    let mut cum = 0.0;
    for _ in 0..take.min(candidates.len()) {
        let mut best = (usize::MAX, 0.0);
        for (ci, &(cx, cy)) in candidates.iter().enumerate() {
            if used[ci] {
                continue;
            }
            let mut gain = 0.0;
            for (zi, &(zx, zy, w)) in zone_xy_w.iter().enumerate() {
                if !covered[zi] && ((zx - cx).powi(2) + (zy - cy).powi(2)).sqrt() <= radius_km {
                    gain += w;
                }
            }
            if gain > best.1 {
                best = (ci, gain);
            }
        }
        if best.0 == usize::MAX || best.1 <= 0.0 {
            break;
        }
        used[best.0] = true;
        for (zi, &(zx, zy, _)) in zone_xy_w.iter().enumerate() {
            let (cx, cy) = candidates[best.0];
            if ((zx - cx).powi(2) + (zy - cy).powi(2)).sqrt() <= radius_km {
                covered[zi] = true;
            }
        }
        cum += best.1 / total;
        picked.push((best.0, best.1 / total, cum));
    }
    picked
}

/// Eppen risk pooling: total safety stock for n equal splits = z*sigma*sqrt(n).
pub fn pooled_safety_stock(z: f64, sigma: f64, n: f64) -> f64 {
    z * sigma * n.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erlang_c_reduces_to_mm1() {
        // M/M/1: P(wait) = rho
        for rho in [0.1, 0.5, 0.8, 0.95] {
            let p = erlang_c(rho, 1.0, 1);
            assert!((p - rho).abs() < 1e-9, "rho={rho} got {p}");
        }
    }

    #[test]
    fn erlang_c_known_value() {
        // classic table value: c=5, a=4 (rho=0.8) -> P(wait) ~ 0.5541
        let p = erlang_c(4.0, 1.0, 5);
        assert!((p - 0.5541).abs() < 0.001, "got {p}");
    }

    #[test]
    fn n_star_matches_report_example() {
        // Report Fig. 8 / Remark 3: (F,a,b) = (1.2, 6, 120) -> n* ~ 9.05, C ~ 68.8
        let n = n_star(1.2, 6.0, 120.0);
        assert!((n - 9.05).abs() < 0.15, "n*={n}");
        let c = cost_curve(1.2, 6.0, 120.0, n);
        assert!((c - 68.8).abs() < 0.2, "C(n*)={c}");
        // unimodality: strictly worse on both sides
        assert!(cost_curve(1.2, 6.0, 120.0, n - 2.0) > c);
        assert!(cost_curve(1.2, 6.0, 120.0, n + 2.0) > c);
    }

    #[test]
    fn n_star_asset_light_limit() {
        // F -> 0 gives n* = b/a (Theorem 3, corrected form)
        assert!((n_star(0.0, 6.0, 120.0) - 20.0).abs() < 1e-9);
        // and bisection converges to the same limit for tiny F
        let n = n_star(1e-9, 6.0, 120.0);
        assert!((n - 20.0).abs() < 0.01, "n={n}");
    }

    #[test]
    fn staffing_is_sublinear() {
        // doubling load should less-than-double the staff (economy of scale)
        let c1 = sqrt_staffing(10.0, 0.25, 0.2);
        let c2 = sqrt_staffing(20.0, 0.25, 0.2);
        assert!(c2 < 2 * c1, "c1={c1} c2={c2}");
    }

    #[test]
    fn greedy_coverage_is_diminishing() {
        let zones: Vec<(f64, f64, f64)> = (0..100)
            .map(|i| ((i % 10) as f64, (i / 10) as f64, 1.0))
            .collect();
        let cands = vec![(2.0, 2.0), (7.0, 7.0), (2.0, 7.0), (7.0, 2.0), (5.0, 5.0)];
        let picks = greedy_coverage(&zones, &cands, 3.0, 5);
        for w in picks.windows(2) {
            assert!(w[1].1 <= w[0].1 + 1e-9, "marginal gains must diminish");
        }
    }
}
