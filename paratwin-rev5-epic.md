# ParaTwin — Revision 5 correction epic

## 1. Epic

| Field | Value |
|---|---|
| **Epic ID** | PTW-EPIC-R5 |
| **Title** | Bring ParaTwin's decision models onto Revision 5 of the ParaExpress technical report |
| **Goal** | Replace the sixteen Revision-4 rules the report now corrects, each with a failing test that pins the corrected value. |
| **Business outcome** | Two corrections move numbers the business acts on, and both currently err the same way: C-01 sizes safety stock on item fill rate when the PX1 promise is measured on the order, and C-03/C-04 overstate the pooling penalty. The network ParaTwin currently recommends is smaller and cheaper than the promise requires. |
| **Scope in** | The sixteen corrections; the tests that prove them; the sequencing between them. |
| **Scope out** | Reliability work Revision 5 also specifies (RTOs, degraded reservation modes, promise-cache staleness, replay targeting, reservation expiry, composed-path latency budgets); document-only corrections; systems outside this repository. |
| **Total story points** | 96 |
| **Story count** | 19 |

All file paths below are relative to the repository root. The simulator lives under `paratwin/`; the report and its companion notes under `deliverables/ParaExpress-Technical-Report/`.

Tests in this codebase are inline `#[cfg(test)] mod tests` blocks at the foot of each module. "Test" fields name the module file and the new test function.

---

## 2. Stories

```
ID:            PTW-01
Title:         Normal and lognormal quantile helpers
Implements:    C-01, C-02
Points:        3 -- pure numerics, no callers to disturb; the only cost is
               getting inverse-CDF accuracy right and proving it.
Files:         paratwin/engine/src/analytics.rs
Current:       No quantile function exists anywhere in the engine. A grep for
               Phi/norm_inv/inverse_cdf/qnorm returns zero hits. The one z
               value in the codebase is the literal 1.65 passed at
               paratwin/engine/src/lib.rs:294.
Required:      Add phi_inv(p) (standard normal inverse CDF) and
               lognormal_quantile(median, log_scale, q). Both are prerequisites
               for the corrected base stock (C-01) and service radius (C-02).
Acceptance:    Given p = 0.983048, When phi_inv(p) is called, Then it returns
               2.1212 +/- 1e-4. Given a lognormal with median 9 and log-scale
               0.35, When the 0.9826 quantile is requested, Then the result
               matches exp(ln(9) + 0.35 * phi_inv(0.9826)) to 1e-9.
Test:          paratwin/engine/src/analytics.rs -- phi_inv_matches_known_quantiles
Depends on:    none
Risk if not done: C-01 and C-02 cannot be computed at all; both stay blocked.
```

```
ID:            PTW-02
Title:         Basket SKU-count model
Implements:    C-01, C-08
Points:        5 -- touches the order struct and the wave feasibility test,
               both hot paths, and every existing DES test must stay green.
Files:         paratwin/engine/src/model.rs,
               paratwin/engine/src/des.rs
Current:       A basket is one uniform scalar. des.rs:148 sets
               `basket_u: rng.gen()` and des.rs:410 tests it with
               `if o.basket_u >= n.spec.assortment_cover { continue; }`.
               There is no notion of distinct SKUs per basket; a grep for
               "sku" returns zero hits.
Required:      Represent the distribution of distinct SKUs per basket, p_j,
               with generating function G(x) = sum_j p_j x^j. Carry the drawn
               SKU count on the order so basket completeness can be evaluated
               per SKU rather than against one scalar.
Acceptance:    Given every basket is exactly 3 distinct SKUs, When G is
               evaluated at x = 0.98, Then G(0.98) = 0.941192 +/- 1e-6, which
               is the order fill rate at item fill rate 0.98.
Test:          paratwin/engine/src/model.rs -- basket_generating_function_matches_check
Depends on:    none
Risk if not done: Order fill rate cannot be distinguished from item fill rate,
               so C-01 and C-08 have nothing to compute against.
```

