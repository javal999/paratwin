"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { fmtIDR, fmtPct, fmtMin } from "@/lib/bridge";
import { api } from "@/lib/api";
import type { Frame, RunResult, Scenario, ScenarioRow } from "@/lib/types";
import SimCanvas from "@/components/SimCanvas";
import { HowToRead, Stat } from "@/components/Info";
import Link from "next/link";

type Phase = "idle" | "running" | "loading-frames" | "ready";

const KIND_LABEL: Record<string, string> = {
  Store: "Consignment store node",
  Dark: "Dark MFC",
  Flagship: "Flagship MFC",
};
const KIND_EXPLAIN: Record<string, string> = {
  Store:
    "A partner store (via Exagon) holding a small ParaExpress shelf. Near-zero fixed cost — it earns a fee per order it picks.",
  Dark: "A rented pick-only space (150–300 m²) in a cheap but central location. The workhorse of the instant promise.",
  Flagship:
    "The largest node: widest assortment, backs up the whole metro for long-tail items and same-day batches.",
};

export default function Runner() {
  const [scenarios, setScenarios] = useState<ScenarioRow[]>([]);
  const [selId, setSelId] = useState<number | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [runId, setRunId] = useState<number | null>(null);
  const [result, setResult] = useState<RunResult | null>(null);
  const [scenario, setScenario] = useState<Scenario | null>(null);
  const [frames, setFrames] = useState<Frame[]>([]);
  const [cursor, setCursor] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(6);
  const [error, setError] = useState("");
  const [showCoverage, setShowCoverage] = useState(false);
  const [showMidMile, setShowMidMile] = useState(false);
  const [selNode, setSelNode] = useState<number | null>(null);
  const raf = useRef<number>(0);
  const lastT = useRef<number>(0);
  const runGen = useRef(0);

  useEffect(() => {
    api.listScenarios()
      .then((r) => {
        setScenarios(r);
        if (r.length > 0) setSelId(r[0].id);
      })
      .catch(() => setError("Engine not reachable."));
  }, []);

  // playback loop
  useEffect(() => {
    if (!playing || frames.length === 0) return;
    const tick = (t: number) => {
      if (lastT.current === 0) lastT.current = t;
      const dt = (t - lastT.current) / 1000;
      lastT.current = t;
      setCursor((c) => {
        const nc = c + dt * speed;
        return nc >= frames.length - 1 ? 0 : nc;
      });
      raf.current = requestAnimationFrame(tick);
    };
    raf.current = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(raf.current);
      lastT.current = 0;
    };
  }, [playing, speed, frames.length]);

  const run = async () => {
    if (selId === null) return;
    const gen = ++runGen.current;
    setPhase("running");
    setError("");
    setResult(null);
    setFrames([]);
    setPlaying(false);
    setSelNode(null);
    try {
      const sc = await api.getScenario(selId);
      if (gen !== runGen.current) return; // stopped
      setScenario(sc);
      const out = await api.runSimulation(selId);
      if (gen !== runGen.current) return; // stopped: discard result
      setRunId(out.run_id);
      setResult(out.result);
      setPhase("loading-frames");
      const fr = await api.getFrames(out.run_id, 0, 2880);
      if (gen !== runGen.current) return;
      setFrames(fr);
      setCursor(0);
      setPhase("ready");
      setPlaying(true);
    } catch (e) {
      if (gen === runGen.current) {
        setError(String(e));
        setPhase("idle");
      }
    }
  };

  const stop = () => {
    runGen.current++;         // discard any in-flight run
    setPlaying(false);
    setCursor(0);
    if (phase === "running" || phase === "loading-frames") setPhase("idle");
  };

  const k = result?.kpi;
  const slaOk = k && k.sla_attainment >= (scenario?.sla_target ?? 0.95);

  // live metrics from the replay frame under the cursor
  const live = useMemo(() => {
    if (frames.length === 0) return null;
    const f = frames[Math.min(Math.floor(cursor), frames.length - 1)];
    const served = f.orders_so_far;
    return {
      minute: f.minute,
      sla: served > 0 ? f.on_time_so_far / served : 1,
      avgCtd: served > 0 ? f.ctd_sum_min / served : 0,
      arrived: f.arrived_so_far,
      lost: f.lost_so_far,
      px2: f.px2_so_far,
      contrib: f.contrib_idr,
      promiseNow: f.promise_now,
      busy: f.busy_pickers,
      totalPickers: Math.max(1, f.total_pickers),
      waitedShare: served > 0 ? f.waited_so_far / served : 0,
      served,
    };
  }, [frames, cursor]);
  const showLive = live !== null && phase === "ready";

  // SLA-feasible service radius shown as coverage rings (Prop. 2 of the report)
  const coverageKm = useMemo(() => {
    if (!scenario) return 0;
    const tNode =
      scenario.pick_mean_min + scenario.courier.match_mean_min + 3.5;
    return Math.max(0, (scenario.courier.speed_kmh / 60) * (scenario.sla_min - tNode)) / 1.18;
  }, [scenario]);

  const nodeStat = selNode !== null && result ? result.nodes[selNode] : null;
  const nodeSpec = selNode !== null && scenario ? scenario.nodes[selNode] : null;

  return (
    <div className="space-y-4">
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-bold text-paragon-800">Simulation Runner</h1>
          <p className="text-sm text-slate-500 mt-1">
            Runs the full Monte Carlo, then replays one simulated day on the map — every rider,
            queue and delivery below is a real event from the twin, not an illustration.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <select
            className="input !w-72"
            value={selId ?? ""}
            onChange={(e) => setSelId(parseInt(e.target.value))}
          >
            {scenarios.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
          <button className="btn-primary" onClick={run} disabled={phase === "running" || selId === null}>
            {phase === "running" ? "Simulating…" : phase === "loading-frames" ? "Loading replay…" : "▶ Run"}
          </button>
          <button
            className="btn-danger"
            onClick={stop}
            disabled={phase === "idle" && !playing && cursor === 0}
            title="Stop the simulation run / replay"
          >
            ⏹ Stop
          </button>
        </div>
      </header>

      {error && <div className="card border-red-300 bg-red-50 text-red-700 text-sm">{error}</div>}

      {phase === "running" && (
        <div className="card flex items-center gap-4">
          <div className="w-5 h-5 rounded-full border-2 border-paragon-300 border-t-paragon-600 animate-spin" />
          <div className="text-sm text-slate-600">
            Simulating {scenario ? `${scenario.replications} independent ${scenario.days}-day weeks` : "…"} of
            orders, riders and replenishment…
          </div>
        </div>
      )}

      {k && scenario && (
        <>
          <section className="grid grid-cols-6 gap-3">
            {showLive && live ? (
              <>
                <Stat
                  label="Promise kept"
                  value={<span className="live-val">{fmtPct(live.sla)}</span>}
                  tone={live.sla >= (scenario.sla_target ?? 0.95) ? "good" : "bad"}
                  sub={`live at ${String(Math.floor((live.minute % 1440) / 60)).padStart(2, "0")}:${String(live.minute % 60).padStart(2, "0")} · week avg ${fmtPct(k.sla_attainment)}`}
                  help="Share of deliveries so far (in the replay) that arrived within the 60-minute promise. The subtitle shows the full Monte-Carlo week average."
                />
                <Stat
                  label="Avg delivery so far"
                  value={<span className="live-val">{fmtMin(live.avgCtd)}</span>}
                  sub={`week: p50 ${fmtMin(k.p50_click_to_door_min)} · p95 ${fmtMin(k.p95_click_to_door_min)}`}
                  help="Mean click-to-door of deliveries completed so far in this replay day."
                />
                <Stat
                  label="Orders today"
                  value={<span className="live-val">{live.arrived.toLocaleString()}</span>}
                  sub={`${live.served} delivered · ${live.px2} to same-day · ${live.lost} lost`}
                  help="Orders placed so far in the replay day, and where they stand right now."
                />
                <Stat
                  label="Contribution today"
                  value={<span className="live-val">{fmtIDR(live.contrib)}</span>}
                  tone={live.contrib < 0 ? "bad" : "default"}
                  sub={`week avg ${fmtIDR(k.contribution_idr)}/day`}
                  help="Running total: margin earned so far minus courier, batching, trucking and the day's fixed costs accrued minute by minute. Usually negative before the lunch peak — fixed costs run from 00:00."
                />
                <Stat
                  label="Promise shown"
                  value={<span className="live-val">{fmtPct(live.promiseNow)}</span>}
                  sub="of metro demand right now"
                  help="Share of demand (weighted) that could see the Instant button at this hour — dips when stock runs low or queues saturate."
                />
                <Stat
                  label="Pickers busy now"
                  value={<span className="live-val">{live.busy}/{live.totalPickers}</span>}
                  sub={`${fmtPct(live.waitedShare)} of orders waited · week ${fmtPct(k.picker_utilization)}`}
                  help="Pickers working at this minute across all facilities, and the share of orders that had to queue so far."
                />
              </>
            ) : (
              <>
                <Stat
                  label="Promise kept"
                  value={fmtPct(k.sla_attainment)}
                  tone={slaOk ? "good" : "bad"}
                  sub={`target ${fmtPct(scenario.sla_target, 0)} · ±${fmtPct(result!.ci["sla_attainment"]?.half_width ?? 0)}`}
                  help="Of all instant orders delivered, the share that arrived within the 60-minute promise."
                />
                <Stat
                  label="Typical delivery"
                  value={fmtMin(k.p50_click_to_door_min)}
                  sub={`slowest 5%: ${fmtMin(k.p95_click_to_door_min)}`}
                  help="Median click-to-door time across the simulated weeks."
                />
                <Stat
                  label="Orders / day"
                  value={Math.round(k.px1_served + k.px2_downgraded)}
                  sub={`${Math.round(k.lost_sales)} lost · ${Math.round(k.px2_downgraded)} moved to same-day`}
                  help="Instant orders fulfilled per day on average."
                />
                <Stat
                  label="Contribution / day"
                  value={fmtIDR(k.contribution_idr)}
                  tone={k.contribution_idr < 0 ? "bad" : "default"}
                  sub={`${fmtIDR(k.contribution_per_order)} per order`}
                  help="Product margin minus riders, batching, pickers, rent and routed trucking."
                />
                <Stat
                  label="Promise shown"
                  value={fmtPct(k.promise_shown_rate)}
                  sub="of metro demand can see 'Instant'"
                  help="Where (weighted by demand) the Instant button could be offered."
                />
                <Stat
                  label="Picker workload"
                  value={fmtPct(k.picker_utilization)}
                  sub={`${fmtPct(k.delay_prob)} of orders waited in queue`}
                  help="Average staff utilization across the simulated weeks."
                />
              </>
            )}
          </section>

          <section className="grid grid-cols-3 gap-3">
            <Stat
              label="Mid-mile routing (CVRP)"
              value={`${k.midmile_km_per_day.toFixed(0)} km/day`}
              sub={`${k.midmile_trucks_per_day.toFixed(1)} MOSTRANS trucks/day · savings + 2-opt routes`}
              help="Overnight replenishment truck routes computed by the capacitated VRP solver (Clarke-Wright savings + 2-opt + Or-opt), not a flat per-stop fee."
            />
            <Stat
              label="Same-day batches (PDP)"
              value={`${k.px2_drops_per_batch.toFixed(1)} drops/run`}
              sub={`${k.px2_rider_km_per_day.toFixed(0)} rider-km/day on PX2 batches`}
              help="Overflow orders are grouped into multi-pickup multi-delivery rider runs (max 8 drops, nearby facilities merged) instead of one trip per order."
            />
            <Stat
              label="Batching savings"
              value={fmtPct(k.px2_batch_savings_pct)}
              tone="good"
              sub="courier cost saved vs one solo trip per order"
              help="What the VRP batching earns: the routed batch cost compared against dispatching every same-day order as its own point-to-point trip."
            />
          </section>

          <section className="card">
            <div className="flex items-center gap-3 mb-3 flex-wrap">
              <button className="btn-ghost !px-3" onClick={() => setPlaying(!playing)} disabled={frames.length === 0}>
                {playing ? "⏸ Pause" : "▶ Play"}
              </button>
              <div className="flex-1 min-w-40">
                <input
                  type="range"
                  min={0}
                  max={Math.max(0, frames.length - 1)}
                  value={Math.floor(cursor)}
                  onChange={(e) => { setCursor(parseInt(e.target.value)); }}
                  className="w-full accent-paragon-500"
                />
                {frames.length > 0 && (
                  <div className="flex justify-between text-[9px] text-slate-400 -mt-1 px-0.5 select-none">
                    {Array.from({ length: Math.floor(frames.length / 360) + 1 }, (_, i) => {
                      const m = i * 360;
                      const hh = String(Math.floor((m % 1440) / 60)).padStart(2, "0");
                      return (
                        <span key={i} className={m % 1440 === 0 ? "font-bold text-slate-500" : ""}>
                          {m % 1440 === 0 ? `Day ${Math.floor(m / 1440) + 1}` : `${hh}:00`}
                        </span>
                      );
                    })}
                  </div>
                )}
              </div>
              <div className="flex items-center gap-1 text-xs text-slate-500">
                speed
                {[2, 6, 15, 40].map((s) => (
                  <button
                    key={s}
                    onClick={() => setSpeed(s)}
                    className={`px-2 py-1 rounded ${speed === s ? "bg-paragon-500 text-white font-bold" : "bg-slate-100 hover:bg-slate-200"}`}
                  >
                    {s}×
                  </button>
                ))}
              </div>
              <div className="flex items-center gap-3 text-xs border-l border-slate-200 pl-3">
                <label className="flex items-center gap-1.5 cursor-pointer text-slate-600">
                  <input type="checkbox" checked={showCoverage} onChange={(e) => setShowCoverage(e.target.checked)} className="accent-paragon-500" />
                  coverage rings (60-min reach)
                </label>
                <label className="flex items-center gap-1.5 cursor-pointer text-slate-600">
                  <input type="checkbox" checked={showMidMile} onChange={(e) => setShowMidMile(e.target.checked)} className="accent-paragon-500" />
                  mid-mile flows (MOSTRANS)
                </label>
              </div>
            </div>

            <div className="relative">
              {frames.length > 0 ? (
                <SimCanvas
                  metroKm={scenario.metro_km}
                  nodes={scenario.nodes}
                  frames={frames}
                  cursor={cursor}
                  coverageKm={coverageKm}
                  showCoverage={showCoverage}
                  showMidMile={showMidMile}
                  selectedNode={selNode}
                  onSelectNode={setSelNode}
                  heightClass="h-[64vh] min-h-[520px]"
                />
              ) : (
                <div className="text-sm text-slate-400 py-20 text-center">No frames.</div>
              )}

              {selNode !== null && nodeSpec && (
                <aside className="absolute top-3 left-3 w-64 rounded-xl border border-slate-200/80 bg-white/95 backdrop-blur shadow-xl p-4">
                  <div className="flex items-center justify-between">
                    <div className="font-bold text-paragon-800 text-sm">
                      Node #{nodeSpec.id}
                    </div>
                    <button className="text-slate-400 hover:text-slate-600 text-sm" onClick={() => setSelNode(null)}>✕</button>
                  </div>
                  <div className="text-xs font-semibold mt-0.5" style={{ color: nodeSpec.kind === "Flagship" ? "#d97b29" : nodeSpec.kind === "Dark" ? "#215aa9" : "#0f8b8d" }}>
                    {KIND_LABEL[nodeSpec.kind]}
                  </div>
                  <p className="text-[11px] text-slate-500 leading-relaxed mt-2">
                    {KIND_EXPLAIN[nodeSpec.kind]}
                  </p>
                  {nodeStat && (
                    <dl className="mt-3 space-y-2 text-xs">
                      {[
                        ["Orders / day", nodeStat.orders.toFixed(0)],
                        ["On-time rate", fmtPct(nodeStat.on_time_rate)],
                        ["Staff utilization", fmtPct(nodeStat.utilization)],
                        ["Stock-out rate", fmtPct(nodeStat.stockout_rate)],
                        ["Pickers", String(nodeSpec.pickers)],
                        ["Assortment coverage", fmtPct(nodeSpec.assortment_cover, 0)],
                      ].map(([l, v]) => (
                        <div key={l} className="flex justify-between border-b border-slate-100 pb-1">
                          <dt className="text-slate-500">{l}</dt>
                          <dd className="font-semibold text-slate-700">{v}</dd>
                        </div>
                      ))}
                    </dl>
                  )}
                </aside>
              )}
            </div>

            <div className="flex gap-5 mt-2.5 text-xs text-slate-500 flex-wrap items-center">
              <span className="font-semibold text-slate-400 uppercase tracking-wide text-[10px]">Legend</span>
              <span><span className="inline-block w-3 h-3 rounded-full align-middle mr-1.5" style={{ background: "#d97b29", boxShadow: "0 0 0 2px white, 0 0 0 3px #d97b29" }} /> Flagship</span>
              <span><span className="inline-block w-3 h-3 rounded-full align-middle mr-1.5 bg-white" style={{ boxShadow: "inset 0 0 0 2px #215aa9" }} /> Dark MFC</span>
              <span><span className="inline-block w-3 h-3 rounded-full align-middle mr-1.5 bg-white" style={{ boxShadow: "inset 0 0 0 2px #0f8b8d" }} /> Store node</span>
              <span className="text-slate-300">|</span>
              <span>scooter <span className="text-slate-400">grey</span> = to pickup · <span style={{ color: "#d97b29" }}>orange</span> = carrying</span>
              <span>house pin <span style={{ color: "#2e8b57" }}>green</span> = on-time · <span style={{ color: "#c0392b" }}>red</span> = late</span>
              <span>truck = MOSTRANS mid-mile</span>
              <span className="ml-auto text-slate-400">click any facility to inspect · badge = live queue</span>
            </div>

            <HowToRead>
              The map replays one simulated day on a stylized metro (roads, river, parks; blue
              shading = where consumers order). Circular pins are fulfillment points, scooters are
              live riders, house pins are deliveries landing. Turn on <b>coverage rings</b> to see
              how far each location can honor the 60-minute promise, and <b>mid-mile flows</b> to
              watch the overnight MOSTRANS trucks that stocked the shelves before demand woke up.
            </HowToRead>
          </section>

          <div className="flex justify-end">
            <Link className="btn-ghost" href={`/results/?run=${runId}`}>
              Full results dashboard →
            </Link>
          </div>
        </>
      )}
    </div>
  );
}
