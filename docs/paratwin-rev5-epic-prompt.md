# Prompt: ParaTwin Revision-5 correction backlog

**How to use:** Run this from the root of the ParaTwin repository in Claude Code. Fill the 1 bracketed slot, `[report path]`, with the path to the Revision 5 report and its companion notes, or delete that line if the repository has no access to them.
**Structure:** COSTAR, with the correction register moved to the top as supplied material, plus Constraints, Known unknowns, and Success criteria as named blocks, and Results as a numbered plan with dependencies. That is the planning-and-decomposition shape; no swap table fired.
**Assumptions made:** Extended reasoning is on, so no step-by-step instruction is included.
The target is Claude Code on a current Claude model, so the prompt constrains subagent use and asks for concision.
Story points are Fibonacci and are assigned after reading the code, not before.
The 16 corrections are embedded verbatim rather than referenced, because a PDF read is lossy and every acceptance test depends on an exact formula.

---

<corrections>
Sixteen corrections from Revision 5 of the ParaExpress technical report. Each replaces a rule that Revision 4 got wrong or left unstated. The `Check` line is a value the corrected implementation must reproduce; use it to write the failing test.

**C-01 Order fill rate.** Base stock is sized from an item fill rate. The PX1 promise is measured on the order, and an order fails if any SKU in its basket is missing. Required: given the distribution of distinct SKUs per basket with generating function G(x) = sum over j of p_j x^j, the item fill rate is beta* = G-inverse(alpha_target), and base stock uses z = Phi-inverse(beta*). Reading Phi-inverse(alpha_target) directly is the defect.
Check: with every basket exactly 3 distinct SKUs and alpha_target = 0.95, beta* = 0.983048 and z = 2.1212. With beta = 0.98 the order fill rate is 0.941192, below the promise.

**C-02 Service radius.** The feasible radius subtracts a single quantile of in-node time from the promise budget and treats travel as deterministic at a mean speed. Quantiles are not additive, so this overstates the radius. Required: R(e1,e2) = v_(1-e2) * (tau - T_route - t_node_(1-e1)) clamped at zero, maximised over e1 + e2 = 1 - alpha, where v_q is the q-quantile effective speed and t_node_q the q-quantile in-node time. Then n_min = ceil(A / (eta * pi * R*^2)).
Check: tau = 60 min, T_route = 4 min, alpha = 0.95, in-node time lognormal with median 9 min and log-scale 0.35, effective speed lognormal about a 20 km/h median with log-scale 0.30. Old formula gives R = 13.332 km; corrected gives R* = 7.124 km at (e1,e2) = (0.0174, 0.0326). At A = 662 km2 and eta = 0.65, n_min goes from 2 to 7.

**C-03 Correlated pooling.** Safety stock under decentralisation uses a square-root-of-n inflation that assumes independent zone demand. The demand model has correlated bursts and the order manager re-pools through substitution and rebalancing. Required: Psi(n,rho) = sqrt(n / (1 + (n-1) rho)) replaces sqrt(n), with rho a fitted parameter, not a constant.
Check: Psi(16,0) = 4.0, Psi(16,1) = 1.0, Psi(9,0.3) = 1.6270.

**C-04 Node count.** The cost curve and its optimum are computed on the uncorrected pooling law. Required: C(n) = F*n + a*Psi(n,rho) + b*n^(-1/2), with stationarity psi(n) = F*n^(3/2) + a(1-rho)n / (2*(1+(n-1)rho)^(3/2)) - b/2. Uniqueness of the optimum is proved only at rho = 0, so the optimum must be found by grid evaluation over n, not by a root find that assumes a single crossing.
Check: (F,a,b) = (1.2, 6, 120) gives n* = 9.071 at rho = 0, 13.153 at rho = 0.3, 13.558 at rho = 0.9. At rho = 0, C(n*) = 68.799.

**C-05 Two-stage sizing.** A single optimal node count is computed for a network whose formats have fixed costs differing by two orders of magnitude. Required: stage 1 activates store nodes (fixed cost approximately zero) in greedy marginal-contribution order until the pool is exhausted or contribution falls below the floor, giving n_0. Stage 2 applies C-04 to the dark-MFC layer only, on residual and overflow demand, giving n_mfc_star. Deployed count is max(n_mfc_star, n_min - n_0) clamped at zero, with n_min from C-02 applied to the combined tier.