```
ID:            PTW-03
Title:         Size base stock from order fill rate
Implements:    C-01
Points:        5 -- the sizing line is one expression, but it currently has no
               z, no sigma and no fill-rate target, so the inputs must be
               introduced and threaded through the scenario.
Files:         paratwin/engine/src/des.rs,
               paratwin/engine/src/model.rs
Current:       des.rs:188 sizes stock as days of cover:
               `let base = (share[i] * sc.orders_per_day * spec.base_stock_days).max(4.0);`
               No service level enters the calculation. Scenario carries
               `sla_target` (model.rs:82) but it is never used for stock.
Required:      beta* = G-inverse(alpha_target); base stock uses
               z = phi_inv(beta*). Reading phi_inv(alpha_target) directly is
               the defect being corrected.
Acceptance:    Given every basket is exactly 3 distinct SKUs and
               alpha_target = 0.95, When base stock is sized, Then
               beta* = 0.983048 +/- 1e-6 and z = 2.1212 +/- 1e-4.
               And given beta = 0.98, Then the realised order fill rate is
               0.941192, below the 0.95 promise.
Test:          paratwin/engine/src/des.rs -- base_stock_uses_order_fill_rate
Depends on:    PTW-01, PTW-02
Risk if not done: Every node is under-stocked against a promise measured on
               the order. This is one of the two errors that reach the business.
```

```
ID:            PTW-04
Title:         Correlated pooling law
Implements:    C-03
Points:        2 -- one pure function plus one hard-coded validation row that
               currently asserts the wrong law.
Files:         paratwin/engine/src/analytics.rs,
               paratwin/engine/src/lib.rs
Current:       analytics.rs:137 `pub fn pooled_safety_stock(z, sigma, n) -> f64
               { z * sigma * n.sqrt() }`. lib.rs:294-303 pins it: a validation
               row asserts the 4-node/1-node ratio equals exactly 2.0.
               A grep for "rho" or "correlat" returns zero hits.
Required:      Psi(n, rho) = sqrt(n / (1 + (n-1) rho)) replaces sqrt(n), with
               rho a fitted parameter, not a constant.
Acceptance:    Given Psi as defined, When evaluated, Then Psi(16, 0) = 4.0,
               Psi(16, 1) = 1.0 and Psi(9, 0.3) = 1.6270, each +/- 1e-4.
               And the lib.rs validation row asserts Psi, not sqrt(n).
Test:          paratwin/engine/src/analytics.rs -- psi_matches_correlated_pooling_check
Depends on:    none
Risk if not done: Decentralisation looks more expensive than it is, pushing
               every sizing decision toward too few nodes.
```

```
ID:            PTW-05
Title:         Node count by grid evaluation under correlated pooling
Implements:    C-04
Points:        5 -- replaces a root find with a grid search and changes a
               published validation number, so the math page and its
               assertions move together.
Files:         paratwin/engine/src/analytics.rs,
               paratwin/engine/src/lib.rs
Current:       analytics.rs:53 `cost_curve` computes `f*n + a*n.sqrt() +
               b/n.sqrt()` on the uncorrected law. analytics.rs:59 `n_star`
               bisects `2Fx^3 + ax^2 - b` under a docstring that asserts
               "Unique minimizer ... psi is strictly increasing".
               lib.rs:272-280 asserts n* = 9.06 +/- 0.15.
Required:      C(n) = F*n + a*Psi(n,rho) + b*n^(-1/2). Uniqueness is proved
               only at rho = 0, so find the optimum by grid evaluation over n,
               never by a root find assuming a single crossing.
Acceptance:    Given (F,a,b) = (1.2, 6, 120), When the optimum is found by
               grid, Then n* = 9.071 at rho = 0, 13.153 at rho = 0.3 and
               13.558 at rho = 0.9, each +/- 0.01; and C(n*) = 68.799 at
               rho = 0 +/- 0.01.
Test:          paratwin/engine/src/analytics.rs -- n_star_grid_matches_rho_sweep
Depends on:    PTW-04
Risk if not done: The recommended node count stays too low at every non-zero
               correlation. The second of the two errors that reach the business.
```

