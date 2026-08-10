# ParaTwin — ParaExpress Digital Twin Simulator

Desktop digital twin of the **ParaExpress** hyperlocal fulfillment network
(ParagonCorp / ParaTech Platform R&D). Simulates the full order lifecycle —
marketplace instant orders → wave routing → MFC/store-node picking → Grab/GoSend
last mile — and turns the results into network, partnership and economics decisions.

Companion to the technical report *ParaExpress: A Mathematically Grounded
Hyperlocal Fulfillment Network and Digital Twin Architecture*
(`../deliverables/ParaExpress-Technical-Report/paraexpress-technical-report-rev5.pdf`).
The Revision-5 correction backlog this code must still absorb is in
`../docs/paratwin-rev5-epic-prompt.md`.

## Simulation models

| Formalism | Where |
|---|---|
| Discrete-event simulation (DES) | `engine/src/des.rs` — event calendar over arrivals, waves, picks, couriers, replenishment, POS/ledger |
| Agent-based modeling | courier agents (idle → to-pickup → to-drop) animated on the metro map |
| Monte Carlo / probabilistic | spatial Poisson demand (Gaussian-mixture field), lognormal pick/match/travel, Bernoulli OTIF & carrier failure, R replications with Student-t 95% CIs |
| State-flow | explicit order state machine (created → routed → picking → courier-matched → handover → delivered / downgraded / lost / oversell) |
| Exact optimization | wave routing via min-cost flow (integral LP, Theorem 5); greedy submodular auto-layout (Theorem 2) |

**Mathematically proven:** the *Model Validation* page runs live simulations and
gates them against the report's closed forms (Erlang-C, Theorem 3's n\*, Eppen's
√n pooling, square-root staffing). `cargo test -p paratwin-engine` additionally
verifies M/M/1 P(wait)=ρ, Erlang table values, routing vs brute force, the
report's Fig. 8 numbers, determinism, and the accounting identity.

## Tech stack

- **Tauri 2** desktop shell (Mac `.dmg` / Windows NSIS `.exe`)
- **Next.js 14 + React 18** frontend, **Nivo** charts, custom canvas animation
- **Rust** engine (`engine/`), shared host layer (`host/`)
- **SQLite** (scenarios, runs, KPI summaries) + **RocksDB** (animation frame
  store & time series; key = run_id ‖ minute, LZ4)

## Develop

```bash
# 1. engine tests
cargo test -p paratwin-engine --release

# 2. browser dev mode (frontend + engine over HTTP)
cargo run -p paratwin-devserver          # http://127.0.0.1:8787
cd web && npm install && npm run dev     # http://localhost:3000

# 3. desktop dev
cd web && npx tauri dev
```

## Build installers

```bash
./web/node_modules/.bin/tauri build      # run from the paratwin/ root
# If the styled DMG step fails (headless session), package it directly:
./scripts/make-dmg.sh
```

Windows `.exe` (NSIS) cannot be cross-compiled from macOS: push this folder to
GitHub and run the **Build ParaTwin installers** workflow
(`.github/workflows/build-installers.yml`) — it produces macOS arm64/x64 `.dmg`
and Windows NSIS installers on native runners.

## Layout

```
engine/     pure Rust simulation + analytics (no I/O)
host/       SQLite + RocksDB persistence, service functions
devserver/  axum HTTP mirror of the Tauri commands (browser dev/testing)
src-tauri/  Tauri v2 app (commands, icons, bundling)
web/        Next.js UI (dashboard, scenarios, runner, results, decisions, math)
```
