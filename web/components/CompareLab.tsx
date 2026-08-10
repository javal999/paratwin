"use client";

// Comparison Lab: run several scenarios back-to-back through the full
// Monte Carlo engine and compare them side by side — table with best-value
// highlighting, SLA-vs-target bars, cost structure, and a plain-language
// verdict. Every run is persisted, so it also appears on the Results page.

import { useMemo, useState } from "react";
import { ResponsiveBar } from "@nivo/bar";
import { api } from "@/lib/api";
import { fmtIDR, fmtMin, fmtPct } from "@/lib/bridge";
import type { KpiSummary, RunResult, ScenarioRow } from "@/lib/types";
import { HowToRead } from "@/components/Info";

const PALETTE = ["#215aa9", "#d97b29", "#0f8b8d", "#7c5cbf"];
const theme = {
  text: { fontSize: 11, fill: "#475569" },
  grid: { line: { stroke: "#e2e8f0", strokeWidth: 1 } },
};

interface Entry {
  row: ScenarioRow;
  result: RunResult;
  color: string;
}

interface Metric {
  label: string;
  better: "high" | "low";
  get: (k: KpiSummary) => number;
  fmt: (v: number) => string;
  ci?: string;
  note?: string;
}

const METRICS: Metric[] = [
  { label: "Promise kept (SLA)", better: "high", get: (k) => k.sla_attainment, fmt: (v) => fmtPct(v), ci: "sla_attainment", note: "share delivered within 60 min" },
  { label: "Typical delivery", better: "low", get: (k) => k.p50_click_to_door_min, fmt: fmtMin, note: "median click-to-door" },
  { label: "Slowest 5%", better: "low", get: (k) => k.p95_click_to_door_min, fmt: fmtMin, ci: "p95_click_to_door_min", note: "the tail that breaks trust" },
  { label: "Orders fulfilled / day", better: "high", get: (k) => k.px1_served + k.px2_downgraded, fmt: (v) => v.toFixed(0) },
  { label: "Lost sales / day", better: "low", get: (k) => k.lost_sales, fmt: (v) => v.toFixed(0), note: "demand turned away" },
  { label: "Contribution / day", better: "high", get: (k) => k.contribution_idr, fmt: fmtIDR, ci: "contribution_idr", note: "margin − all fulfillment costs" },
  { label: "Contribution / order", better: "high", get: (k) => k.contribution_per_order, fmt: fmtIDR },
  { label: "Courier cost / day", better: "low", get: (k) => k.courier_cost_idr, fmt: fmtIDR, note: "scales with volume — healthy" },
  { label: "Fixed cost / day", better: "low", get: (k) => k.labor_cost_idr + k.rent_cost_idr + k.midmile_cost_idr, fmt: fmtIDR, note: "labor + rent + trucking: paid even on quiet days" },
  { label: "Promise shown", better: "high", get: (k) => k.promise_shown_rate, fmt: (v) => fmtPct(v), note: "metro coverage of the Instant button" },
  { label: "Picker workload", better: "high", get: (k) => k.picker_utilization, fmt: (v) => fmtPct(v), note: "low = over-staffed, >80% = queues" },
  { label: "Stock availability", better: "high", get: (k) => k.fill_rate, fmt: (v) => fmtPct(v), ci: "fill_rate" },
];