```
ID:            PTW-06
Title:         Service radius from joint quantiles
Implements:    C-02
Points:        5 -- the formula is small but it is duplicated at three call
               sites with different ad-hoc slack terms, and all three must
               converge on one function.
Files:         paratwin/engine/src/analytics.rs,
               paratwin/engine/src/lib.rs,
               paratwin/engine/src/des.rs
Current:       analytics.rs:83 `let r = (speed_kmh / 60.0) * (tau_min -
               t_node_min).max(0.0);` -- one quantile of in-node time
               subtracted from the budget, travel deterministic at a mean
               speed. The same shape is open-coded at lib.rs:455-457 and
               des.rs:1074-1075 with hand-tuned constants (-10.0 and -8.0).
Required:      R(e1,e2) = v_(1-e2) * (tau - T_route - t_node_(1-e1)), clamped
               at zero, maximised over e1 + e2 = 1 - alpha. Then
               n_min = ceil(A / (eta * pi * R*^2)).
Acceptance:    Given tau = 60, T_route = 4, alpha = 0.95, in-node time
               lognormal (median 9, log-scale 0.35), effective speed lognormal
               (median 20 km/h, log-scale 0.30), When R* is maximised, Then
               R* = 7.124 km +/- 0.01 at (e1,e2) = (0.0174, 0.0326), against
               13.332 km for the old formula; and at A = 662 km2, eta = 0.65,
               n_min = 7, not 2.
Test:          paratwin/engine/src/analytics.rs -- service_radius_uses_joint_quantiles
Depends on:    PTW-01
Risk if not done: Coverage, auto-layout and the promise-shown metric all
               believe a radius nearly twice the feasible one.
```

```
ID:            PTW-07
Title:         Batch-arrival staffing inflation
Implements:    C-09
Points:        3 -- one formula wrapped around an existing search; the wave
               length is already on the scenario.
Files:         paratwin/engine/src/analytics.rs,
               paratwin/engine/src/model.rs
Current:       analytics.rs:40 `sqrt_staffing(lambda, mu, delta)` walks c
               upward until Erlang-C delay probability falls below delta.
               It assumes Poisson arrivals. Work is released to pickers in
               waves of `sc.wave_seconds` (model.rs:84, default 20.0 at
               lib.rs:155), which the staffing model never sees.
Required:      c_f = rho_f + beta_delta * sqrt(1 + lambda_f * Delta / 2) *
               sqrt(rho_f), with Delta the wave length. Delta becomes a tuned
               parameter traded against routing quality, not a constant.
Acceptance:    Given Delta = 30 s, When the inflation factor
               sqrt(1 + lambda*Delta/2) is evaluated, Then it is 1.0206 at
               10 orders/hour, 1.1180 at 60 and 1.2748 at 150, each +/- 1e-4.
Test:          paratwin/engine/src/analytics.rs -- batch_arrival_inflation_matches_check
Depends on:    none
Risk if not done: Nodes are understaffed in exactly the regime the design
               creates, because waves are what the router deliberately does.
```

```
ID:            PTW-08
Title:         Capacitated greedy and withdrawal of the 63.2% claim
Implements:    C-07
Points:        5 -- the recompute-marginals half already holds; the work is
               adding capacity, proving non-submodularity, and removing the
               claim from six places including three UI pages.
Files:         paratwin/engine/src/analytics.rs,
               paratwin/engine/src/lib.rs,
               paratwin/web/app/decisions/page.tsx,
               paratwin/web/app/math/page.tsx,
               paratwin/web/app/scenarios/page.tsx
Current:       analytics.rs:93 `greedy_coverage` does recompute marginals each
               outer iteration (analytics.rs:104-119), so that half is already
               right -- but it has no capacity argument at all. The guarantee
               is claimed at analytics.rs:90, lib.rs:16-17, lib.rs:445,
               decisions/page.tsx:187 and 217-222 ("63.2% of the optimal
               activation set at every prefix"), math/page.tsx:32 and
               scenarios/page.tsx:257-258.
Required:      Honour binding throughput caps under whole-order assignment.
               Do not claim the (1-1/e) bound in code comments, docstrings,
               reports or UI copy -- capacity destroys submodularity.
Acceptance:    Given nodes 0,1,2 with zone demand (0.6, 1.0, 0.4), throughput
               (0.6, 1.0, 0.4) and unit contribution rows
               [[1.0,0.6,0.6],[1.0,0.6,1.0],[0.6,1.0,1.0]], When coverage is
               evaluated under whole-order assignment, Then g({1}) = 1.0,
               g({1,2}) = 1.0, g({0,1}) = 1.2 and g({0,1,2}) = 1.6, so node
               2's marginal value rises from 0 to 0.4 as the set grows.
Test:          paratwin/engine/src/analytics.rs -- capacitated_coverage_is_not_submodular
Depends on:    none
Risk if not done: A guarantee that does not hold is shown to the reader on
               three screens and relied on by auto-layout.
```

