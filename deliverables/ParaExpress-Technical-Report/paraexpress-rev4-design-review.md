# ParaExpress: Understanding and Council Review

Source: *ParaExpress: A Mathematically Grounded Hyperlocal Fulfillment Network and Digital Twin Architecture for Instant Beauty Commerce in Indonesia*, ParaTech Platform R&D Technical Report, Vol. 1 No. 1, August 2026.

---

# Part 1 — What ParaExpress is

## What the report is and what it is asking for

It is an internal R&D technical report from ParagonCorp's ParaTech Platform division that specifies a new fulfillment network and the software to run it. It is not a research paper seeking publication and not a business case seeking a number; it is a design specification with proofs attached, asking for approval of a 30-month phased build. It bundles four documents that normally travel separately: a competitor benchmark (Section II), a formal optimization model with ten theorems and propositions (Sections III–IV), a systems architecture with interface contracts and non-functional requirements (Sections V–VIII), and a rollout plan with gates and unit economics (Sections IX–X).

## The core thesis

Whoever delivers fastest owns the consumer relationship, and eventually the shelf. ParagonCorp cannot out-build Shopee's parcel network, so instead it redefines the fulfillment layer for one category it already dominates. It superimposes a hyperlocal tier of small fulfillment nodes on its existing distribution backbone, built first from consignment stores it already stocks, so that any channel order reaches the consumer in under 60 minutes at near-zero incremental fixed cost.

## The mechanism, end to end

A consumer taps "Instant" on Shopee. Checkout shows a promise served from the ADR cache, which knows whether a feasible node exists for that zone. The order webhook hits MagPie+, which normalizes it, explodes any bundle into component SKUs, and attaches promise class PX1. The Order Routing Engine collects orders into a 5–30 second wave, filters to nodes that can cover the zone within the promise and hold every SKU in available-to-promise inventory, then solves a min-cost assignment and picks node f*. The Virtual Inventory Service atomically reserves the units. A task lands on a tablet at a dark MFC or in the Exagon PX app at a consignment store. Staff pick, pack, and QR-seal, while a Grab rider booked at pick start is already arriving. A handover scan closes custody. The rider delivers, MagPie+ posts back to the marketplace, and every event lands in the lakehouse where ParaTwin uses it to recalibrate. If no node clears the PX1 filters, the order is re-quoted as PX2 (4-hour batched slot) or falls back to PX3, the existing next-day parcel flow from Tegal.

## The mathematical spine

Each result buys one design decision.

- **Theorem 1** (network design is NP-hard, by Set Cover reduction): you cannot solve this exactly at national scale, so pair approximation algorithms with a simulation twin instead of trusting one big MILP. This is the justification for ParaTwin existing at all.
- **Theorem 2** (coverage is monotone submodular, greedy gets 1−1/e ≈ 63.2%, and that is the best any polynomial algorithm can do): open nodes in greedy marginal-coverage order.
- **Corollary 1** (consignment stores dominate as first waves, because their fixed cost is roughly zero): the asset-light thesis, stated as a theorem.
- **Lemma 1** (last-mile distance scales as √(A/n)) and **Proposition 1** (safety stock inflates by √n when you split demand across nodes): the two opposing forces in node count.
- **Theorem 3** (total cost is unimodal in n, with a unique optimum n\* that rises with volume and falls with volatility): how many nodes a metro should have.
- **Proposition 2** (a hard floor n_min = ⌈A/(ηπR²)⌉ set by the feasible service radius): why investing in pick speed reduces the required node count quadratically. The network is sized at max(n_min, n\*).
- **Theorem 4** (forward assortment is exactly greedy under uniform slots via a matroid, an FPTAS under general slots, and (1−1/e) with basket-completeness): the 400–600 SKU per-node assortment rule.
- **Theorem 5** (the routing LP is integral and solvable by successive shortest paths): exact optimal routing in milliseconds, so no heuristic sits on the critical path.
- **Proposition 3** (square-root safety staffing, from Halfin–Whitt): staffing grows sublinearly, favouring fewer busier nodes.
- **Proposition 4** (safe available-to-promise under concurrent channels, via one linearizable ledger plus a protection buffer): the consignment-store oversell problem becomes a tunable buffer.

## What is genuinely new

The NDC, the Tegal B2C DC, the regional B2B DCs, MagPie, Heron, Exagon and ShipGo all exist. New: the Tier 2 fulfillment layer itself (store nodes, dark MFCs, flagship MFCs), a cross-dock function bolted onto existing DCs, and three platform services — the Order Routing Engine, the Virtual Inventory and Distributed Order Management service, and ParaTwin. The single most distinctive architectural claim is that the twin and production share one decision codebase, so a simulated design is executable and an executed design is replayable.

## Load-bearing assumptions

1. Shopee and TikTok Shop will let ParagonCorp self-fulfill instant orders from its own nodes.
2. Grab and GoSend on-demand capacity is available and priced at the assumed per-order rates.
3. Consignment partners will accept reserved stock, a locked shelf, a pick app, a pick SLA, and 5-minute POS posting, in exchange for a per-order fee.
4. Indonesian beauty consumers convert on a 60-minute promise at a rate that fills 40–80 orders per node per day.
5. Beauty's AOV and value density clear break-even at a fraction of grocery's required order density.
6. Zone demand is independent enough for the pooling mathematics, and metro density is uniform enough for the closed-form node count.
7. ParaTwin can be calibrated to within 10% KPI deviation against real traces.

Assumptions 1, 3 and 4 are the ones with no evidence in the document.

---

# Part 2 — Council review

