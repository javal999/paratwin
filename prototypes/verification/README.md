# Verification scripts

Reproduce every computed number in the ParaExpress technical report. Run them before trusting a figure that came out of a model.

## Requirements

```bash
pip install numpy scipy matplotlib
```

## Scripts

**`verify_results.py`** — checks the closed forms in Section IV.

- Pooling law across correlation values, including the limits at 0 and 1.
- Node-count curve: optimum, cost at optimum, unimodality by counting interior turning points, and the monotone rise of the optimum with correlation.
- Wave-batching inflation factors for each node format at 5 and 30 second waves, and the resulting picker headcount.
- Order fill rate inversion under 3 basket-size distributions.
- Service radius under the risk-budget split, against the Revision 4 formula, and the resulting node floor.

**`submodularity_search.py`** — searches small instances of the capacitated coverage problem for a diminishing-returns violation. Finds one for whole-order assignment and none for the splittable relaxation over 700 instances. The counterexample printed here is the one in the report, Section IV-B.

Runs for several minutes. The 4-by-4 adversarial pass is the slow part.

**`make_cost_curve.py`** — regenerates `tex/figures/costcurve.pdf`, the design curve in Figure 1.

## What these do not do

They check the arithmetic, not the inputs. Every parameter is illustrative until the open items close. See `deliverables/ParaExpress-Technical-Report/open-items-register.md`.