```
ID:            PTW-09
Title:         LP-rounding cross-check as second candidate generator
Implements:    C-07
Points:        5 -- new solver path, but the min-cost-flow machinery in
               routing.rs is a working model to follow.
Files:         paratwin/engine/src/analytics.rs,
               paratwin/engine/src/routing.rs
Current:       Greedy is the only candidate generator. lib.rs:61 and
               lib.rs:457 both call `greedy_coverage` and take its output as
               the layout. Nothing cross-checks it.
Required:      Add an LP-rounding cross-check as a second candidate generator,
               and report both candidates rather than assuming greedy wins.
Acceptance:    Given the C-07 three-node instance, When both generators run,
               Then the LP-rounding candidate attains g >= the greedy
               candidate's 1.6, and the better of the two is the one returned.
Test:          paratwin/engine/src/analytics.rs -- lp_rounding_matches_or_beats_greedy
Depends on:    PTW-08
Risk if not done: With the bound withdrawn and no second generator, layout
               quality has no check of any kind.
```

```
ID:            PTW-10
Title:         Marginal contribution floor on node activation
Implements:    C-06
Points:        5 -- requires a basket-value distribution the Scenario does not
               have; today value is the single scalar `aov_idr`.
Files:         paratwin/engine/src/analytics.rs,
               paratwin/engine/src/lib.rs,
               paratwin/engine/src/model.rs
Current:       analytics.rs:110-118 ranks candidates purely by covered demand
               weight (`gain += w`). lib.rs:446 `rank_store_activations`
               returns `marginal_coverage`. Value never enters: model.rs:87
               carries one `aov_idr` scalar and no distribution.
Required:      Rank and filter by marginal contribution, netting courier fee,
               pick cost and channel commission against margin, with an
               explicit minimum-contribution threshold derived from the
               basket-value distribution.
Acceptance:    Given a candidate whose covered baskets have value below
               courier fee plus pick cost plus commission, When activation is
               ranked, Then the candidate is excluded and cumulative
               contribution after activation is strictly higher than under
               coverage-only ranking on the same instance.
Test:          paratwin/engine/src/analytics.rs -- contribution_floor_rejects_value_destroying_nodes
Depends on:    PTW-08
Risk if not done: Coverage rises while contribution falls, and the decisions
               page recommends the activations that cause it.
```

```
ID:            PTW-11
Title:         Two-stage network sizing
Implements:    C-05
Points:        8 -- no sizing routine exists; node counts are inputs today.
               This is the largest genuinely new component in the epic.
Files:         paratwin/engine/src/lib.rs,
               paratwin/engine/src/analytics.rs
Current:       Nothing computes a node count. `auto_layout` (lib.rs:18) takes
               `n_dark` and `n_store` as given, and `preset` hard-codes them:
               3/0 at lib.rs:196, 2/15 at lib.rs:201, 6/30 at lib.rs:205.
               `network_sweep` (lib.rs:332) varies n from outside.
Required:      Stage 1 activates store nodes (fixed cost approximately zero)
               in greedy marginal-contribution order until the pool is
               exhausted or contribution falls below the floor, giving n_0.
               Stage 2 applies C-04 to the dark-MFC layer only, on residual
               and overflow demand, giving n_mfc_star. Deployed count is
               max(n_mfc_star, n_min - n_0) clamped at zero, with n_min from
               C-02 on the combined tier.
Acceptance:    Given the pilot scenario, When two-stage sizing runs, Then it
               returns (n_0, n_mfc_star, deployed) rather than accepting node
               counts as inputs, and deployed >= n_min - n_0 with n_min taken
               from the corrected C-02 radius (7 on the C-02 check inputs).
Test:          paratwin/engine/src/lib.rs -- two_stage_sizing_respects_sla_floor
Depends on:    PTW-05, PTW-06, PTW-10
Risk if not done: One optimal node count is computed for formats whose fixed
               costs differ by two orders of magnitude.
```