Forty-one findings survived the challenge pass. Findings are ordered Critical, Major, Minor within each lens.

## Lens 1 — Optimization and mathematics

```
F-01 | Math | Critical | Confidence: High
Anchor: Proposition 4; Section VIII-E; Section VIII-J
Issue: Proposition 4 guarantees no oversell for a single node and SKU under atomic
compare-and-decrement on one ledger partition. Section VIII-E gives the DOM
cross-node rebalancing, safety-stock substitution and split-shipment rules, all of
which move or commit stock across two partitions. Section VIII-J states the ledger
is single-writer-per-node-partition "without cross-partition coordination." A
rebalance is by definition cross-partition, so the proposition's safety argument
does not cover the operations the DOM is specified to perform.
Failure scenario: A node-down rebalance moves 40 units of a hero SKU from node A to
node B. A concurrent PX1 order reserves against A's pre-transfer ATP. The order is
accepted, the picker finds an empty shelf, and the 60-minute promise fails after
the consumer has paid.
Fix: Extend Proposition 4 to a two-phase reservation across partitions, or forbid
rebalancing while any reservation is open on the affected SKU and state that
constraint in the DOM specification.
```

```
F-02 | Math | Critical | Confidence: High
Anchor: Proposition 2; equations (9) and (11); Section IV-D
Issue: Equation (9) requires the alpha-quantile of the sum of queue, pick, handover
and travel time. Proposition 2 instead subtracts a single in-node alpha-quantile
t_node from tau and treats the remaining travel time as deterministic, giving
R = v(tau − T_route − t_node)+. Travel is not deterministic; Section IX-C fits
travel-time distributions by quantile regression from rider GPS. Subtracting one
alpha-quantile from a budget does not produce an alpha-quantile of the sum.
Failure scenario: The true 95th-percentile radius is smaller than R, so n_min from
equation (11) is understated. A metro sized on the stated formula opens too few
nodes and misses the 95% attainment target on exactly the tail days that matter.
Fix: Define R from the joint quantile of in-node plus travel time, computed
numerically in ParaTwin, and report the gap against the closed form.
```

```
F-03 | Math | Major | Confidence: High
Anchor: Proposition 1 proof; Theorem 3; Section VIII-E; Section IX-A
Issue: Proposition 1's square-root penalty assumes independent demand splitting.
Two mechanisms in the report break that. Cross-node substitution and rebalancing
(Section VIII-E) partially re-pool inventory. Hawkes-burst demand from live
commerce (Section IX-A) is positively correlated across zones. Both make the true
decentralization penalty smaller than sqrt(n), so the coefficient a = h·z_beta·sigma
in Theorem 3 is overstated and n* is biased low.
Failure scenario: Theorem 3 recommends 9 nodes for a metro where the correct answer
under correlated demand and partial pooling is 12 to 15. The network is
under-built, and the shortfall shows as PX1 downgrade rate rather than as a visible
capacity error.
Fix: Estimate the pooling coefficient empirically in ParaTwin instead of assuming
sqrt(n), and report n* as a range under the fitted correlation.
```

```
F-04 | Math | Major | Confidence: High
Anchor: Theorem 2; Corollary 1; equation (5); Table III
Issue: Theorem 2 states the submodularity and (1−1/e) result explicitly for the
uncapacitated case, mu_f = infinity. The deployed network is throughput-capped:
constraint (5) enforces mu_f, and Table III gives 5–10, 20–60 and 60–150 orders per
hour for the three node formats. Coverage under capacity is not submodular in
general, and greedy loses the guarantee. Corollary 1 then relies on "Because g is
monotone" to justify the rollout order in the real network.
Failure scenario: Greedy opens a high-coverage store node in a dense catchment that
saturates at 8 orders per hour, blocking demand a slightly-lower-coverage but
higher-throughput dark MFC would have served. Rollout order is wrong from wave two
onward and nothing in the gates detects it.
Fix: State the guarantee under capacity, or re-run the rollout ordering in ParaTwin
against the capacitated objective and treat greedy as a heuristic seed.
```

```
F-05 | Math | Major | Confidence: High
Anchor: Proposition 3; equation (12); Remark 3; Section VI-C
Issue: Proposition 3 models the node as M/M/c and applies Halfin-Whitt square-root
staffing. Remark 3 batches orders into waves of 5 to 30 seconds, and Section VI-C
describes wave-based operation throughout. Batched release makes arrivals to the
picker pool a batch process, not Poisson. Batch arrivals produce materially higher
delay variance than M/M/c at equal load, and equation (12) targets a delay
probability in the tail.
Failure scenario: A node staffed to the formula holds mean throughput but breaches
the queue-delay target during wave peaks, which is precisely where the 95% PX1
attainment is won or lost.
Fix: Restate the staffing result for batch arrivals, or state the wave size at
which the M/M/c approximation is defensible and bound the error.
```

```
F-06 | Math | Major | Confidence: High
Anchor: Theorem 4(iii) and its proof sketch
Issue: The basket-completeness result degrades the (1−1/e) guarantee "by at most the
factor (1−epsilon)" where epsilon is the probability a basket crosses brand blocks.
The proof sketch bounds the probability mass of cross-block baskets, not their
revenue. A cross-brand basket in beauty, such as a Wardah base with a Make Over
lip, is plausibly larger than the average basket, so epsilon of baskets can carry
materially more than epsilon of revenue. "Empirically high in beauty" carries no
citation and no MagPie figure.
Failure scenario: Single-brand share is 85% of baskets but 70% of revenue. The
assortment is optimized against a guarantee that is 15 points weaker than stated,
and the highest-value baskets are the ones that split.
Fix: Bound the revenue mass, not the count, and publish the measured single-brand
basket share from MagPie order history.
```

