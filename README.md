# paraexpress

Brand-owned hyperlocal fulfillment network and digital twin for instant beauty commerce in Indonesia. ParaTech Platform R&D.

## Structure

- `paratwin/` — the ParaTwin digital twin simulator: Rust engine, Tauri 2 desktop shell, Next.js frontend
- `deliverables/` — the shipped technical report and its companion review
- `prototypes/` — runnable scripts that reproduce every number in the report
- `knowledge/` — source Revision 4, the review prompt, and the correction-to-source map
- `docs/` — the Revision-5 correction backlog prompt (C-01…C-16) for ParaTwin
- `tex/` — LaTeX source for the report, compiled with `./build-tex.sh`

## Current state

Revision 5 of the technical report is complete and verified. It supersedes Revision 4.

Revision 5 corrects 5 results that were used outside the conditions under which they were proved, adds 3 results the design needed and did not have, replaces an acceptance gate that an incorrect model could pass, adds the demand experiment and kill criteria the program lacked, and registers 25 open items.

10 open items block Phase 0. See `deliverables/ParaExpress-Technical-Report/open-items-register.md`.

## Build

```bash
./build-tex.sh
```

Compiles `tex/main.tex` with `tectonic` and writes `tex/output/main.pdf` (gitignored). The shipped copy in `deliverables/` is a snapshot of that output.

`tectonic` is not vendored in this repo. The script takes it from `$TECTONIC` if set, otherwise from `PATH`:

```bash
TECTONIC=/path/to/tectonic ./build-tex.sh
```

For the simulator, see `paratwin/README.md`.

## Notes

`GOALS.md` holds project scope, milestones and done criteria.
