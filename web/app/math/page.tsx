"use client";

import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { ValidationRow } from "@/lib/types";

const MODELS = [
  {
    name: "Discrete-Event Simulation (DES)",
    detail:
      "Event-calendar kernel over order arrival, wave routing, pick start/end, courier match/arrive, handover, delivery, replenishment, POS sale and ledger posting. Time advances event-to-event; 10⁵–10⁶ events per replication.",
  },
  {
    name: "Agent-Based Model (ABM)",
    detail:
      "Couriers are agents with states idle → to-pickup → to-drop, spawned by the match process, moving on the metro plane at traffic-adjusted speed. The animation replays their trajectories from the RocksDB frame store.",
  },
  {
    name: "Monte Carlo / probabilistic layer",
    detail:
      "Poisson order arrivals over a Gaussian-mixture demand field; lognormal pick, match and travel noise; Bernoulli carrier failure and OTIF; R independent replications with Student-t 95% confidence intervals on every KPI.",
  },
  {
    name: "State-flow (order lifecycle)",
    detail:
      "Every order walks the explicit state machine created → routed → picking → courier-matched → handover → delivered / downgraded / lost / oversell — the same state machine specified for MagPie+ in the report.",
  },
];

const THEOREMS = [
  { ref: "Theorem 1", claim: "SLA-constrained network design is NP-hard", use: "why design runs through the twin, not one exact solve" },
  { ref: "Theorem 2 + Cor. 1", claim: "coverage is submodular; greedy ≥ 63.2% of optimum", use: "auto-layout and store-activation ranking" },
  { ref: "Theorem 3", claim: "C(n)=Fn+a√n+b/√n has a unique minimizer n*", use: "network-size sweep recommendation" },
  { ref: "Prop. 2", claim: "SLA floor n_min = A/(ηπR²), R = v(τ−t_node)⁺", use: "feasibility filter in wave routing" },
  { ref: "Prop. 3", claim: "M/M/c delay probability; square-root staffing", use: "picker staffing per node; validated below" },
  { ref: "Prop. 4", claim: "ATP ledger with buffer s bounds oversell probability", use: "store-node POS latency + buffer mechanics" },
  { ref: "Theorem 5", claim: "wave routing LP is integral; min-cost flow is exact", use: "the router in this app is that exact algorithm" },
];

export default function MathPage() {
  const [rows, setRows] = useState<ValidationRow[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  const run = async () => {
    setBusy(true); setErr("");
    try {
      setRows(await api.validateMath());
    } catch (e) { setErr(String(e)); } finally { setBusy(false); }
  };
  useEffect(() => {
    // auto-run the gate so the page is alive on first visit
    run();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="space-y-4">
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-bold text-paragon-800">Model Validation</h1>
          <p className="text-sm text-slate-500 mt-1">
            The twin is trusted only because it reproduces the closed-form laws of the
            ParaExpress report. Run the gate below — it executes live simulations and compares
            them against the theorems.
          </p>
        </div>
        <button className="btn-primary" onClick={run} disabled={busy}>
          {busy ? "Validating… (runs real simulations)" : "∑ Run validation gate"}
        </button>
      </header>

      {err && <div className="card border-red-300 bg-red-50 text-red-700 text-sm">{err}</div>}

      {rows && (
        <section className="card">
          <h3 className="font-bold text-paragon-700 text-sm mb-3">
            Analytic law vs. simulated statistic
            {rows.every((r) => r.pass)
              ? <span className="ml-3 text-accent-green">— ALL PASS ✓</span>
              : <span className="ml-3 text-accent-red">— FAILURES PRESENT</span>}
          </h3>
          <table className="table-base">
            <thead>
              <tr><th>Law</th><th>Reference</th><th>Analytic</th><th>Twin</th><th>Rel. error</th><th>Gate</th></tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.law}>
                  <td className="font-medium">{r.law}</td>
                  <td className="text-xs text-slate-400">{r.reference}</td>
                  <td className="font-mono text-xs">{r.analytic.toFixed(4)}</td>
                  <td className="font-mono text-xs">{r.simulated.toFixed(4)}</td>
                  <td className="font-mono text-xs">{(r.rel_err * 100).toFixed(2)}%</td>
                  <td>{r.pass ? <span className="text-accent-green font-bold">PASS</span> : <span className="text-accent-red font-bold">FAIL</span>}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="text-[11px] text-slate-400 mt-3">
            Erlang-C is compared against the day-average arrival rate while the twin runs
            nonhomogeneous (peaked) demand — the gate checks structural agreement of queueing
            behavior; exact-rate agreement is verified in the engine&apos;s unit tests
            (M/M/1 P(wait)=ρ, Erlang table values, routing vs brute force).
          </p>
        </section>
      )}

      <section className="grid grid-cols-2 gap-4">
        <div className="card">
          <h3 className="font-bold text-paragon-700 text-sm mb-3">Simulation formalisms in this twin</h3>
          <div className="space-y-3">
            {MODELS.map((m) => (
              <div key={m.name}>
                <div className="font-semibold text-sm text-slate-700">{m.name}</div>
                <p className="text-xs text-slate-500 leading-relaxed">{m.detail}</p>
              </div>
            ))}
          </div>
        </div>
        <div className="card">
          <h3 className="font-bold text-paragon-700 text-sm mb-3">Theorems wired into the product</h3>
          <table className="table-base">
            <thead><tr><th>Result</th><th>Claim</th><th>Where it is used</th></tr></thead>
            <tbody>
              {THEOREMS.map((t) => (
                <tr key={t.ref}>
                  <td className="font-mono text-xs text-paragon-600 whitespace-nowrap">{t.ref}</td>
                  <td className="text-xs">{t.claim}</td>
                  <td className="text-xs text-slate-500">{t.use}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="text-[11px] text-slate-400 mt-3">
            Full statements and proofs: <i>ParaExpress: A Mathematically Grounded Hyperlocal
            Fulfillment Network and Digital Twin Architecture</i>, ParaTech Platform R&amp;D, Aug 2026.
          </p>
        </div>
      </section>
    </div>
  );
}