```
F-07 | Math | Major | Confidence: Medium
Anchor: Corollary 1; Table IV; Section VI-D
Issue: Corollary 1 concludes that "any store with positive marginal coverage
improves profit" because fixed cost is near zero. Objective (2) nets variable
fulfillment cost against margin, so the conclusion requires the covered demand to
carry positive contribution, not merely positive coverage. Table IV states one AOV
(IDR 220,000) with no distribution, and Section II-E cites a beauty basket range of
IDR 150,000-350,000. Opening on mean economics over-opens against the left tail.
Failure scenario: A store node in a low-AOV catchment covers real demand at a
contribution that does not clear the rider fee plus pick cost plus marketplace
commission. Greedy keeps opening it because coverage is positive.
Fix: State the AOV floor below which marginal coverage is unprofitable, and add it
as a filter in the greedy expansion.
```

```
F-08 | Math | Major | Confidence: High
Anchor: Theorem 5; Section III-E; Section VIII-F
Issue: Theorem 5 proves offline optimality for a fixed wave. Section III-E says P3
is "re-solved every few seconds (or on each order arrival)," which makes the real
problem online with commitment. The report never states whether a re-solve may
reassign an order whose pick has already started. If it may, offline optimality is
not the relevant benchmark and no competitive ratio is given. If it may not, the
optimality claim is local to a single wave and the sequence of waves has no stated
guarantee.
Failure scenario: A late high-value order triggers a re-solve that reassigns an
in-flight order, wasting the pick and breaching its clock, or the router locks
early commitments and drifts far from the daily optimum with no bound.
Fix: State the commitment policy explicitly, and give the guarantee that applies to
the sequence of waves rather than to one wave.
```

```
F-09 | Math | Major | Confidence: High
Anchor: Theorem 3; Table III; principles P-1 and P-2
Issue: Theorem 3 derives a single optimal node count from a homogeneous per-node
fixed cost F. The architecture is deliberately heterogeneous: Table III gives store
nodes a fixed cost of approximately zero, dark MFCs IDR 35-60M per month, and
flagship MFCs more. P-2 sequences stores first and MFCs into the gaps. Under mixed
F the correct object is a marginal-cost ordering, not one n*. The report never
states which F enters the closed form or how max(n_min, n*) is evaluated across
formats.
Failure scenario: A planner reads n* = 9 for a metro and opens nine mixed nodes,
when the correct answer is twenty store nodes plus two MFCs, or four MFCs alone.
The number is used as a target and it is not one.
Fix: State that n* applies to the MFC layer conditional on the store layer already
placed, and give the two-stage procedure explicitly.
```

```
F-10 | Math | Major | Confidence: High
Anchor: Assumption 1; Lemma 1; equation (11); Section X
Issue: Assumption 1 requires uniform demand density, identical nodes and balanced
catchments. The deployment contradicts all three: Table III has three node formats
with 5-10x throughput spread, and Phase 1 opens 15-25 store nodes with 2 dark MFCs
and 1 flagship in one cluster. Jakarta beauty demand is not uniform. The packing
efficiency eta is introduced in equation (11) as the escape hatch and is never
given a value, a measurement method, or a range.
Failure scenario: A planner applies n_min with eta = 1 implicitly and under-sizes
the network by the share of area that is water, toll road, or non-residential.
Fix: Estimate eta per metro from the road network and publish it alongside n_min,
and state the error bound of the uniform approximation against ParaTwin.
```

```
F-11 | Math | Minor | Confidence: High
Anchor: equations (7), (10), (12); Definition 4; Section III-C
Issue: Notation is not fully declared. The holding cost h appears in a = h·z_beta·sigma
in Theorem 3 and is never defined in the text. beta is the in-node fill-rate target
in (7) and reappears as beta_delta in (12) through the Halfin-Whitt equation without
a note that these are different objects. theta is the order-loss probability in
Section III-C while Theta is the twin's calibration parameter set in Definition 4.
Failure scenario: A reader implementing equation (10) picks the wrong holding-cost
basis and the node-count curve shifts without anyone noticing.
Fix: Add a notation table and rename the colliding symbols.
```

## Lens 2 — Fulfillment and inventory operations

```
F-12 | Operations | Critical | Confidence: High
Anchor: equation (7); Theorem 4; Table II; Table XIV
Issue: Equation (7) sets base stock from a per-SKU fill-rate target beta. Table II
promises PX1 at 95% attainment on the order. A PX1 order fails if any SKU in the
basket is unavailable, so order-level availability is roughly beta raised to the
basket size. At beta = 0.98 and a three-item basket that is 94.1%, already below
the promise before any queue, pick or travel variance is counted. Theorem 4's
basket-completeness objective maximizes revenue, not availability, so it does not
close the gap. Table XIV tracks split-shipment rate but sets no basket-fill target.
Failure scenario: Every node hits its 98% per-SKU fill target, every operational
metric is green, and PX1 attainment sits below 95% for structural reasons no
dashboard shows.
Fix: Set beta from a basket-level target given the observed basket-size
distribution, and add basket fill rate to the KPI vector in Table XIV.
```