```
ID:            PTW-12
Title:         Assortment bound on revenue share, not basket count
Implements:    C-08
Points:        5 -- needs the SKU model from PTW-02 and a brand-block
               attribute that does not exist yet.
Files:         paratwin/engine/src/model.rs,
               paratwin/engine/src/analytics.rs
Current:       Assortment is one scalar per node, `assortment_cover: f64`
               (model.rs:23), tested at des.rs:410. There are no brand blocks
               (grep "brand" returns zero hits), no basket-value distribution,
               and the completeness bound is not computed anywhere.
Required:      Compute both the count share and the revenue share of baskets
               that cross brand blocks, and use the revenue share in the bound
               and in any reported guarantee.
Acceptance:    Given a basket set where cross-block baskets are 20% by count
               but 35% by revenue, When the bound is computed, Then it uses
               0.35, and a test asserts the two shares are reported separately
               and are not equal.
Test:          paratwin/engine/src/analytics.rs -- assortment_bound_uses_revenue_share
Depends on:    PTW-02
Risk if not done: The guarantee is stated against the wrong measure whenever
               cross-block baskets are worth more than average, which is the
               expected case.
```

```
ID:            PTW-13
Title:         Effective posting latency and node suspension
Implements:    C-10
Points:        5 -- adds a heartbeat event and a feasibility gate to the wave
               loop, which is the hottest path in the DES.
Files:         paratwin/engine/src/des.rs,
               paratwin/engine/src/model.rs
Current:       des.rs:631 posts the ledger a fixed delay after the sale:
               `push!(now + sc.pos_latency_min, Ev::LedgerPost(ni))`, with
               `pos_latency_min` a scenario constant (model.rs:102, default
               5.0). There is no heartbeat concept -- grep returns zero hits --
               and no path removes a node from the feasible set.
Required:      l_eff(t) = max(observed steady-state latency,
               t - last heartbeat timestamp). Size the protection buffer
               against l_eff. When l_eff exceeds the window the buffer can
               cover within shelf quantity, remove the node from the feasible
               set for new orders until a heartbeat and a posting flush arrive.
Acceptance:    Given a store node whose heartbeat stops at t, When t - last
               heartbeat exceeds the buffer's coverage window, Then the node
               is excluded from the candidate list built at des.rs:409-445 and
               readmitted only after both a heartbeat and a posting flush.
Test:          paratwin/engine/src/des.rs -- stale_node_is_suspended_from_feasible_set
Depends on:    none
Risk if not done: An offline store client keeps accepting orders against a
               ledger that stopped moving hours ago.
```

```
ID:            PTW-14
Title:         Independent liveness and latency probe
Implements:    C-12
Points:        3 -- small once PTW-13 exists, but it must be wired to a
               separate signal, which is the whole point.
Files:         paratwin/engine/src/des.rs,
               paratwin/engine/src/model.rs
Current:       The buffer is a static scenario constant, `store_buffer`
               (model.rs:104, default 2.0), applied at des.rs:427-431 and
               des.rs:665-669. It is not tuned from posting history today, so
               the specific circular signal the correction describes does not
               yet exist -- but neither does the independent probe that would
               keep it from being introduced by PTW-13.
Required:      Gate the buffer on an independent liveness and latency probe,
               not on posting history alone.
Acceptance:    Given posting history that appears healthy because postings
               have stopped arriving, When the independent probe reports the
               integration down, Then the buffer grows rather than shrinks.
Test:          paratwin/engine/src/des.rs -- buffer_grows_when_probe_fails_despite_clean_history
Depends on:    PTW-13
Risk if not done: PTW-13's l_eff is derived from the same source as the risk,
               so the buffer shrinks exactly when it should grow.
```

