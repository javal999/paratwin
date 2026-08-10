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

/// Greedy coverage ranking (Theorem 2 / Corollary 1): candidates ranked by
/// marginal covered demand within radius_km, with marginals recomputed after
/// every activation rather than ranked once and iterated.
/// Returns (candidate_index, marginal_gain, cumulative_coverage share).
///
/// No (1-1/e) guarantee is claimed. That bound is proved for uncapacitated
/// nodes; under binding throughput caps and whole-order assignment, capacity
/// creates complementarity and coverage is not submodular, so it does not
/// transfer. See `capacitated_coverage` for the counterexample (Rev 5, C-07).
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

// ---------------- Revision 5 corrections ----------------

/// PTW-01 / C-01: standard normal inverse CDF.
///
/// Acklam's rational approximation (relative error < 1.15e-9), refined by one
/// Halley step against erf so the result is accurate to full f64 precision.
/// C-01 sizes base stock from this, so the quantile has to be exact rather
/// than table-lookup close.
pub fn phi_inv(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    const A: [f64; 6] = [
        -3.969683028665376e+01, 2.209460984245205e+02, -2.759285104469687e+02,
        1.38357751867269e+02, -3.066479806614716e+01, 2.50662827745924e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01, 1.615858368580409e+02, -1.556989798598866e+02,
        6.680131188771972e+01, -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03, -3.223964580411365e-01, -2.400758277161838e+00,
        -2.549732539343734e+00, 4.374664141464968e+00, 2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03, 3.224671290700398e-01, 2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    const P_LOW: f64 = 0.02425;

    let x = if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - P_LOW {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };

    // one Halley refinement: e = Phi(x) - p, u = e * sqrt(2pi) * exp(x^2/2)
    let e = 0.5 * erfc(-x / std::f64::consts::SQRT_2) - p;
    let u = e * (2.0 * std::f64::consts::PI).sqrt() * (x * x / 2.0).exp();
    x - u / (1.0 + x * u / 2.0)
}

/// Complementary error function, needed only to refine `phi_inv`.
/// Numerical Recipes' Chebyshev form; fractional error < 1.2e-7, which is far
/// inside what one Halley step needs to converge.
fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 2.0 / (2.0 + z);
    let ty = 4.0 * t - 2.0;
    const COF: [f64; 28] = [
        -1.3026537197817094, 6.419697923564903e-1, 1.9476473204185836e-2,
        -9.56151478680863e-3, -9.46595344482036e-4, 3.66839497852761e-4,
        4.2523324806907e-5, -2.0278578112534e-5, -1.624290004647e-6,
        1.303655835580e-6, 1.5626441722e-8, -8.5238095915e-8,
        6.529054439e-9, 5.059343495e-9, -9.91364156e-10,
        -2.27365122e-10, 9.6467911e-11, 2.394038e-12,
        -6.886027e-12, 8.94487e-13, 3.13092e-13,
        -1.12708e-13, 3.81e-16, 7.106e-15,
        -1.523e-15, -9.4e-17, 1.21e-16, -2.8e-17,
    ];
    let mut d = 0.0f64;
    let mut dd = 0.0f64;
    for j in (1..COF.len()).rev() {
        let tmp = d;
        d = ty * d - dd + COF[j];
        dd = tmp;
    }
    let ans = t * (-z * z + 0.5 * (COF[0] + ty * d) - dd).exp();
    if x >= 0.0 { ans } else { 2.0 - ans }
}

/// PTW-01 / C-02: q-quantile of a lognormal given its median and log-scale.
/// The median is exp(mu), so the quantile is median * exp(sigma * z_q).
pub fn lognormal_quantile(median: f64, log_scale: f64, q: f64) -> f64 {
    median * (log_scale * phi_inv(q)).exp()
}

/// PTW-04 / C-03: correlated pooling factor, replacing sqrt(n).
///
/// Psi(n, rho) = sqrt(n / (1 + (n-1) rho)). The sqrt(n) law it replaces
/// assumes independent zone demand; the demand model has correlated bursts and
/// the order manager re-pools through substitution and rebalancing, so the
/// uncorrected law overstates the penalty for decentralising. `rho` is a
/// fitted parameter, not a constant.
pub fn psi(n: f64, rho: f64) -> f64 {
    let denom = 1.0 + (n - 1.0) * rho;
    if denom <= 0.0 {
        return f64::INFINITY;
    }
    (n / denom).sqrt()
}

/// PTW-07 / C-09: batch-arrival staffing inflation for a wave of `wave_seconds`.
///
/// Square-root staffing assumes Poisson arrivals, but work reaches pickers in
/// waves, so the variability seen at the node is higher. The corrected form is
/// c_f = rho_f + beta_delta * sqrt(1 + lambda_f * Delta / 2) * sqrt(rho_f);
/// this returns the sqrt(1 + lambda*Delta/2) factor, with Delta in hours to
/// match `lambda_per_hour`. Delta is a tuned parameter traded against routing
/// quality, not a constant.
pub fn batch_arrival_inflation(lambda_per_hour: f64, wave_seconds: f64) -> f64 {
    let delta_hours = wave_seconds / 3600.0;
    (1.0 + lambda_per_hour * delta_hours / 2.0).max(1.0).sqrt()
}

