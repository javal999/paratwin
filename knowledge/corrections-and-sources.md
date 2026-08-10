# ParaExpress Revision 5: corrections and their sources

Every correction in Revision 5 applies a known result. None is a new mathematical contribution. This file is the lookup table: what changed, why, and the published work it rests on.

Full derivations are in the report, Appendix A-B. This is the index.

## Corrections

| Change | What Revision 4 did | Why it is wrong | Rests on |
|---|---|---|---|
| **Proposition 3, order fill rate** (new) | Sized safety stock from an item fill rate, promised 95% measured on the order | An order fails if any basket SKU is missing, so the two quantities differ by the basket-size generating function. Inverting it gives the item rate an order promise requires | Song 1998, Oper. Res. 46(6); Hausman, Lee & Zhang 1998, EJOR 109(3); Esary, Proschan & Walkup 1967, Ann. Math. Statist. 38(5) for the association bound |
| **Proposition 2, service radius** (restated) | Subtracted a single quantile of in-node time from the promise budget, treating travel as deterministic at a mean speed | Uses an identity that fails for any random travel time. Quantiles and value-at-risk are not additive. Replaced by a Boole risk-budget split, distribution-free and dependence-free | Artzner, Delbaen, Eber & Heath 1999, Math. Finance 9(3); Embrechts, McNeil & Straumann 2002; Boole via Feller 1968 |
| **Proposition 1, pooling penalty** (restated) | Square-root safety-stock inflation assuming independent zone demand | The design models correlated demand bursts and re-pools through substitution and rebalancing. Eppen's own 1979 paper covers the correlated case; Revision 4 cited it and used only the independent special case | Eppen 1979, Manage. Sci. 25(5) |
| **Theorem 3, node count** (restated) | Single optimal node count on the independent pooling law | Restated on the corrected law. The optimum rises with correlation, so Revision 4 biased the node count down | Follows from Proposition 1 |
| **Section IV-B, capacitated coverage** | Cited the greedy 63.2% guarantee, proved for uncapacitated nodes, in a network with binding throughput caps | Explicit counterexample: capacity creates complementarity, so whole-order coverage is not submodular and greedy loses the inherited bound. A guarantee is recoverable by a different algorithm | Nemhauser, Wolsey & Fisher 1978 for scope; Chekuri & Kumar 2004 for the cardinality-constrained case; Fleischer, Goemans, Mirrokni & Sviridenko 2011; Ageev & Sviridenko 2004 |
| **Proposition 4, staffing** (restated) | Halfin-Whitt square-root staffing on a Poisson-arrival queue, with wave-batched release specified elsewhere in the same document | Wave release raises the variability of the stream offered to pickers. The safety term inflates, which makes wave length a two-sided decision rather than a routing-only parameter | Halfin & Whitt 1981, Oper. Res. 29(3); Whitt 1993, POM 2(2); Whitt 2004, Oper. Res. 52(6); Chaudhry & Templeton 1983 |
| **Proposition 5, available-to-promise safety** (restated) | Single-partition invariant with a fixed 5-minute posting latency | The order manager moves stock across partitions and the store client may run offline for hours. Adds effective latency from liveness heartbeats, allocation suspension, and a cross-partition commit rule | Herlihy & Wing 1990, TOPLAS 12(3); Gray & Reuter 1993; Garcia-Molina & Salem 1987, SIGMOD; Gilbert & Lynch 2002, SIGACT News 33(2) |
| **Commitment policy** (new) | "Re-solved every few seconds" with per-wave optimality claimed | States the commitment rule, reframes the router as online, and measures regret against clairvoyant instead of claiming an unproven bound | Klapp, Erera & Toriello 2018, EJOR 271(2); Ulmer et al. 2021, Transp. Sci. 55(1); Lan et al. 2024, Transp. Sci. 58(4) |
| **Theorem 4 part (iv), basket bound** | Degradation factor stated on the count share of cross-brand baskets | The objective measures revenue, so the bound has to be on revenue share. Cross-brand beauty baskets are plausibly larger than average | Sviridenko 2004, Oper. Res. Lett. 32(1); Kellerer, Pferschy & Pisinger 2004 |
| **Twin policy check** (new) | Shared decision code between twin and production presented as pure benefit | Shared code removes drift and removes independent verification of the decision model. Adds an off-policy scored held-out policy | Sargent 2013, J. Simulation 7(1); Li, Chu, Langford & Wang 2011, WSDM |
| **Experiment E8, incrementality** (new) | Cannibalization listed as a risk while the same mechanism was sold as a partner benefit | Makes incrementality a measured treatment effect with a matched-market design | Imbens & Rubin 2015; Abadie, Diamond & Hainmueller 2010, JASA 105(490) |
| **Kill criteria** (new) | Gates tested execution quality only, with no stop condition | A design with no falsifying observation cannot be tested by its own gates | Popper 1959; Klein 2007, HBR 85(9); Flyvbjerg 2006, PMJ 37(3) |
| **Appendix B, sensitivity** (new) | One illustrative curve, parameters fixed | The design rests on parameters with no stated ranges. Adds a ranked variance-contribution analysis | Saltelli et al. 2008 |

## The review method itself

The six-lens structure is not arbitrary. Perspective and scenario-based reading beat ad hoc and checklist review in replicated controlled experiments, and the mechanism is coverage: one reader with a general mandate re-finds the defects their own expertise makes salient.

- Fagan 1976, IBM Syst. J. 15(3) — structured inspection with defined roles.
- Porter, Votta & Basili 1995, IEEE TSE 21(6) — defect-based scenario reading beats ad hoc and checklist.
- Basili et al. 1996, Empirical Softw. Eng. 1(2) — perspective-based reading.
- Klein 2007, HBR 85(9) — the premortem, applied here to the review's own findings rather than to the plan.

The adversarial pass discarded 7 of 48 candidate findings, 5 of them because the reviewer's arithmetic was wrong and Revision 4's was right. Those 7 are recorded in the report, Appendix A-E, so they are not raised again.