```
ID:            PTW-15
Title:         Explicit reservation and router commitment policy
Implements:    C-13
Points:        5 -- commitment holds by construction today but is nowhere
               stated, and the release paths and regret metric are absent.
Files:         paratwin/engine/src/des.rs,
               paratwin/engine/src/routing.rs
Current:       The wave loop (des.rs:398-509) re-solves over `pending` only;
               an assigned order leaves the pending list at des.rs:505
               (`pending = keep`) and is never revisited. So no committed
               order is currently moved -- but nothing states the rule, no
               path releases an order on outage or expiry, and regret is never
               computed (grep returns zero hits). routing.rs:1-4 claims
               Theorem 5 optimality without qualifying it.
Required:      An order is committed when its reservation is confirmed and the
               node task is pushed; a committed order is never reassigned by a
               later solve, and is released only by cancellation, node outage
               or reservation expiry. Claim optimality per wave over
               uncommitted orders only. Compute realised regret against the
               clairvoyant assignment on completed traces and report it.
Acceptance:    Given a committed order and a later solve that would prefer a
               different node, When the wave re-solves, Then the order stays
               at its committed node; and given a node outage, Then it is
               released. Realised regret appears in the run output.
Test:          paratwin/engine/src/des.rs -- committed_orders_survive_resolve
Depends on:    none
Risk if not done: The optimality claim is unqualified and there is no defined
               behaviour when a node fails mid-pick.
```

```
ID:            PTW-16
Title:         Returns and re-attempts as first-class events
Implements:    C-15
Points:        8 -- touches the event set, the outcome state machine, the
               ledger, the economics identity and the calibration contracts.
Files:         paratwin/engine/src/des.rs,
               paratwin/engine/src/model.rs
Current:       `enum Ev` (des.rs:66-78) has no return or re-attempt event.
               `enum Outcome` (des.rs:24-31) ends at Oversell -- there is no
               Returned state. `KpiSummary` (model.rs:149-191) has no return
               cost line, and the contribution identity at des.rs:967 nets
               only courier, labor, rent, store fees and mid-mile.
Required:      Return and re-attempt events in the event set; return-to-stock
               as a normal ledger transaction behind a quality gate; a
               per-order return-handling cost in node economics; and a returns
               feed in the calibration contracts.
Acceptance:    Given a delivery that fails, When the return event fires, Then
               the order reaches a Returned terminal state, stock is credited
               back through the quality gate, and the return-handling cost
               appears in contribution. The order-conservation test at
               lib.rs:545 is extended to include returns and still balances.
Test:          paratwin/engine/src/des.rs -- returns_conserve_orders_and_stock
Depends on:    none
Risk if not done: The twin cannot see a cost it exists to optimise against,
               and calibration has no returns signal.
```

```
ID:            PTW-17
Title:         Lateral replenishment between nodes
Implements:    C-16
Points:        8 -- a new transfer path priced by the router, plus the
               trigger rule, on top of a replenishment model that has only
               one restock mechanism today.
Files:         paratwin/engine/src/des.rs,
               paratwin/engine/src/vrp.rs
Current:       `Ev::Replenish` (des.rs:593-624) is the only path that restores
               stock, on a `midmile.period_days` cycle. A node that runs dry
               fails the ATP check at des.rs:307 and the order is marked
               Oversell immediately. Grep for "transfer", "lateral" and
               "rebalanc" returns zero hits.
Required:      A lateral transfer path between fulfilment nodes, priced by the
               router against the cost of downgrading the order, executed
               under C-11, and triggered when expected downgrade cost over the
               remaining review period exceeds transfer cost.
Acceptance:    Given a node at zero stock with a donor node inside transfer
               range, When expected downgrade cost over the remaining review
               period exceeds transfer cost, Then a lateral transfer is issued
               and the order is served rather than downgraded.
Test:          paratwin/engine/src/des.rs -- lateral_transfer_fires_when_cheaper_than_downgrade
Depends on:    PTW-16
Risk if not done: A stocked-out node waits for the next wave while an
               equivalent capability serves external orders.
```