**C-06 Contribution floor.** Greedy expansion selects nodes by marginal coverage. The objective nets variable fulfilment cost against margin, so covering demand whose basket value does not clear courier fee plus pick cost plus channel commission destroys value while raising coverage. Required: rank and filter by marginal contribution, with an explicit minimum-contribution threshold derived from the basket-value distribution.

**C-07 Capacitated rollout.** The greedy 63.2% guarantee is proved for uncapacitated nodes and is applied to a network with binding throughput caps. Under whole-order assignment, capacity creates complementarity and the coverage value is not submodular, so the guarantee does not transfer. Required: recompute marginals after every activation rather than ranking once and iterating a fixed list; do not claim the bound in code comments, docstrings or reports; add an LP-rounding cross-check as a second candidate generator.
Check: nodes 0,1,2 with zone demand (0.6, 1.0, 0.4), node throughput (0.6, 1.0, 0.4), and unit contribution rows [[1.0,0.6,0.6],[1.0,0.6,1.0],[0.6,1.0,1.0]]. Under whole-order assignment g({1}) = 1.0, g({1,2}) = 1.0, g({0,1}) = 1.2, g({0,1,2}) = 1.6, so the marginal value of node 2 rises from 0 to 0.4 as the set grows.

**C-08 Assortment revenue bound.** The basket-completeness guarantee degrades by a factor derived from the share of baskets that cross brand blocks, counted by basket. The objective measures revenue. Required: compute both the count share and the revenue share, and use the revenue share in the bound and in any reported guarantee.

**C-09 Batch-arrival staffing.** Square-root staffing is applied with a Poisson-arrival assumption while work is released to pickers in waves. Required: c_f = rho_f + beta_delta * sqrt(1 + lambda_f * Delta / 2) * sqrt(rho_f), where Delta is the wave length. Delta becomes a tuned parameter traded against routing quality, not a constant.
Check: the inflation factor at Delta = 30 s is 1.0206 at 10 orders/hour, 1.1180 at 60, and 1.2748 at 150.

**C-10 Effective posting latency and allocation suspension.** The available-to-promise safety condition assumes a fixed 5-minute point-of-sale posting latency while the store client may run offline for hours. Required: effective latency l_eff(t) = max(observed steady-state latency, t - last heartbeat timestamp). The protection buffer is sized against l_eff. When l_eff exceeds the window the buffer can cover within shelf quantity, the node is removed from the feasible set for new orders until a heartbeat and a posting flush arrive.

**C-11 Cross-partition transfer.** Rebalancing, substitution sourcing and split shipment move stock between nodes whose ledger partitions have no cross-partition coordination. Required: such a transfer executes as a two-phase commit across both partitions, or as a compensating saga in which the source decrement commits first and reservations against the source are frozen for the duration. No transfer may proceed while an uncommitted reservation exists on the affected node-SKU pair.

**C-12 Independent buffer signal.** The protection buffer is tuned from each store's own posting-latency history, so the control signal and the risk share a source: when an integration degrades, the observed history lags and the buffer shrinks exactly when it should grow. Required: gate the buffer on an independent liveness and latency probe, not on posting history alone.

**C-13 Router commitment policy.** The router re-solves continuously while claiming per-wave optimality, with no stated rule on whether a re-solve may move an order whose pick has started. Required: an order is committed when its reservation is confirmed and the node task is pushed; a committed order is never reassigned by a later solve, and is released only by cancellation, node outage or reservation expiry. Optimality is claimed per wave over uncommitted orders only. Realised regret against the clairvoyant assignment is computed on completed traces and reported.

**C-14 Independent policy check.** The twin and production share decision code, which removes model drift and also removes independent verification of the decision model. Trace replay and holdout forecasting validate the environment model, not the routing cost model. Required: score a fixed fraction of waves in shadow against a held-out naive policy (nearest feasible node), estimate the difference off-policy, and fail certification when the decision model cannot beat the naive policy on live traffic.

