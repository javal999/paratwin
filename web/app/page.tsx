"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { fmtIDR, fmtPct } from "@/lib/bridge";
import { api } from "@/lib/api";
import type { RunRow, ScenarioRow } from "@/lib/types";
import FlowJourney from "@/components/FlowJourney";
import { Info } from "@/components/Info";

export default function Dashboard() {
  const [runs, setRuns] = useState<RunRow[]>([]);
  const [scenarios, setScenarios] = useState<ScenarioRow[]>([]);
  const [offline, setOffline] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        setRuns(await api.listRuns());
        setScenarios(await api.listScenarios());
      } catch {
        setOffline(true);
      }
    })();
  }, []);

  const last = runs[0];

  return (
    <div className="space-y-6">
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-bold text-paragon-800">ParaExpress Digital Twin</h1>
          <p className="text-sm text-slate-500 mt-1">
            Simulate the hyperlocal fulfillment network before committing capital — DES +
            agent-based couriers + Monte Carlo, validated against the ParaExpress theorems.
          </p>
        </div>
        <Link href="/runner/" className="btn-primary">▶ Run a simulation</Link>
      </header>

      {offline && (
        <div className="card border-amber-300 bg-amber-50 text-amber-800 text-sm">
          Engine not reachable. In the desktop app this never happens; in browser dev mode,
          start <code className="font-mono">cargo run -p paratwin-devserver</code>.
        </div>
      )}

      <section className="grid grid-cols-4 gap-4">
        <div className="card">
          <div className="stat-label">Latest run SLA (PX1 ≤ 60 min)</div>
          <div className={`stat-value ${last && last.sla < 0.95 ? "text-accent-red" : ""}`}>
            {last ? fmtPct(last.sla) : "—"}
          </div>
          <div className="text-xs text-slate-400 mt-1">target ≥ 95%</div>
        </div>
        <div className="card">
          <div className="stat-label">Contribution / day</div>
          <div className="stat-value">{last ? fmtIDR(last.contribution_idr) : "—"}</div>
          <div className="text-xs text-slate-400 mt-1">margin − fulfillment costs</div>
        </div>
        <div className="card">
          <div className="stat-label">Network size</div>
          <div className="stat-value">{last ? `${last.n_nodes} nodes` : "—"}</div>
          <div className="text-xs text-slate-400 mt-1">stores + dark MFCs + flagship</div>
        </div>
        <div className="card">
          <div className="stat-label">Saved scenarios</div>
          <div className="stat-value">{scenarios.length}</div>
          <div className="text-xs text-slate-400 mt-1">{runs.length} runs recorded</div>
        </div>
      </section>

      <FlowJourney scenario={scenarios[0]?.scenario ?? null} />

      <section className="grid grid-cols-3 gap-4">
        {[
          {
            href: "/scenarios/",
            title: "1 · Configure",
            body: "Design the network: dark MFCs, consignment store nodes, couriers (Grab/GoSend), MOSTRANS mid-mile, demand and SLA.",
          },
          {
            href: "/runner/",
            title: "2 · Simulate",
            body: "Watch the twin run a full day of orders on the metro map — couriers, queues and deliveries in motion, with live KPIs.",
          },
          {
            href: "/decisions/",
            title: "3 · Decide",
            body: "Network-size sweep (Theorem 3), consignment activation ranking (Corollary 1), partnership comparison — with confidence intervals.",
          },
        ].map((c) => (
          <Link key={c.href} href={c.href} className="card hover:border-paragon-400 transition-colors">
            <div className="text-paragon-600 font-bold">{c.title}</div>
            <p className="text-sm text-slate-500 mt-2 leading-relaxed">{c.body}</p>
          </Link>
        ))}
      </section>

      <section className="card">
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-bold text-paragon-800">Recent runs</h2>
          <Link href="/results/" className="text-sm text-paragon-600 hover:underline">
            open results →
          </Link>
        </div>
        {runs.length === 0 ? (
          <p className="text-sm text-slate-400">
            No runs yet. Pick a preset scenario and press Run — the pilot preset simulates
            Phase 1 of the ParaExpress rollout (1 flagship + 2 dark MFCs + 15 store nodes).
          </p>
        ) : (
          <table className="table-base">
            <thead>
              <tr>
                <th>#</th><th>Scenario</th><th>When</th><th>Nodes</th>
                <th>Orders/day</th><th>SLA</th><th>Contribution/day</th>
              </tr>
            </thead>
            <tbody>
              {runs.slice(0, 8).map((r) => (
                <tr key={r.id}>
                  <td className="font-mono text-xs">{r.id}</td>
                  <td>{r.scenario_name}</td>
                  <td className="text-slate-400 text-xs">{r.created_at}</td>
                  <td>{r.n_nodes}</td>
                  <td>{Math.round(r.orders_per_day)}</td>
                  <td className={r.sla >= 0.95 ? "text-accent-green font-semibold" : "text-accent-red font-semibold"}>
                    {fmtPct(r.sla)}
                  </td>
                  <td>{fmtIDR(r.contribution_idr)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="card">
        <h2 className="font-bold text-paragon-800 mb-3">The four numbers that matter <Info text="Every dashboard in ParaTwin rolls up to these four. If all four are healthy, the network design works." /></h2>
        <div className="grid grid-cols-4 gap-4 text-sm">
          {[
            { t: "Promise kept (SLA)", d: "Of the orders we promised in 60 minutes, how many actually arrived in 60 minutes. Target: 95%. Below target = consumers stop trusting the Instant button." },
            { t: "Contribution / day", d: "Product margin minus everything it cost to fulfill: riders, pickers, rent, trucking, store fees. Positive and growing = the network pays for itself." },
            { t: "Promise shown", d: "Share of the metro (weighted by demand) where the Instant option could be offered at all. Low = coverage gaps; fix with more store nodes, not discounts." },
            { t: "Lost sales / day", d: "Orders we had to turn away — no stock nearby, or too far to reach in time. Every one is revenue handed to a competitor's dark store." },
          ].map((g) => (
            <div key={g.t} className="rounded-lg bg-slate-50 p-3">
              <div className="font-semibold text-paragon-700">{g.t}</div>
              <p className="text-xs text-slate-500 leading-relaxed mt-1">{g.d}</p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