```
F-13 | Operations | Major | Confidence: High
Anchor: Table VI; Table IV; Table XIII
Issue: Returns and failed deliveries get one line in Table VI: re-attempt policy and
node return-to-stock with a QC gate. There is no node-level operating model. A
returned unit at a store node with 5 square metres and 150-300 SKUs needs QC,
restow, a ledger correction and a consignment settlement adjustment. Table IV's
node P&L has no return-handling cost line and Table XIII has no returns feed into
ParaTwin calibration, so the twin cannot see the cost either.
Failure scenario: NDR runs at 8% in a pilot cluster. Store staff have no
return-handling routine, returned stock sits unposted, the ATP ledger drifts, and
the buffer policy of Proposition 4 is compensating for an error it was not designed
to see.
Fix: Specify the node-level returns routine, add a cost line to Table IV, and add a
returns feed to Table XIII.
```

```
F-14 | Operations | Major | Confidence: High
Anchor: equation (7); Section VI-B; Section VII-A
Issue: Replenishment is a daily or every-other-day wave from Tier 1, so equation
(7) sizes base stock over T_r + L_f. Between waves the only stated recovery from a
stockout is downgrade to PX2/PX3 or DOM substitution. Section VII-A describes an
intra-day emergency replenishment capability for nearby B2B doors as a revenue
opportunity, but the network never uses that same capability for its own nodes.
Failure scenario: A TikTok live event clears a hero SKU at a node by 11:00. The node
is dark on that SKU for 22 hours while a sister node 3 km away holds 60 units, and
the design's answer is to downgrade the promise.
Fix: Specify an intra-day lateral transfer path between Tier 2 nodes with a cost and
a trigger rule, and let the ORE price it against downgrade.
```

```
F-15 | Operations | Major | Confidence: High
Anchor: Table III; Section VIII-C
Issue: The store node is specified at 5-10 orders per hour with a 7-minute pick
target, staffed by existing store staff at zero incremental fixed cost. Ten orders
per hour at 7 minutes is 70 minutes of picking work per hour from a person who is
simultaneously serving walk-in retail. The throughput figure and the staffing model
cannot both hold. Section VIII-C's accuracy scoring penalizes the store for the
resulting failures without changing the capacity arithmetic.
Failure scenario: A store node is quoted at its stated throughput during a Saturday
peak, picks slip past 7 minutes, PX1 clocks breach, the accuracy score drops, the
ORE quotes the store less, and the partner concludes the program does not work.
Fix: Derive store-node throughput from a measured share of staff time, state the
walk-in traffic assumption behind it, and cap ORE allocation at that number.
```

```
F-16 | Operations | Major | Confidence: High
Anchor: Table IV; Section VIII-B; Section X
Issue: Shrinkage plus cycle-count allowance is set at 0.3% of sales in the dark-MFC
economics and no figure is given for store nodes, where the exposure is structurally
different: ParagonCorp-owned high-value small-format stock, on a locked shelf, in a
third party's premises, picked by that third party's staff. The controls are blind
cycle counting by ABC class and accuracy scoring. No shrink gate appears in the
Section X phase criteria.
Failure scenario: Store-node shrink runs at 1.5% of sales. At the stated
contribution it consumes a large share of node margin, and it surfaces first in a
monthly C-class count, by which point 15-25 nodes are live.
Fix: Set a separate shrink allowance for store nodes, and add a shrink threshold to
the Phase 1 gate.
```

## Lens 3 — Platform and systems engineering

```
F-17 | Systems | Critical | Confidence: High
Anchor: Table XI; Section VIII-J; Figure 6; Table VI
Issue: The VIS is on every order's critical path at 99.99% availability and 10 ms
p99, and the stated durability mechanism is synchronous replication with RPO 0 on a
single-writer-per-node partition. Synchronous replication protects durability, not
availability: a partition whose writer is down blocks every order for that node until
failover. RTO is given for MagPie+ (15 minutes) and not for the VIS. A 99.99% budget
is about 4.4 minutes per month, so one 15-minute failover exhausts three months of
budget. Figure 6 has no path that completes an order without the VIS, even though a
promise waterfall exists for every other failure.
Failure scenario: A VIS partition failover during a Friday evening peak stalls every
PX1 order for the affected nodes. Orders neither reserve nor gracefully downgrade;
they hang until the checkout times out.
Fix: State the VIS RTO, and specify the degraded mode: quote PX2 or PX3 when the
ledger is unreachable, and reconcile on recovery.
```

```
F-18 | Systems | Critical | Confidence: High
Anchor: Proposition 4; Section VIII-C; Section VIII-J; Table X
Issue: Proposition 4's safety condition requires POS sales to post within latency
ell, and Section VIII-J allows edge clients including the Exagon PX app to run
store-and-forward offline for at least 4 hours. A POS sale is a decrement the server
has not authorized, which is exactly the event the proposition needs inside ell. The
claim that offline replay is "conflict-free" because each partition has a single
writer holds for ordering, not for correctness of the ATP the ledger published while
the store was dark. For an offline store, ell is 4 hours, not the 5 minutes assumed.
Failure scenario: A store's connectivity drops at 14:00. It sells 6 units of a hero
SKU across the counter. The VIS still shows those units available, accepts three PX1
orders against them, and the oversell is discovered at pick.
Fix: Suspend ORE allocation to a node whose POS feed has been silent beyond ell, or
size the buffer for the full offline window rather than for the observed median.
```