/// PTW-08 / C-07: best total contribution from activating node set `active`,
/// under whole-zone assignment and binding node throughput caps.
///
/// `contribution[node][zone]` is a per-unit rate; a zone's whole demand goes to
/// at most one active node, and a node's assigned demand may not exceed its cap.
pub fn capacitated_coverage(
    active: &[usize],
    zone_demand: &[f64],
    node_capacity: &[f64],
    contribution: &[Vec<f64>],
) -> f64 {
    if active.is_empty() || zone_demand.is_empty() {
        return 0.0;
    }
    // Zones are indivisible, so this is a generalised assignment problem and
    // is NP-hard in general. Branch and bound is exact; the admissible bound
    // is each remaining zone taking its best rate over the active set with
    // capacity ignored. Large layouts should go through the LP-rounding
    // cross-check (PTW-09) rather than lean on this.
    let order: Vec<usize> = {
        let mut z: Vec<usize> = (0..zone_demand.len()).collect();
        z.sort_by(|a, b| zone_demand[*b].partial_cmp(&zone_demand[*a]).unwrap());
        z
    };
    // suffix[i] = optimistic value of zones order[i..]
    let mut suffix = vec![0.0; order.len() + 1];
    for i in (0..order.len()).rev() {
        let z = order[i];
        let best = active
            .iter()
            .map(|&n| contribution[n][z])
            .fold(0.0f64, f64::max);
        suffix[i] = suffix[i + 1] + zone_demand[z] * best.max(0.0);
    }

    struct Ctx<'a> {
        active: &'a [usize],
        order: &'a [usize],
        zone_demand: &'a [f64],
        node_capacity: &'a [f64],
        contribution: &'a [Vec<f64>],
        suffix: &'a [f64],
        load: Vec<f64>,
        best: f64,
    }

    fn search(c: &mut Ctx, i: usize, acc: f64) {
        if acc + c.suffix[i] <= c.best + 1e-12 {
            return; // cannot beat the incumbent
        }
        if i == c.order.len() {
            if acc > c.best {
                c.best = acc;
            }
            return;
        }
        let z = c.order[i];
        let d = c.zone_demand[z];
        for k in 0..c.active.len() {
            let n = c.active[k];
            if c.load[n] + d <= c.node_capacity[n] + 1e-12 {
                c.load[n] += d;
                let gain = d * c.contribution[n][z];
                search(c, i + 1, acc + gain);
                c.load[n] -= d;
            }
        }
        // leaving a zone unserved is always allowed
        search(c, i + 1, acc);
    }

    let mut ctx = Ctx {
        active,
        order: &order,
        zone_demand,
        node_capacity,
        contribution,
        suffix: &suffix,
        load: vec![0.0; node_capacity.len()],
        best: 0.0,
    };
    search(&mut ctx, 0, 0.0);
    ctx.best
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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

    // ---------------- Revision 5 corrections ----------------
    // Expected values are the Check lines of the correction register
    // (docs/paratwin-rev5-epic-prompt.md). They are hand-computed from the
    // stated formulas, never read back out of this implementation.

    /// PTW-01 / C-01. beta* = G-inverse(alpha) and z = Phi-inverse(beta*).
    #[test]
    fn phi_inv_matches_known_quantiles() {
        // C-01 Check: with every basket exactly 3 distinct SKUs and
        // alpha_target = 0.95, beta* = 0.983048 and z = 2.1212.
        assert!((phi_inv(0.983048) - 2.1212).abs() < 1e-4, "got {}", phi_inv(0.983048));
        // the defect this replaces: reading Phi-inverse(alpha) directly
        assert!((phi_inv(0.95) - 1.6449).abs() < 1e-4, "got {}", phi_inv(0.95));
        assert!(phi_inv(0.5).abs() < 1e-9, "median must be 0");
        // symmetry
        assert!((phi_inv(0.1) + phi_inv(0.9)).abs() < 1e-9);
    }

    /// PTW-01 / C-02. Lognormal quantiles used by the corrected radius.
    #[test]
    fn lognormal_quantile_matches_closed_form() {
        // median 9, log-scale 0.35 -> exp(ln 9 + 0.35 * z_q)
        let want = 9.0 * (0.35 * phi_inv(0.95)).exp();
        assert!((lognormal_quantile(9.0, 0.35, 0.95) - want).abs() < 1e-9);
        // the median is returned at q = 0.5 regardless of scale
        assert!((lognormal_quantile(9.0, 0.35, 0.5) - 9.0).abs() < 1e-9);
        // C-02 old-formula radius: v_median * (tau - T_route - t_node_0.95)
        let r_old = (20.0 / 60.0) * (60.0 - 4.0 - lognormal_quantile(9.0, 0.35, 0.95));
        assert!((r_old - 13.332).abs() < 5e-3, "old radius {r_old}");
    }

    /// PTW-04 / C-03. Psi(n,rho) = sqrt(n / (1 + (n-1) rho)) replaces sqrt(n).
    #[test]
    fn psi_matches_correlated_pooling_check() {
        assert!((psi(16.0, 0.0) - 4.0).abs() < 1e-4, "Psi(16,0)");
        assert!((psi(16.0, 1.0) - 1.0).abs() < 1e-4, "Psi(16,1)");
        assert!((psi(9.0, 0.3) - 1.6270).abs() < 1e-4, "Psi(9,0.3)");
        // rho = 0 must reduce to the uncorrected law it replaces
        for n in [1.0, 4.0, 9.0, 25.0] {
            assert!((psi(n, 0.0) - n.sqrt()).abs() < 1e-12, "rho=0 must be sqrt(n)");
        }
    }

    /// PTW-07 / C-09. Inflation factor sqrt(1 + lambda*Delta/2) at Delta = 30 s.
    #[test]
    fn batch_arrival_inflation_matches_check() {
        assert!((batch_arrival_inflation(10.0, 30.0) - 1.0206).abs() < 1e-4);
        assert!((batch_arrival_inflation(60.0, 30.0) - 1.1180).abs() < 1e-4);
        assert!((batch_arrival_inflation(150.0, 30.0) - 1.2748).abs() < 1e-4);
        // a zero-length wave is the Poisson case: no inflation
        assert!((batch_arrival_inflation(60.0, 0.0) - 1.0).abs() < 1e-12);
    }

    /// PTW-08 / C-07. Capacity destroys submodularity, so the greedy
    /// (1-1/e) bound does not transfer. This is the counterexample.
    #[test]
    fn capacitated_coverage_is_not_submodular() {
        let demand = [0.6, 1.0, 0.4];
        let capacity = [0.6, 1.0, 0.4];
        let contribution = vec![
            vec![1.0, 0.6, 0.6],
            vec![1.0, 0.6, 1.0],
            vec![0.6, 1.0, 1.0],
        ];
        let g = |s: &[usize]| capacitated_coverage(s, &demand, &capacity, &contribution);

        assert!((g(&[1]) - 1.0).abs() < 1e-9, "g({{1}}) = {}", g(&[1]));
        assert!((g(&[1, 2]) - 1.0).abs() < 1e-9, "g({{1,2}}) = {}", g(&[1, 2]));
        assert!((g(&[0, 1]) - 1.2).abs() < 1e-9, "g({{0,1}}) = {}", g(&[0, 1]));
        assert!((g(&[0, 1, 2]) - 1.6).abs() < 1e-9, "g({{0,1,2}}) = {}", g(&[0, 1, 2]));

        // the marginal value of node 2 RISES as the set grows: 0 -> 0.4.
        // Under submodularity it could only fall.
        let small = g(&[1, 2]) - g(&[1]);
        let large = g(&[0, 1, 2]) - g(&[0, 1]);
        assert!((small - 0.0).abs() < 1e-9, "marginal in {{1}} = {small}");
        assert!((large - 0.4).abs() < 1e-9, "marginal in {{0,1}} = {large}");
        assert!(large > small, "complementarity: marginal must rise, {small} -> {large}");
    }

    proptest! {
        /// Psi is a pooling *discount*: never worse than full pooling (1),
        /// never better than independence (sqrt n), and falls as rho rises.
        #[test]
        fn psi_is_bounded_and_monotone_in_rho(
            n in 1.0f64..64.0, a in 0.0f64..1.0, b in 0.0f64..1.0
        ) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(psi(n, lo) >= psi(n, hi) - 1e-12, "must not rise with rho");
            prop_assert!(psi(n, hi) >= 1.0 - 1e-9, "never below full pooling");
            prop_assert!(psi(n, lo) <= n.sqrt() + 1e-9, "never above independence");
        }

        /// phi_inv is the inverse of the normal CDF, so it must be strictly
        /// increasing and antisymmetric about p = 0.5.
        #[test]
        fn phi_inv_is_monotone_and_antisymmetric(p in 1e-6f64..0.5) {
            prop_assert!((phi_inv(p) + phi_inv(1.0 - p)).abs() < 1e-6);
            prop_assert!(phi_inv(p) < phi_inv(p + 1e-3));
            prop_assert!(phi_inv(p) < 0.0);
        }

        /// Inflation is 1 at zero load or zero wave, and rises with both.
        #[test]
        fn batch_inflation_rises_with_load_and_wave(lam in 0.0f64..500.0, w in 0.0f64..300.0) {
            prop_assert!(batch_arrival_inflation(lam, w) >= 1.0 - 1e-12);
            prop_assert!(batch_arrival_inflation(lam, w) <= batch_arrival_inflation(lam + 1.0, w) + 1e-12);
            prop_assert!(batch_arrival_inflation(lam, w) <= batch_arrival_inflation(lam, w + 1.0) + 1e-12);
        }
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