export default function CompareLab({ scenarios }: { scenarios: ScenarioRow[] }) {
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [progress, setProgress] = useState<{ done: number; total: number; name: string } | null>(null);
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [err, setErr] = useState("");

  const toggle = (id: number) => {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else if (next.size < 4) next.add(id);
    setSelected(next);
  };

  const run = async () => {
    const picks = scenarios.filter((s) => selected.has(s.id));
    if (picks.length < 2) return;
    setErr("");
    setEntries(null);
    try {
      const out: Entry[] = [];
      for (let i = 0; i < picks.length; i++) {
        setProgress({ done: i, total: picks.length, name: picks[i].name });
        const r = await api.runSimulation(picks[i].id);
        out.push({ row: picks[i], result: r.result, color: PALETTE[i % PALETTE.length] });
      }
      setEntries(out);
    } catch (e) {
      setErr(String(e));
    } finally {
      setProgress(null);
    }
  };

  const verdict = useMemo(() => {
    if (!entries || entries.length === 0) return null;
    const meeting = entries.filter(
      (e) => e.result.kpi.sla_attainment >= e.row.scenario.sla_target - 0.01,
    );
    const pool = meeting.length ? meeting : entries;
    const win = pool.reduce((a, b) =>
      b.result.kpi.contribution_idr > a.result.kpi.contribution_idr ? b : a,
    );
    const runnerUp = entries.filter((e) => e !== win)
      .sort((a, b) => b.result.kpi.contribution_idr - a.result.kpi.contribution_idr)[0];
    return { win, runnerUp, allMeet: meeting.length === entries.length };
  }, [entries]);

  const slaChart = useMemo(() => {
    if (!entries) return [];
    return entries.map((e) => ({
      scenario: short(e.row.name),
      "Promise kept %": +(e.result.kpi.sla_attainment * 100).toFixed(1),
      color: e.color,
    }));
  }, [entries]);

  const costChart = useMemo(() => {
    if (!entries) return [];
    return entries.map((e) => {
      const k = e.result.kpi;
      return {
        scenario: short(e.row.name),
        Courier: +(k.courier_cost_idr / 1e6).toFixed(1),
        Labor: +(k.labor_cost_idr / 1e6).toFixed(1),
        Rent: +(k.rent_cost_idr / 1e6).toFixed(1),
        "Mid-mile": +(k.midmile_cost_idr / 1e6).toFixed(1),
        "Store fees": +(k.store_fees_idr / 1e6).toFixed(1),
      };
    });
  }, [entries]);

  const contribChart = useMemo(() => {
    if (!entries) return [];
    return entries.map((e) => ({
      scenario: short(e.row.name),
      "Contribution (IDR M/day)": +(e.result.kpi.contribution_idr / 1e6).toFixed(1),
      color: e.color,
    }));
  }, [entries]);

  return (
    <section className="card border-2 border-paragon-200">
      <div className="flex items-start justify-between gap-6 flex-wrap">
        <div>
          <h3 className="font-bold text-paragon-700 text-lg">
            Comparison Lab{" "}
            <span className="text-xs font-normal text-slate-400">
              run up to 4 network designs head-to-head
            </span>
          </h3>
          <p className="text-sm text-slate-500 mt-1 max-w-xl">
            Pick 2–4 scenarios. Each one is simulated through the full Monte Carlo engine on the
            same demand assumptions, then compared metric by metric — the fair way to choose a
            network design.
          </p>
        </div>
        <button
          className="btn-primary"
          onClick={run}
          disabled={selected.size < 2 || progress !== null}
        >
          {progress
            ? `Simulating ${progress.done + 1}/${progress.total}…`
            : `⚖ Run & compare (${selected.size})`}
        </button>
      </div>

      {/* scenario picker chips */}
      <div className="flex gap-2 mt-4 flex-wrap">
        {scenarios.map((s, i) => {
          const on = selected.has(s.id);
          return (
            <button
              key={s.id}
              onClick={() => toggle(s.id)}
              className={`rounded-full px-3.5 py-1.5 text-xs font-semibold border transition-colors ${
                on
                  ? "bg-paragon-500 border-paragon-500 text-white"
                  : "bg-white border-slate-300 text-slate-600 hover:border-paragon-400"
              }`}
            >
              {on ? "✓ " : ""}{s.name}
              <span className={`ml-1.5 font-normal ${on ? "text-paragon-100" : "text-slate-400"}`}>
                {s.scenario.nodes.length} nodes · {Math.round(s.scenario.orders_per_day)}/d
              </span>
            </button>
          );
        })}
      </div>

      {progress && (
        <div className="mt-4 flex items-center gap-3">
          <div className="w-4 h-4 rounded-full border-2 border-paragon-300 border-t-paragon-600 animate-spin" />
          <div className="text-sm text-slate-600">Running “{progress.name}”…</div>
          <div className="flex-1 h-2 bg-slate-100 rounded-full overflow-hidden">
            <div
              className="h-full bg-paragon-500 transition-all"
              style={{ width: `${((progress.done + 0.5) / progress.total) * 100}%` }}
            />
          </div>
        </div>
      )}
      {err && <div className="mt-3 text-sm text-red-600">{err}</div>}

      {entries && verdict && (
        <div className="mt-5 space-y-5">
          {/* verdict */}
          <div className="rounded-lg bg-green-50 border border-green-200 px-4 py-3 text-sm text-green-900">
            <b>Verdict: “{verdict.win.row.name}”</b> — best contribution (
            {fmtIDR(verdict.win.result.kpi.contribution_idr)}/day) among designs
            {verdict.allMeet ? " (all meet the promise target)" : " that meet the promise target"}.
            {verdict.runnerUp && (
              <>
                {" "}Runner-up “{verdict.runnerUp.row.name}” trails by{" "}
                {fmtIDR(
                  verdict.win.result.kpi.contribution_idr -
                    verdict.runnerUp.result.kpi.contribution_idr,
                )}
                /day.
              </>
            )}{" "}
            Both runs are saved — open them on the Results page for the full drill-down.
          </div>

          {/* metric table */}
          <div className="overflow-x-auto">
            <table className="table-base">
              <thead>
                <tr>
                  <th className="w-52">Metric</th>
                  {entries.map((e) => (
                    <th key={e.row.id}>
                      <span className="inline-block w-2.5 h-2.5 rounded-full mr-1.5 align-middle" style={{ background: e.color }} />
                      {e.row.name}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {METRICS.map((m) => {
                  const vals = entries.map((e) => m.get(e.result.kpi));
                  const best = m.better === "high" ? Math.max(...vals) : Math.min(...vals);
                  return (
                    <tr key={m.label}>
                      <td>
                        <div className="font-medium">{m.label}</div>
                        {m.note && <div className="text-[10px] text-slate-400">{m.note}</div>}
                      </td>
                      {entries.map((e, i) => {
                        const isBest = vals[i] === best && new Set(vals).size > 1;
                        const ci = m.ci ? e.result.ci[m.ci] : undefined;
                        return (
                          <td key={e.row.id} className={isBest ? "bg-green-50 font-bold text-green-800" : ""}>
                            {m.fmt(vals[i])}
                            {isBest && " ★"}
                            {ci && (
                              <span className="block text-[10px] font-normal text-slate-400">
                                ± {m.ci === "contribution_idr" ? fmtIDR(ci.half_width) : m.fmt(ci.half_width)}
                              </span>
                            )}
                          </td>
                        );
                      })}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          <HowToRead>
            ★ marks the best value in each row (green = better). Read the top three rows for the
            consumer experience, the middle for volume, the bottom half for the economics. A design
            that wins contribution but loses “promise shown” is earning more from a smaller map —
            decide if that trade fits the growth plan.
          </HowToRead>

          {/* charts */}
          <div className="grid grid-cols-3 gap-4">
            <div className="card !shadow-none h-[300px]">
              <h4 className="font-bold text-paragon-700 text-xs">Promise kept vs 95% target</h4>
              <div className="h-[240px]">
                <ResponsiveBar
                  data={slaChart} keys={["Promise kept %"]} indexBy="scenario" theme={theme}
                  colors={({ data }) => (data as { color: string }).color}
                  margin={{ top: 12, right: 12, bottom: 56, left: 40 }}
                  minValue={Math.min(80, ...slaChart.map((d) => d["Promise kept %"])) - 2}
                  maxValue={100}
                  markers={[{ axis: "y", value: 95, lineStyle: { stroke: "#c0392b", strokeDasharray: "5 4", strokeWidth: 1.5 }, legend: "target", textStyle: { fontSize: 10, fill: "#c0392b" } }]}
                  labelTextColor="white" borderRadius={3} padding={0.4}
                  axisBottom={{ tickRotation: -18 }}
                />
              </div>
            </div>
            <div className="card !shadow-none h-[300px]">
              <h4 className="font-bold text-paragon-700 text-xs">Cost structure (IDR M/day)</h4>
              <div className="h-[240px]">
                <ResponsiveBar
                  data={costChart}
                  keys={["Courier", "Labor", "Rent", "Mid-mile", "Store fees"]}
                  indexBy="scenario" theme={theme}
                  colors={["#d97b29", "#215aa9", "#0f8b8d", "#2e8b57", "#94a3b8"]}
                  margin={{ top: 12, right: 12, bottom: 56, left: 40 }}
                  labelSkipHeight={12} labelTextColor="white" borderRadius={2} padding={0.4}
                  axisBottom={{ tickRotation: -18 }}
                  legends={[{ dataFrom: "keys", anchor: "bottom", direction: "row", translateY: 54, itemWidth: 62, itemHeight: 12, symbolSize: 9, itemTextColor: "#64748b" }]}
                />
              </div>
            </div>
            <div className="card !shadow-none h-[300px]">
              <h4 className="font-bold text-paragon-700 text-xs">Contribution (IDR M/day)</h4>
              <div className="h-[240px]">
                <ResponsiveBar
                  data={contribChart} keys={["Contribution (IDR M/day)"]} indexBy="scenario" theme={theme}
                  colors={({ data }) => (data as { color: string }).color}
                  margin={{ top: 12, right: 12, bottom: 56, left: 44 }}
                  labelTextColor="white" borderRadius={3} padding={0.4}
                  axisBottom={{ tickRotation: -18 }}
                />
              </div>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

function short(name: string): string {
  return name.length > 22 ? name.slice(0, 20) + "…" : name;
}