```
F-19 | Systems | Critical | Confidence: Medium
Anchor: Proposition 4; Section VIII-C; Section VIII-B; Table XIII
Issue: The protection buffer that makes consignment stores safe is auto-tuned from
each store's own posting-latency history. The control loop and the risk share a
signal source. When a store's POS integration degrades, the observed history lags
the true latency, so the buffer shrinks exactly when it should grow. The only stated
independent detection is cycle-count variance, which runs daily for A-class and
monthly for C-class.
Failure scenario: A partner switches POS vendors and posting silently moves from 3
minutes to 40. The buffer, fitted on months of 3-minute history, does not react.
Oversells accumulate for weeks until a C-class count exposes the drift.
Fix: Add an independent liveness and latency probe on the POS feed, and gate the
buffer on the probe rather than on posting history alone.
```

```
F-20 | Systems | Major | Confidence: Medium
Anchor: Section VIII-F; Section IX-A; Section IX-C; Figure 7
Issue: The twin and production share one decision codebase, presented throughout as
an unqualified strength that eliminates model drift. It also eliminates independent
verification of the decision logic. Trace-driven replay and holdout forecasting
certify the environment model against actuals; neither tests whether the routing cost
model is right, because the twin makes the same decision and then scores it. This
matters most for counterfactual evaluation, which is what the optimizer bank is for:
candidate designs that were never run cannot be checked against any observed outcome.
Failure scenario: A sign error or a mis-scaled congestion shadow price in c_of makes
the ORE systematically prefer nearer nodes with worse courier availability. Every
simulated design inherits the same preference, and certification passes.
Fix: Name the blind spot in Section IX, and add an independent policy check, such as
scoring live decisions against a simple held-out alternative policy in shadow mode.
```

```
F-21 | Systems | Major | Confidence: High
Anchor: Table VI; Table IX; Table X; Figure 6
Issue: The ADR cache serves the promise at checkout and is computed from live ATP,
ORE feasibility and Grab ETA. Its refresh interval and staleness bound are never
stated. Table X bounds the stock feed at 30 seconds of staleness, but courier
availability moves faster than that under surge, and Section VII-A calls the
dynamically computed promise the single most important conversion lever.
Failure scenario: A rainstorm collapses Grab availability across a cluster. The ADR
cache keeps showing PX1 for several minutes. Consumers buy on a 60-minute promise,
the ORE finds no feasible node, and the orders re-quote to PX2 after purchase.
Fix: State the ADR staleness bound per input, and require the courier-availability
input to refresh faster than the stock input.
```

```
F-22 | Systems | Major | Confidence: Medium
Anchor: Table VIII; Section VIII-E; Section VIII-J
Issue: The ORE decision log is append-only and replayable, the bus is at-least-once
with idempotency keys, and reservations have a TTL whose value is never given. The
idempotency guarantee is stated for bus delivery, not for the reservation lifecycle.
Nothing states that a replayed routing decision is inert against a live VIS.
Failure scenario: During recovery an operator replays a decision segment into the
live path rather than the twin. Reservations are re-issued against stock already
committed, and the ledger's own correctness argument does not cover it.
Fix: State the reservation TTL, and make replay target-aware so a decision log can
only be applied to the twin unless explicitly re-authorized.
```

```
F-23 | Systems | Minor | Confidence: High
Anchor: Table IX; Figure 6
Issue: Table XI and Table IX give per-service p99 latencies (promise 50 ms, ATP
reserve 10 ms, route solve 100 ms, courier book 2 s, node push 1 s). Summing p99s
does not yield the p99 of the composed path. Figure 6 implies an end-to-end budget
of roughly 8 seconds from order to pick start, and no composed-path target is stated
or tested anywhere.
Failure scenario: Every service meets its individual target and the composed p99 sits
well outside the Figure 6 timeline, eroding the in-node time budget that
Proposition 2 converts into service radius.
Fix: State a composed order-to-pick-start p99 and test it as its own NFR.
```

## Lens 4 — Unit economics and commercial structure

```
F-24 | Economics | Critical | Confidence: High
Anchor: Section X Phase 1; Table IV; Section IX-C
Issue: The Phase 1 gate is "contribution/order within 15% of twin prediction." That
tests agreement between two models, not agreement with reality. Table IV is
explicitly labelled design targets rather than commitments, and the twin is
calibrated on Tegal marketplace history, which is a different fulfillment mode. A
confidently wrong twin passes the gate.
Failure scenario: The twin predicts IDR 60,000 contribution, actuals land at IDR
56,000, the gate passes at 7% deviation, and nobody notices that the design target
underpinning the whole capex case was IDR 75,000.
Fix: Re-cut the gate as an absolute threshold against the design target, and keep
twin agreement as a separate calibration check.
```

```
F-25 | Economics | Critical | Confidence: High
Anchor: Section X risks; Section VII-A; Section IX-E
Issue: Cannibalization of B2B doors is listed as a risk in Section X and the same
mechanism is presented as a partner benefit in Section VII-A. Both cannot hold at
full magnitude. If the consignment store would have made the sale over the counter,
ParagonCorp is paying a per-order fulfillment fee plus a rider fee for revenue it
already had. No estimate of incremental versus cannibalized volume appears anywhere,
and none of experiments E1 through E6 measures it.
Failure scenario: 40% of store-node orders are walk-in customers who ordered on their
phone from inside or near the store. Node economics look healthy per order and the
program is margin-negative against the counterfactual.
Fix: Add a holdout experiment that measures incremental volume against matched
non-participating stores, and gate Phase 2 on the incrementality result.
```

```
F-26 | Economics | Major | Confidence: High
Anchor: Table IV
Issue: The stated break-even of approximately 40-80 orders per day does not reconcile
with the other lines in the same table. Fixed cost of IDR 35-60M per month is
1.17-2.0M per day; contribution after fulfillment of IDR 55,000-90,000 implies
13-36 orders per day. Loading 2-6 pickers per shift at plausible Jakarta rates raises
it to roughly 16-53. Either contribution is lower than stated or the fixed base is
larger, and the table does not say which.
Failure scenario: A reviewer checks the arithmetic, finds the headline number
unsupported by the table it sits in, and discounts the rest of the economics.
Fix: Show the break-even derivation line by line, including every cost inside the
fixed base.
```