```
ID:            PTW-18
Title:         Cross-partition transfer protocol
Implements:    C-11
Points:        3 -- small in this codebase because the ledger is a single f64
               per node; the work is the protocol and its invariant, not
               distributed plumbing.
Files:         paratwin/engine/src/des.rs
Current:       There is no transfer to protect. The ledger is a plain field
               (`ledger: f64`, des.rs:56), decremented at des.rs:498 and reset
               wholesale at des.rs:615. There are no partitions and no
               first-class reservations; grep for "two-phase", "saga" and
               "reservation" returns zero hits.
Required:      A cross-node stock move executes as a two-phase commit across
               both partitions, or as a compensating saga in which the source
               decrement commits first and reservations against the source are
               frozen for the duration. No transfer may proceed while an
               uncommitted reservation exists on the affected node-SKU pair.
Acceptance:    Given an uncommitted reservation on a node-SKU pair, When a
               transfer touching that pair is attempted, Then it is refused;
               and given a transfer in flight, Then new reservations against
               the source are frozen until it commits or compensates.
Test:          paratwin/engine/src/des.rs -- transfer_refused_while_reservation_uncommitted
Depends on:    PTW-15, PTW-17
Risk if not done: Lateral transfers and substitution sourcing can double-spend
               the same unit across partitions.
```

```
ID:            PTW-19
Title:         Shadow scoring against a held-out naive policy
Implements:    C-14
Points:        8 -- a second policy, an off-policy estimator and a
               certification gate, none of which exist.
Files:         paratwin/engine/src/lib.rs,
               paratwin/engine/src/des.rs
Current:       `validate_math()` (lib.rs:231-318) checks environment laws only
               -- Erlang-C, n*, the asset-light limit, pooling, staffing
               sublinearity. Nothing scores the decision model. Grep for
               "shadow", "holdout" and "naive" returns zero hits. Twin and
               production share the routing code, so there is no independent
               verification of the routing cost model.
Required:      Score a fixed fraction of waves in shadow against a held-out
               naive policy (nearest feasible node), estimate the difference
               off-policy, and fail certification when the decision model
               cannot beat the naive policy on live traffic.
Acceptance:    Given a run with shadow scoring enabled, When the decision
               model's estimated advantage over nearest-feasible-node is not
               positive, Then certification fails and the run is reported as
               uncertified. A `validate_math()` row covers the policy check
               alongside the environment rows.
Test:          paratwin/engine/src/lib.rs -- certification_fails_when_policy_loses_to_naive
Depends on:    PTW-15
Risk if not done: Trace replay validates the environment model and nothing
               validates the decision model, which is what the twin sells.
```

---

## 3. Traceability

| Correction | Stories | Status |
|---|---|---|
| C-01 Order fill rate | PTW-01, PTW-02, PTW-03 | mapped |
| C-02 Service radius | PTW-01, PTW-06 | mapped |
| C-03 Correlated pooling | PTW-04 | mapped |
| C-04 Node count | PTW-05 | mapped |
| C-05 Two-stage sizing | PTW-11 | mapped — no sizing routine exists today; node counts are inputs at `lib.rs:196/201/205`, so the story builds the component rather than correcting one |
| C-06 Contribution floor | PTW-10 | mapped |
| C-07 Capacitated rollout | PTW-08, PTW-09 | mapped |
| C-08 Assortment revenue bound | PTW-12 | mapped — the bound itself is not computed anywhere; assortment is one scalar at `model.rs:23`. The story adds the computation and the revenue-share measure together |
| C-09 Batch-arrival staffing | PTW-07 | mapped |
| C-10 Effective posting latency | PTW-13 | mapped |
| C-11 Cross-partition transfer | PTW-18 | mapped — no transfers exist yet, so the protocol lands with PTW-17's transfer path |
| C-12 Independent buffer signal | PTW-14 | mapped — the circular signal does not exist yet (`store_buffer` is static at `model.rs:104`); the story prevents PTW-13 from introducing it |
| C-13 Router commitment policy | PTW-15 | mapped — commitment holds by construction at `des.rs:505` but is unstated, and release paths and regret are absent |
| C-14 Independent policy check | PTW-19 | mapped |
| C-15 Returns | PTW-16 | mapped |
| C-16 Lateral replenishment | PTW-17 | mapped |

All sixteen appear exactly once. None is unlocated: every correction has an identified home in files I opened.

---

## 4. Sequence

**Wave 1 — no dependencies. Foundations and self-contained formulas. 18 points.**
PTW-01 (3), PTW-02 (5), PTW-04 (2), PTW-07 (3), PTW-08 (5)
Rationale: pure functions and one docs-and-capacity story. Nothing here waits on anything, and PTW-01/PTW-02/PTW-04 unblock the widest set downstream.

