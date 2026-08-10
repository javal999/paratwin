# Goals (paraexpress)

**Last reviewed:** 2026-08-10

## Linked workspace themes

- `T-ARCH` — primary. ParaExpress is a reference architecture for a new fulfillment tier plus the ParaTwin simulation-optimization loop.
- `T-INV` — the Virtual Inventory Service is a single available-to-promise ledger across marketplace, direct and offline POS channels, which is the same integration problem as inventory integration at a different grain.
- `T-TMS` — mid-mile waves to MOSTRANS and on-demand last mile through Grab and GoSend reuse the transport execution work.
- `T-VMI` — consignment stores as fulfillment nodes extend the Exagon consignment model from replenishment into fulfillment.

## Scope

- Specify a hyperlocal fulfillment network that delivers beauty orders in under 60 minutes from the node nearest the consumer, across marketplace and direct channels.
- Prove the network is buildable: node count, assortment, staffing, routing and inventory correctness each derived rather than asserted.
- Define ParaTwin as the gate between a candidate design and a funded one.

Out of scope for this document: the build itself, vendor selection, and the legal and regulatory review (registered as O-24).

## Milestones

- **Revision 4** (Aug 2026) — first complete specification. Superseded.
- **Revision 5** (Aug 2026) — corrections implemented, 25 open items registered, kill criteria defined. Current.
- **Phase 0 exit gate** — 8 open items closed (O-1 to O-7, O-23), 2 closed at Phase 0 start (O-8, O-25), ParaTwin certified, experiments E1 to E3 and E7 read out.
- **Phase 1 pilot** — 15 to 25 store nodes plus 3 MFCs in the South Jakarta cluster, E8 incrementality holdout live.

## Definition of done for this project

The report is done when a Phase 0 funding decision can be taken from it without anyone having to supply a number that is not in it. That is not true today: 25 open items are outstanding and 10 of them block Phase 0.

Three of those decide whether the program exists at all:

1. **O-1** — will Shopee or TikTok Shop permit seller-fulfilled instant from ParagonCorp nodes. Every phase is downstream of this.
2. **O-2** — do Indonesian beauty consumers convert on a 60-minute promise at a rate that fills a node. Measured by experiment E7.
3. **O-3** — the basket-size distribution, which sets the item fill rate the 95% order promise actually requires.

## Success signals

- Order-level fill rate, not item fill rate, is what gets reported against the PX1 promise.
- No phase gate can be passed by a model agreeing with itself.
- Every number in the report is either measured, derived from a measured number, or marked open.

## Baseline

This project follows root `GOALS.md` as baseline and extends it with project-level outcomes.