```
F-27 | Economics | Major | Confidence: Medium
Anchor: Table IV; Section VII-A
Issue: The node P&L omits or leaves ambiguous several material lines. Marketplace
commission on the sale is not avoided by self-fulfilling, and the table does not say
whether it sits inside the 40-55% gross margin. The delivery fee that Section VII-A
calls "subsidizable as a promotion lever" has no assumed subsidy rate. Packaging,
QR seals, returns and inventory carrying cost are absent.
Failure scenario: Marketplace commission turns out to be outside the margin line. At
10% of a IDR 220,000 order that is IDR 22,000, roughly a quarter to a third of stated
contribution, and break-even moves by a similar proportion.
Fix: State every line in and out of the contribution figure, and show the P&L at a
fully subsidized delivery fee as the downside case.
```

```
F-28 | Economics | Major | Confidence: High
Anchor: Section VII-B; Table IV
Issue: The own-fleet decision threshold, roughly 35-45 orders per day per dedicated
rider-equivalent at 70% utilization, is stated without the cost comparison behind it.
No rider cost, vehicle cost, insurance or per-km Grab rate is given. Courier fee is
the largest variable line in Table IV, and the threshold cannot be re-derived when
Grab's rates move.
Failure scenario: Grab raises instant per-km pricing 20%. Nobody can recompute the
threshold from the report, so the own-fleet review in Phase 3 restarts from scratch.
Fix: Publish the cost model behind the threshold as an equation with named inputs.
```

```
F-29 | Economics | Minor | Confidence: High
Anchor: Section X risks; Table XII
Issue: The mitigation offered for marketplace program dependence is "multi-channel
demand, direct channels growing under the same promise." Growth of direct channels
is a strategy, not a mitigation: it has no target share, no date, and no gate.
Failure scenario: Direct channels stay at a small share through Phase 2 and the named
mitigation was never tracked because it had no measurable form.
Fix: Convert it into a Phase 2 gate with a direct-channel share target.
```

## Lens 5 — Indonesia execution reality

```
F-30 | Execution | Critical | Confidence: High
Anchor: Section VII-A; Table V; Section II-G; Section X
Issue: The entire PX1 promise depends on Shopee and TikTok Shop permitting
seller-fulfilled instant orders from ParagonCorp's own nodes with a ParaExpress-booked
courier. Table V records the contract as "seller-fulfilled instant SLA programs;
order webhooks; promise ADR feed." No named program, no letter of intent and no pilot
agreement appears. Section II-G notes Shopee holds roughly 63% of platform beauty GMV
and deployed 200 of its own micro-fulfillment centres, so the counterparty is a
direct competitor in this exact layer. Phases 1 through 4 are all downstream of a
permission that is asserted.
Failure scenario: Shopee declines seller-fulfilled instant for a category where it
runs its own MFCs, or grants it on terms that require SPX Instant as the courier. The
routing engine loses its cost lever and the design reduces to brand.com and WhatsApp,
which carry a fraction of the volume the node economics require.
Fix: Move a written answer from one marketplace into the Phase 0 gate, and specify
the fallback network economics if the answer is no.
```

```
F-31 | Execution | Critical | Confidence: High
Anchor: Table V; Section VIII-C; Section VI-A; Section X Phase 1
Issue: The consignment-store thesis is the zero-capex core of the design, and the
partner-side ask is large: reserved stock in a distinct ledger location, a physically
locked shelf, an Android pick app, accept-pick-pack-handover within 7 minutes, POS
posting within 5 minutes, cycle-count duty, and an accuracy score that reduces order
allocation. The instrument is an addendum to an existing consignment agreement. There
is no partner-side P&L, no expected sign-up rate, no recruitment plan, and no
handling for stores that sign and then do not comply. Phase 1 assumes 15-25 nodes.
Failure scenario: Recruitment converts at 15% rather than the assumed rate, the pilot
cluster reaches 6 store nodes, coverage does not clear n_min, and the pilot tests a
network that was never built.
Fix: Model the partner P&L, run a recruitment pre-test before Phase 1 commits, and
specify the exit path for a non-compliant node.
```

```
F-32 | Execution | Critical | Confidence: High
Anchor: Section II-E; Table IV; Section IX-E
Issue: The demand side of the thesis has no Indonesian evidence. Section II-E argues
beauty clears break-even at a fraction of grocery's order density and cites Table IV,
which is a design-target P&L built on the assumption being tested. There is no
observed instant-attach rate for beauty on an Indonesian marketplace, no conversion
lift for a 60-minute promise, and no willingness-to-pay for the delivery fee.
Experiments E1 through E6 vary node count, assortment depth, courier strategy,
campaign stress, disruption and POS latency; every one is a supply-side test.
Failure scenario: Indonesian beauty buyers convert on a 60-minute promise at half the
assumed rate. Nodes run at 20 orders per day, every operational KPI is green, and the
network is structurally loss-making with no experiment designed to have caught it.
Fix: Add a demand experiment to the E-series before Phase 1: a promise A/B on
brand.com or a single marketplace cluster measuring attach rate, conversion lift and
fee sensitivity.
```