**C-15 Returns as a first-class event.** Failed deliveries and returns have no event, no cost line and no calibration feed, so the twin cannot see a cost it is meant to optimise against. Required: return and re-attempt events in the event set; return-to-stock as a normal ledger transaction with a quality gate; a per-order return-handling cost in the node economics; and a returns feed in the calibration contracts.

**C-16 Lateral replenishment.** A stocked-out node waits for the next replenishment wave, with promise downgrade or substitution as the only recovery, while an equivalent capability exists for external orders. Required: a lateral transfer path between fulfilment nodes, priced by the router against the cost of downgrading the order, executed under C-11, and triggered when expected downgrade cost over the remaining review period exceeds transfer cost.
</corrections>

<report_location>
The Revision 5 report and its companion notes are in this repository at
`deliverables/ParaExpress-Technical-Report/`:

- `paraexpress-technical-report-rev5.pdf` — the shipped report
- `paraexpress-technical-report-rev5-singlefile.tex` — its LaTeX source
- `paraexpress-rev4-design-review.md` — the review that produced the corrections
- `open-items-register.md` — the 25 open items, 10 of which block Phase 0

Related: `knowledge/paraexpress-technical-report-rev4.pdf` (superseded),
`knowledge/corrections-and-sources.md` (correction-to-source map),
`prototypes/verification/` (scripts reproducing the report's numbers).

The ParaTwin source this epic maps onto is under `paratwin/` — the Rust engine
is `paratwin/engine/src/`, the desktop shell `paratwin/src-tauri/`, the frontend
`paratwin/web/`. Story file paths must be written relative to the repository
root and therefore begin with `paratwin/`.
</report_location>

CONTEXT

ParaTwin is the simulation and optimisation codebase for ParaExpress, a hyperlocal fulfilment network. It generates candidate network designs, evaluates them under stochastic demand, and certifies the ones that reach a funding decision. It was built against Revision 4 of the technical report. Revision 5 corrects sixteen rules that Revision 4 either proved under conditions the deployment violates, or left unstated. Every correction above is either a formula the code computes, a policy the code enforces, or an event the model does not currently have.

Two of them change numbers the business acts on. C-01 raises the safety-stock target because the promise is measured on the order and the code sizes on the item. C-03 and C-04 raise the optimal node count because the pooling penalty is overstated. Both errors currently run in the same direction: they produce a network smaller and cheaper than the promise requires.

The engineer reading your output has the repository open and has not read the report.

OBJECTIVE

Produce one markdown file, `paratwin-rev5-epic.md`, containing an epic and a set of Fibonacci-pointed stories that map every correction above onto named files in this repository, sequenced by dependency, ready to be worked one story at a time.

AUDIENCE

A product manager who will take the epic into sprint planning, and the engineers who will pick up individual stories. Neither has read Revision 5. Both can read code. After reading, the product manager decides sprint order and the engineer picks up story 1 without asking a question.

CONSTRAINTS

- Write no implementation code in this pass. Read the repository, produce the backlog, stop.
- Touch one file: `paratwin-rev5-epic.md`. Everything else in the repository is read-only for this pass, including tests, configuration and generated output.
- Point every story on the Fibonacci scale 1, 2, 3, 5, 8, 13. Any story that lands above 8 is split until every piece is 8 or below.
- Base every estimate on files you have opened. Do not size a correction whose code you have not located; record it as unlocated instead.
- Cover all sixteen corrections. A correction with no home in this codebase gets an entry saying so and why, not silence.
- Do not use subagents to verify or re-check your own work.
- Keep prose short. Tables and bullet fields, not paragraphs.

WORK IN TWO STEPS

First, read the repository and work in the open, producing for each correction: the file and function that implements its current behaviour, the specific line quoted, the change required, a point value, and the reason for that value. Cover all sixteen before writing any output structure. Where the code is not there, record what you searched for.

Second, and only then, write `paratwin-rev5-epic.md` in the structure below.

RESULTS

The file contains these sections in this order.

**1. Epic**

One block: epic ID, title, a one-sentence goal, the business outcome in two sentences, scope in and scope out, total story points, and story count.

**2. Stories**

One block per story, in dependency order, each under 200 words:

```
ID:            PTW-##
Title:
Implements:    C-## (one or more)
Points:        1 | 2 | 3 | 5 | 8   -- and one line saying why that number
Files:         real paths from this repository
Current:       what the code does now, with the function or line quoted
Required:      what it must do, stated as the rule or formula
Acceptance:    Given / When / Then, including at least one numeric assertion
                from the correction's Check line where one exists
Test:          the test file and the name of the test to add
Depends on:    story IDs, or "none"
Risk if not done: one sentence naming what stays wrong
```

**3. Traceability table**

Sixteen rows, one per correction: correction ID, story IDs, and status, where status is `mapped`, `no code home` with a reason, or `unlocated` with what you searched for. Every correction appears exactly once.

**4. Sequence**

The stories grouped into waves. A wave contains stories with no unmet dependency. Give each wave a one-line rationale and its point subtotal.

**5. Assumptions**

Everything you assumed about this codebase that you could not confirm from it. One line each, kept out of the story blocks.

**6. What would invalidate this plan**

The conditions under which the sequence or the sizing stops being right. One line each.

**7. Questions**

What you need from the product manager before story 1 starts. One line each, with the story it blocks.

SUCCESS CRITERIA

- All sixteen corrections appear exactly once in the traceability table, and none is silently dropped.
- Every story names at least one real file path from this repository.
- Every story carries an acceptance criterion an engineer could turn into a failing test today, and every correction with a Check line has that numeric value asserted in the story that implements it.
- No story is pointed above 8.
- Every story's dependency field is filled, including the ones with no dependencies.
- No implementation code was written in this pass.

SCOPE

In scope: the sixteen corrections above, the tests that prove them, and the sequencing between them.

Out of scope, and to be listed under Questions rather than planned: reliability work that Revision 5 also specifies but this epic excludes (recovery-time objectives, degraded reservation modes, promise-cache staleness bounds, replay targeting, reservation expiry, composed-path latency budgets); the document-only corrections; and changes to systems outside this repository.

When implementation starts on a later pass, the vibe-code-reviewer and vibe-test-enforcer skills govern that work. This pass produces the backlog only.

Begin by reading the repository. Write the file when the sixteen corrections are located.

---

## Notes on design decisions

- **The corrections are embedded, not referenced.** Each carries the exact formula and, where one exists, a numeric value the fixed code must reproduce. That single choice is what turns a vague "apply the Revision 5 corrections" into sixteen stories with failing tests attached. A prompt that pointed at the PDF would produce a backlog of restated section titles.
- **Sizing happens after reading, and every estimate must name its files.** Points assigned from a specification alone are fiction. The constraint that an unlocated correction is recorded as unlocated rather than sized is what stops the model filling the gap with a plausible number.
- **Two steps, reason then format.** Forcing a structured schema and hard reasoning into one pass measurably degrades the reasoning: JSON output cost GPT-3.5 27 points on GSM8K. The mapping and sizing happen in the open first, the file gets written second.
- **Full coverage, no severity or priority filter.** The prompt never says "focus on the important ones." A correction with no code home produces a visible row saying so. Filtering instructions applied before generation remove findings the filter's author could not have anticipated, and current models follow them faithfully.
- **No persona.** No "you are a staff engineer." Across 12 conditions and 6 models, personas showed no reliable gain on knowledge tasks and 9 significant negative differences on MMLU-Pro. The domain context sits in the Context block instead, where it is actually read.
- **Subagent use is constrained and concision is requested.** Current Claude models delegate readily and run long by default. Neither helps when the deliverable is one file the model can write itself.
- **The two behaviour-changing corrections are named in Context.** C-01 and C-03/C-04 both move numbers the business acts on, and both currently err toward under-building. Saying so up front changes how the model sizes them; leaving it to be inferred from sixteen equal-looking entries does not.
- **Length is bounded per story, never by story count.** A cap on the number of stories is a coverage filter wearing a different hat.