**Wave 2 — the two business-moving corrections land. 20 points.**
PTW-03 (5), PTW-05 (5), PTW-06 (5), PTW-09 (5)
Rationale: C-01 and C-04, the two that currently under-build the network, complete here. Do this wave before anyone quotes a node count again.

**Wave 3 — decision quality and independent signals. 21 points.**
PTW-10 (5), PTW-12 (5), PTW-13 (5), PTW-15 (5), (PTW-14 waits on PTW-13 within the wave)
Rationale: contribution-aware activation and the ATP safety gate. PTW-15 is independent and can start any time; it sits here because PTW-18 and PTW-19 both need it.

**Wave 4 — new mechanics. 19 points.**
PTW-11 (8), PTW-14 (3), PTW-16 (8)
Rationale: two-stage sizing needs C-02/C-04/C-06 done. Returns is independent and large, so it starts as early as capacity allows.

**Wave 5 — transfers and certification. 19 points.**
PTW-17 (8), PTW-18 (3), PTW-19 (8)
Rationale: lateral replenishment needs the returns ledger transaction shape; the transfer protocol needs both a transfer and a reservation; certification needs commitment.

---

## 5. Assumptions

- The inline `#[cfg(test)] mod tests` blocks are the whole test suite; there is no separate `tests/` directory in the workspace.
- `rho` in C-03/C-04 is fitted outside this repository and arrives as a scenario parameter; no fitting procedure is in scope here.
- The basket-value and SKU-count distributions in C-01, C-06 and C-08 will be supplied as scenario inputs; the repository has no historical basket data.
- `T_route` in C-02 maps to the in-node-to-road handover, closest to the existing `HANDOVER_MIN` constant (`des.rs:18`), but the report may define it more broadly.
- "Node throughput cap" in C-07 maps to the existing `pickers * QUEUE_CAP_FACTOR` wave cap (`des.rs:481-487`); if Revision 5 means a daily cap, PTW-08 grows.
- The Rev-5 PDF in `deliverables/` was not read for this pass; the correction register in `docs/paratwin-rev5-epic-prompt.md` was treated as the specification, per that prompt's own instruction.
- Point values assume the existing DES tests must stay green; several of them (`lib.rs:476-638`) pin current behaviour and will need updating rather than deleting.

---

## 6. What would invalidate this plan

- If `rho` is not a scenario input but must be estimated from simulated traces, PTW-04 and PTW-05 both grow and a fitting story appears ahead of them.
- If the SKU-count distribution must be per-node rather than global, PTW-02 grows past 8 and splits.
- If Revision 5's node throughput cap is a daily capacity rather than the per-wave cap already in the code, PTW-08's acceptance instance no longer matches the engine's units.
- If returns must reconcile against a real returns feed rather than a simulated event, PTW-16 splits into event modelling and contract work.
- If the desktop shell or the web pages are expected to expose every new parameter, each of PTW-03, PTW-05, PTW-06 and PTW-13 gains UI work not counted here.
- If the twin must stay numerically comparable to Revision-4 runs, PTW-03 and PTW-05 need a compatibility mode, which is not in any estimate.

---

## 7. Questions

- Is `rho` supplied per scenario, or must ParaTwin fit it? — blocks PTW-04, PTW-05.
- Where does the distinct-SKU-per-basket distribution come from? — blocks PTW-02, and therefore PTW-03 and PTW-12.
- What is the minimum contribution threshold, and who owns it? — blocks PTW-10, and therefore PTW-11.
- Does the C-07 throughput cap mean the per-wave cap already in the code, or a daily one? — blocks PTW-08.
- What fraction of waves should be shadow-scored, and what advantage margin fails certification? — blocks PTW-19.
- Should the Revision-4 numbers stay reproducible for comparison? — blocks PTW-03, PTW-05.
- Which reliability items from Revision 5 (RTOs, degraded reservation modes, promise-cache staleness, replay targeting, reservation expiry, composed-path latency budgets) are deferred to a later epic rather than dropped? — blocks nothing, but the open-items register expects an answer.
```