```
F-33 | Execution | Major | Confidence: High
Anchor: Section VII-B; Table V; Section IX-E
Issue: Dual-carrier last mile treats Grab and GoSend as independent capacity. In
Jakarta they draw on substantially the same rider pool and respond to the same rain,
the same rush hour and the same live-commerce spikes. Failover between correlated
suppliers is not redundancy. Experiment E5 stresses flood-day travel times and surge
pricing but scores SLA retention, not the correlation between the two carriers'
availability.
Failure scenario: A heavy afternoon rain collapses both carriers together. The rate
shop finds no acceptable quote from either, the promise waterfall dumps a whole
cluster to PX2, and the resilience score never modelled the joint event.
Fix: Measure the correlation between Grab and GoSend availability from historical
booking data, and treat E5's carrier availability as a joint, not marginal,
distribution.
```

```
F-34 | Execution | Major | Confidence: Medium
Anchor: Table IV; Table III; Proposition 3
Issue: Pick-pack labour is priced at IDR 4,000-6,000 per order, square-root staffed.
Table III puts 2-6 pickers per shift in a 150-300 square metre leased site, which in
Indonesia is a formal employment relationship carrying minimum-wage floors, BPJS
contributions and THR. The report never states whether pickers are ParagonCorp
employees, an outsourced vendor's, or the 3PL's. That choice changes both the unit
cost and the shift flexibility the square-root staffing rule assumes.
Failure scenario: Formal employment sets a fixed floor per node regardless of volume.
The staffing curve that Proposition 3 makes sublinear becomes a step function, and
low-volume nodes carry cost the model treats as variable.
Fix: State the employment model per node format and re-derive the labour line under
it.
```

```
F-35 | Execution | Major | Confidence: High
Anchor: Section X Phases 2-4
Issue: Phases 0, 1 and 2 carry explicit acceptance gates. Phase 3, the multi-metro
expansion to Bandung, Surabaya, Medan and Makassar across months 18-30, has none, and
Phase 4 has none. Theorem 3's n* depends on metro-specific volume and volatility that
have been measured nowhere outside Jakarta.
Failure scenario: Phase 3 launches on Jakarta parameters. Surabaya's demand density
does not support the node count, and there is no stated criterion that would have
stopped it.
Fix: Give Phases 3 and 4 their own gates, with a per-metro parameter re-estimation
required before each expansion.
```

## Lens 6 — The report as a written argument

```
F-36 | Report | Critical | Confidence: High
Anchor: Theorem 3; Proposition 2; Figure 8; Section IX-E
Issue: No sensitivity analysis appears anywhere. Figure 8 sweeps node count for one
parameter set explicitly labelled illustrative. Theorem 3's n* depends on volume,
volatility, holding cost, fill-rate target, courier cost per km, the metric constant
k and metro area; Proposition 2's n_min depends on speed, promise, route time,
in-node time and packing efficiency. None has a stated range, source, or contribution
to variance. Experiment E1 sweeps n and holds every parameter fixed.
Failure scenario: A reader cannot tell whether real Jakarta gives n* = 9 or n* = 25.
The capex ask, the recruitment target and the phase plan all rest on a number whose
uncertainty is unquantified and unquantifiable from the document.
Fix: Add a tornado chart over the parameters of Theorem 3 and Proposition 2 with
stated ranges and sources, and report n* as an interval.
```

```
F-37 | Report | Critical | Confidence: High
Anchor: Section X; Section XI
Issue: The report never states what would falsify its thesis. The phase gates test
execution quality: twin deviation, contribution per order, pick SLA, promise-shown
rate, network contribution. None tests whether the premise holds, and no kill
criterion for the program appears. Section XI closes on a claim about who will win
the category rather than on the conditions under which ParagonCorp should stop.
Failure scenario: Phase 1 hits every gate on a subsidized promise in the densest
cluster in Indonesia. The gates certify that a well-run pilot can be well run, and
Phase 2 commits capital on that basis.
Fix: State the kill criteria explicitly, tied to demand-side outcomes, and put them
alongside the gates in Section X.
```

```
F-38 | Report | Major | Confidence: High
Anchor: Table I; Table XII
Issue: Table I states competitor internals (assortment per node, promise, key
mechanism) with references grouped per row rather than per cell, and several values
rest on a single press source. Table XII then compares ParaExpress against
"benchmark practice" summarized from Table I, so an under-supported cell in Table I
propagates into a superiority claim in Table XII.
Failure scenario: A reviewer checks the Astro or Freshippo row, finds the figure is a
press estimate rather than a disclosed operating number, and treats the whole
benchmark, and the superiority table built on it, as unreliable.
Fix: Cite per cell, and mark estimated values as estimates in both tables.
```

```
F-39 | Report | Major | Confidence: High
Anchor: Table XII; Section VIII-K
Issue: Table XII compares a specification against deployed practice. Every
ParaExpress cell is design intent; every benchmark cell is a system running today.
The caption concedes this in one word ("designed to") and the surrounding text then
uses the table as evidence of capability advantage.
Failure scenario: The table is lifted into a steering-committee deck, where "exact
min-cost flow with congestion and displacement pricing" versus "nearest-node rules"
reads as a shipped advantage over Shopee rather than as a plan.
Fix: Retitle the columns as ParaExpress target versus benchmark observed, and add a
third column for ParaExpress achieved, empty until Phase 1 fills it.
```

