"use client";

import { useEffect, useMemo, useState } from "react";
import { fmtIDR, fmtPct, fmtMin } from "@/lib/bridge";
import { api } from "@/lib/api";
import { HowToRead } from "@/components/Info";
import CompareLab from "@/components/CompareLab";
import type { PartnershipOption, ScenarioRow, StoreRankRow, SweepPoint } from "@/lib/types";
import { ResponsiveLine } from "@nivo/line";

const PB = "#215aa9", PO = "#d97b29", PG = "#2e8b57", PR = "#c0392b";
const theme = {
  text: { fontSize: 11, fill: "#475569" },
  grid: { line: { stroke: "#e2e8f0", strokeWidth: 1 } },
};

export default function Decisions() {
  const [scenarios, setScenarios] = useState<ScenarioRow[]>([]);
  const [selId, setSelId] = useState<number | null>(null);
  const [sweep, setSweep] = useState<SweepPoint[] | null>(null);
  const [partners, setPartners] = useState<PartnershipOption[] | null>(null);
  const [stores, setStores] = useState<StoreRankRow[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState("");

  useEffect(() => {
    api.listScenarios().then((r) => {
      setScenarios(r);
      if (r.length) setSelId(r[0].id);
    }).catch(() => setErr("Engine not reachable."));
  }, []);

  const runSweep = async () => {
    if (selId === null) return;
    setBusy("sweep"); setErr("");
    try {
      setSweep(await api.networkSweep(selId, 2, 12));
    } catch (e) { setErr(String(e)); } finally { setBusy(null); }
  };
  const runPartners = async () => {
    if (selId === null) return;
    setBusy("partners"); setErr("");
    try {
      setPartners(await api.comparePartnerships(selId));
    } catch (e) { setErr(String(e)); } finally { setBusy(null); }
  };
  const runStores = async () => {
    if (selId === null) return;
    setBusy("stores"); setErr("");
    try {
      setStores(await api.rankStores(selId, 15));
    } catch (e) { setErr(String(e)); } finally { setBusy(null); }
  };

  const sweepChart = useMemo(() => {
    if (!sweep) return [];
    return [
      { id: "SLA %", color: PG, data: sweep.map((p) => ({ x: p.n_nodes, y: Math.round(p.sla * 1000) / 10 })) },
      { id: "contribution (IDR M/day)", color: PB, data: sweep.map((p) => ({ x: p.n_nodes, y: Math.round(p.contribution_idr / 1e6 * 10) / 10 })) },
      { id: "promise shown %", color: PO, data: sweep.map((p) => ({ x: p.n_nodes, y: Math.round(p.promise_shown * 1000) / 10 })) },
    ];
  }, [sweep]);

  const bestN = useMemo(() => {
    if (!sweep || sweep.length === 0) return null;
    const feasible = sweep.filter((p) => p.sla >= 0.93);
    const pool = feasible.length ? feasible : sweep;
    return pool.reduce((a, b) => (b.contribution_idr > a.contribution_idr ? b : a));
  }, [sweep]);

  return (
    <div className="space-y-4">
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-bold text-paragon-800">Decision Studio</h1>
          <p className="text-sm text-slate-500 mt-1">
            Twin-gated decisions: every recommendation below is produced by simulation +
            the report&apos;s theorems, not by static spreadsheets.
          </p>
        </div>
        <select className="input !w-80" value={selId ?? ""} onChange={(e) => { setSelId(parseInt(e.target.value)); setSweep(null); setPartners(null); setStores(null); }}>
          {scenarios.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
        </select>
      </header>

      {err && <div className="card border-red-300 bg-red-50 text-red-700 text-sm">{err}</div>}

      <div className="flex gap-2 flex-wrap items-center">
        <span className="text-xs font-semibold text-slate-400 uppercase tracking-wide">Which question?</span>
        {[
          ["compare-lab", "Which design wins overall?"],
          ["q-mfcs", "How many dark MFCs?"],
          ["q-partner", "Grab, dual-carrier or own fleet?"],
          ["q-stores", "Which stores to activate first?"],
        ].map(([id, label]) => (
          <button
            key={id}
            className="rounded-full border border-paragon-300 bg-paragon-50 text-paragon-700 px-3 py-1 text-xs font-semibold hover:bg-paragon-100"
            onClick={() => document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" })}
          >
            {label}
          </button>
        ))}
      </div>

      <div id="compare-lab"><CompareLab scenarios={scenarios} /></div>

      {/* 1: network size */}
      <section className="card" id="q-mfcs">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-bold text-paragon-700">1 · How many dark MFCs? <span className="text-xs font-normal text-slate-400">(Theorem 3: unique optimum; Prop. 2: SLA floor)</span></h3>
            <p className="text-sm text-slate-500 mt-1">Sweeps the dark-MFC count 2→12 with auto-layout, holding stores fixed. Short-horizon Monte Carlo per point.</p>
          </div>
          <button className="btn-primary" onClick={runSweep} disabled={busy !== null}>
            {busy === "sweep" ? "Sweeping…" : "Run sweep"}
          </button>
        </div>
        {sweep && (
          <>
            <div className="h-[300px] mt-3">
              <ResponsiveLine
                data={sweepChart} theme={theme} colors={[PG, PB, PO]}
                margin={{ top: 30, right: 20, bottom: 40, left: 48 }}
                axisBottom={{ legend: "dark MFCs (n)", legendOffset: 32, legendPosition: "middle" }}
                axisLeft={{ legend: "% / IDR M", legendOffset: -40, legendPosition: "middle" }}
                curve="monotoneX" pointSize={7} useMesh
                legends={[{ anchor: "top-left", direction: "row", itemWidth: 170, itemHeight: 14, translateY: -26, symbolSize: 8 }]}
                markers={bestN ? [{ axis: "x", value: bestN.n_nodes, lineStyle: { stroke: PR, strokeDasharray: "5 5" }, legend: `n* = ${bestN.n_nodes}`, textStyle: { fill: PR, fontSize: 11 } }] : []}
              />
            </div>
            <HowToRead>Each point is a full simulation of the network with that many dark MFCs. Read left to right: too few and the green SLA line sags (promise breaks); too many and the blue contribution line slides (rent and split stock eat the margin). The dashed line marks the sweet spot.</HowToRead>
            {bestN && (
              <div className="mt-2 rounded-lg bg-paragon-50 border border-paragon-200 px-4 py-3 text-sm text-paragon-800">
                <b>Recommendation:</b> {bestN.n_nodes} dark MFCs — SLA {fmtPct(bestN.sla)}, contribution {fmtIDR(bestN.contribution_idr)}/day,
                promise shown {fmtPct(bestN.promise_shown)}. The simulated curve reproduces the unimodal C(n) of Theorem 3:
                too few nodes breach the SLA floor (Prop. 2), too many fragment inventory and add fixed cost.
              </div>
            )}
          </>
        )}
      </section>

      {/* 2: partnerships */}
      <section className="card" id="q-partner">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-bold text-paragon-700">2 · Last-mile partnership <span className="text-xs font-normal text-slate-400">(Grab-only vs dual-carrier vs own fleet)</span></h3>
            <p className="text-sm text-slate-500 mt-1">Same scenario, three courier strategies; recommendation = max contribution among options meeting α.</p>
          </div>
          <button className="btn-primary" onClick={runPartners} disabled={busy !== null}>
            {busy === "partners" ? "Comparing…" : "Compare"}
          </button>
        </div>
        {partners && (
          <>
          <table className="table-base mt-3">
            <thead>
              <tr><th></th><th>Strategy</th><th>SLA</th><th>p95 CTD</th><th>Courier cost/day</th><th>Contribution/day</th><th></th></tr>
            </thead>
            <tbody>
              {partners.map((p) => (
                <tr key={p.name} className={p.recommended ? "bg-green-50" : ""}>
                  <td className="w-8 text-center">{p.recommended ? "✅" : ""}</td>
                  <td>
                    <div className="font-semibold">{p.name}</div>
                    <div className="text-xs text-slate-400">{p.description}</div>
                  </td>
                  <td className={p.sla >= 0.95 ? "text-accent-green font-semibold" : "text-accent-red font-semibold"}>{fmtPct(p.sla)}</td>
                  <td>{fmtMin(p.p95_min)}</td>
                  <td>{fmtIDR(p.courier_cost_idr)}</td>
                  <td className="font-semibold">{fmtIDR(p.contribution_idr)}</td>
                  <td className="text-xs text-slate-400">{p.recommended ? "recommended" : ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <HowToRead>All three strategies fulfill the same simulated orders. ✅ marks the best contribution among options that still meet the service target — note how “own fleet” buys a faster worst-case (p95) but costs more per day; that trade only pays once volume per rider is high enough.</HowToRead>
          </>
        )}
      </section>

      {/* 3: store activation */}
      <section className="card" id="q-stores">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-bold text-paragon-700">3 · Consignment store activation order <span className="text-xs font-normal text-slate-400">(greedy marginal coverage, re-ranked after each activation)</span></h3>
            <p className="text-sm text-slate-500 mt-1">Ranks candidate store locations by marginal covered demand — activate top of list first, re-rank after each wave.</p>
          </div>
          <button className="btn-primary" onClick={runStores} disabled={busy !== null}>
            {busy === "stores" ? "Ranking…" : "Rank stores"}
          </button>
        </div>
        {stores && (
          <div className="grid grid-cols-2 gap-6 mt-3">
            <table className="table-base">
              <thead><tr><th>Rank</th><th>Location (km)</th><th>Marginal coverage</th><th>Cumulative</th></tr></thead>
              <tbody>
                {stores.map((s) => (
                  <tr key={s.rank}>
                    <td className="font-bold text-paragon-600">#{s.rank}</td>
                    <td className="font-mono text-xs">({s.x_km.toFixed(1)}, {s.y_km.toFixed(1)})</td>
                    <td>{fmtPct(s.marginal_coverage)}</td>
                    <td>
                      <div className="flex items-center gap-2">
                        <div className="w-28 h-2 bg-slate-100 rounded-full overflow-hidden">
                          <div className="h-full bg-accent-teal" style={{ width: `${s.cumulative_coverage * 100}%` }} />
                        </div>
                        {fmtPct(s.cumulative_coverage)}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="text-sm text-slate-500 leading-relaxed bg-slate-50 rounded-lg p-4 self-start">
              <b className="text-paragon-700">Why greedy order matters.</b> The value of a store
              depends on which stores are already active, so ranking by <i>marginal</i> gain — not
              store size or standalone sales — beats ranking by any standalone measure. Marginals are
              recomputed after every activation rather than ranked once.
              <br /><br />
              <b className="text-paragon-700">No worst-case guarantee is claimed.</b> The (1−1/e)
              bound holds for uncapacitated nodes. These nodes have binding throughput caps, and
              under whole-order assignment capacity creates complementarity — a node&rsquo;s marginal
              value can <i>rise</i> as the active set grows — so coverage is not submodular and the
              bound does not transfer (Revision 5, C-07).
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
