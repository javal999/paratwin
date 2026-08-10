# ParaExpress open items register

**Source:** Technical Report Revision 5, Section XI, Table 9.
**Last reviewed:** 2026-08-10

25 items. 10 block Phase 0: 8 must close before Phase 0 exits, 2 before it starts.

An item is here because the specification needs the number and nobody has confirmed it. Writing a plausible value instead produces a design that runs and is quietly wrong, so the rule is a visible hole with an owner rather than a guess.

## Blocking Phase 0

| ID | Question | Owner (role) | Blocks | Status |
|---|---|---|---|---|
| O-8 | Numeric thresholds for kill criteria 1 to 5, published before any experiment reads out | R&D + Finance | Phase 0 start | Open |
| O-25 | Named individual owners against every role in this register | Head of Platform R&D | Phase 0 start | Open |
| O-1 | Written marketplace confirmation that seller-fulfilled instant from brand nodes is permitted, and on what courier terms | Commercial Lead | Phase 0 exit | Open |
| O-2 | Instant attach rate, conversion lift, delivery-fee elasticity for beauty (experiment E7) | Commercial Lead | Phase 0 exit | Open |
| O-3 | Distinct-SKU basket-size distribution, which sets the item fill rate the order promise requires | Data Lead | Phase 0 exit | Open |
| O-4 | Grab and GoSend commercial rate card at projected volume tier | Commercial Lead | Phase 0 exit | Open |
| O-5 | Consignment partner-side P&L, recruitment pre-test conversion, non-compliance exit path | Consignment Ops | Phase 0 exit | Open |
| O-6 | Heron pick-accuracy and pick-time baselines at Tegal | Warehouse Systems | Phase 0 exit | Open |
| O-7 | Whether channel commission sits inside or outside the 40 to 55% gross-margin line | Commercial Lead | Phase 0 exit | Open |
| O-23 | Assumed delivery-fee subsidy rate | Commercial Lead | Phase 0 exit | Open |

## Blocking Phase 1

| ID | Question | Owner (role) | Blocks | Status |
|---|---|---|---|---|
| O-9 | Measured packing and overlap efficiency per metro | Data Lead | Phase 1 gate | Open |
| O-10 | Fitted in-node time and effective-speed quantile functions | Data Lead | Phase 1 gate | Open |
| O-11 | Fitted zone-demand correlation by hour and campaign state | Data Lead | Phase 1 gate | Open |
| O-12 | Single-brand basket share by count and by revenue | Data Lead | Phase 1 gate | Open |
| O-13 | Basket-value distribution and the node-activation contribution floor | Data Lead | Phase 1 gate | Open |
| O-14 | Measured store-node picking capacity under observed walk-in traffic | Consignment Ops | Phase 1 gate | Open |
| O-15 | POS posting-latency and connectivity-uptime distribution across partners | Consignment Ops | Phase 1 gate | Open |
| O-16 | Virtual Inventory Service recovery-time objective and measured failover; reservation TTL; composed order-to-pick p99 | Platform Eng | Phase 1 gate | Open |
| O-17 | Missing P&L lines: returns rate and cost, packaging, carrying cost, employment model, store-node shrink allowance, rent validation, break-even derivation | Finance + Ops | Phase 1 gate | Open |
| O-18 | Joint availability and price correlation of Grab and GoSend | Data Lead | Phase 1 gate | Open |
| O-19 | Minimum back-room footprint holding one week of instant demand | Platform R&D | Phase 1 gate | Open |
| O-22 | AOV distribution, not mean, by catchment | Data Lead | Phase 1 gate | Open |
| O-24 | Regulatory review: BPOM storage and traceability at Tier-2 nodes; halal-chain integrity across consignment partners | Regulatory Affairs | Phase 1 gate | Open |
| O-20 | Proof or counterexample for submodularity of the splittable capacitated coverage value | Platform R&D | Phase 1 exit | Open |

## Blocking Phase 2

| ID | Question | Owner (role) | Blocks | Status |
|---|---|---|---|---|
| O-21 | Own-fleet cost model and the equation behind the 35 to 45 orders per rider threshold | Finance + Commercial | Phase 2 gate | Open |

## The 3 that decide whether the program exists

**O-1, marketplace access.** Shopee holds roughly 63% of platform beauty GMV and runs 200 of its own micro-fulfillment centres. It is both the largest demand channel and a direct operator in the layer ParaExpress wants to occupy. Every phase is downstream of a permission that Revision 4 asserted. This is a commercial conversation, not a study.

**O-2, demand.** Revision 4 had 6 supply-side experiments and no demand test. Whether Indonesian beauty consumers convert on a 60-minute promise at a rate that fills a node is the single assumption every node economic rests on, and it was never scheduled to be measured. Experiment E7 runs on brand.com inside Phase 0 and does not need the network.

**O-3, basket size.** A PX1 order fails if any SKU in the basket is missing, so the item fill rate the 95% order promise requires depends on how many distinct SKUs a basket holds. At a mean of 2.45 distinct SKUs, a 98% item target yields 95.2% order availability. At 3.05 it yields 94.0%, below the promise. Nobody has pulled the distribution from MagPie.