```
F-40 | Report | Minor | Confidence: Medium
Anchor: principle P-3; Section VIII-B
Issue: Two load-bearing operational claims have no derivation. "A 20-40 square metre
back room holds a week of instant demand" carries the store-node feasibility case and
is stated without a slot count, a cube calculation or a demand rate. The 99.9% pick
accuracy target is set with no current Heron baseline to show the gap.
Failure scenario: The back-room claim is used to qualify candidate stores, the real
requirement is 60 square metres, and the recruitment funnel is built on the wrong
filter.
Fix: Show the slot arithmetic behind the back-room claim and state Heron's current
pick accuracy alongside the target.
```

```
F-41 | Report | Minor | Confidence: High
Anchor: References [17], [20], [21], [31], [33], [38]
Issue: Reference quality is uneven in a way that tracks the argument's weak points.
The optimization and simulation citations ([39]-[74]) are journal-grade. The market
and competitor citations lean on trade blogs, press releases, personal blogs and
vendor pages. The strategic premise, that instant beauty is a real market in
Indonesia, rests on the weaker half.
Failure scenario: An external reviewer notices the asymmetry and reads it as rigor
applied where it was easy and withheld where it mattered.
Fix: Upgrade the market citations to industry data or primary research, or mark the
market section's claims as directional.
```

---

## DROPPED

Seven candidate findings were checked and did not survive.

1. **Theorem 1's Set Cover reduction is invalid.** Checked. One demand zone per element, one candidate node per set with zero-cost coverage, uniform fixed cost epsilon, and margin M much larger than the total fixed cost together force any optimal solution to cover all zones and minimize the node count. The reduction is standard and correct.
2. **Remark 2's worked numbers do not check.** They do. The cubic 2.4x³ + 6x² − 120 = 0 has its positive root at x ≈ 3.012, giving n\* ≈ 9.07 and C(n\*) ≈ 68.8, matching the text. The F → 0 limit b/a = 20 is also correct.
3. **Table III's node throughput contradicts Section VIII-B's picker rate.** It does not. 2-6 pickers at 10-14 orders per hour maps onto the stated 20-60 orders per hour for a dark MFC, and 6-15 pickers maps onto 60-150 for a flagship.
4. **The claim that 3 minutes of in-node time buys 1 km of radius at 20 km/h is wrong.** It is right, and the quadratic node-count consequence follows correctly from n_min scaling as 1/R².
5. **Theorem 5's O(|O|²|F|) complexity is overstated.** It is not. Successive shortest paths over a bipartite graph with O(|O||F|) arcs, one unit augmentation per order, each a Dijkstra pass with Johnson potentials, gives the stated bound up to logarithmic factors.
6. **Theorem 3's cubic is misderived.** Verified by differentiation: C'(n) = F + (a/2)n^(−1/2) − (b/2)n^(−3/2), multiplied by n^(3/2), substituting x = √n and doubling gives exactly 2Fx³ + ax² − b = 0.
7. **Proposition 1 contradicts Proposition 3 by making decentralization simultaneously costly and cheap.** It does not. Proposition 1 is about inventory, which is penalized by splitting; Proposition 3 is about labour, which is also penalized by splitting. Both point the same way and the report presents them as counterbalancing Proposition 2's coverage pressure, which is consistent.

---

## Unverified dependencies

Facts this review would need from outside the report to settle a finding.

1. Whether Shopee's and TikTok Shop's Indonesian seller-fulfilled instant programs currently permit a brand to fulfill from its own nodes with a self-booked courier, and on what terms (F-30).
2. Current GrabExpress Instant and GoSend Instant commercial per-km rates at ParagonCorp's projected volume tier (F-27, F-28).
3. Observed instant-attach rate, conversion lift and delivery-fee willingness-to-pay for beauty on Indonesian marketplaces (F-32).
4. Actual single-brand basket share by count and by revenue in MagPie's B2C order history, which is the epsilon in Theorem 4(iii) (F-06).
5. Basket-size distribution in MagPie history, needed to convert the order-level 95% promise into a per-SKU fill target (F-12).
6. Heron's current pick accuracy and pick time baselines at Tegal (F-40).
7. Exagon partner count, geographic distribution, and the current distribution of stock accuracy across partners (F-31).
8. Whether marketplace commission sits inside or outside Table IV's 40-55% gross margin line (F-27).
9. Historical joint availability of Grab and GoSend in Jakarta during rain and peak (F-33).
10. Jakarta Class-B/C rents for 150-300 square metre light-industrial and ruko space in 2026, to check the IDR 35-60M range (F-26).

---

## Fix these first

1. **F-12 — set the fill-rate target from the basket, not the SKU.** One line of arithmetic shows a 98% per-SKU target yields 94.1% availability on a three-item basket, below the 95% PX1 promise. It costs one recomputation and it moves base stock, assortment depth and node economics everywhere downstream. Highest consequence per unit of effort in the document.
2. **F-30 — get a written answer from one marketplace before Phase 0 closes.** Every phase is downstream of a permission that is currently assumed. This is a commercial conversation, not a study, and the answer changes whether the rest of the report is a plan or a hypothesis.
3. **F-32 and F-37 — add one demand experiment and a kill criterion.** The E-series has six supply-side experiments and zero demand tests, and Section X has five gates and no stop condition. A promise A/B on brand.com is cheap, runs inside Phase 0, and is the only thing in this list that can tell you the thesis is wrong before capital commits.
4. **F-24 — re-cut the Phase 1 gate against the design target rather than against twin agreement.** A wording change with a large governance effect: as written, a confidently wrong twin passes.
5. **F-02 and F-03 — recompute n_min and n\* under a joint quantile and fitted demand correlation.** Both errors push the required node count up, both are recomputations ParaTwin can run on existing data, and together they change the capex ask and the recruitment target before Phase 1 sets them.
