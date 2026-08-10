"use client";

import { useEffect, useMemo, useState } from "react";
import { fmtIDR, fmtPct, fmtMin } from "@/lib/bridge";
import { api } from "@/lib/api";
import { HowToRead } from "@/components/Info";
import type { RunResult, RunRow, Scenario } from "@/lib/types";
import { ResponsiveLine } from "@nivo/line";
import { ResponsiveBar } from "@nivo/bar";
import { ResponsivePie } from "@nivo/pie";
import { ResponsiveHeatMap } from "@nivo/heatmap";

const PB = "#215aa9", PT = "#0f8b8d", PO = "#d97b29", PG = "#2e8b57", PR = "#c0392b";
const theme = {
  text: { fontSize: 11, fill: "#475569" },
  axis: { legend: { text: { fontSize: 11, fill: "#334155" } } },
  grid: { line: { stroke: "#e2e8f0", strokeWidth: 1 } },
};

export default function Results() {
  const [runs, setRuns] = useState<RunRow[]>([]);
  const [runId, setRunId] = useState<number | null>(null);
  const [res, setRes] = useState<RunResult | null>(null);
  const [sc, setSc] = useState<Scenario | null>(null);

  useEffect(() => {
    api.listRuns().then((r) => {
      setRuns(r);
      const q = new URLSearchParams(window.location.search).get("run");
      const id = q ? parseInt(q) : r[0]?.id;
      if (id) setRunId(id);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    if (runId === null) return;
    api.getRun(runId).then(setRes).catch(() => {});
    api.getRunScenario(runId).then(setSc).catch(() => {});
  }, [runId]);

  const k = res?.kpi;

  const hourlyData = useMemo(() => {
    if (!res) return [];
    return [
      {
        id: "orders/h",
        color: PB,
        data: res.hourly.map((h) => ({ x: h.hour, y: Math.round(h.orders * 10) / 10 })),
      },
    ];
  }, [res]);

  const slaData = useMemo(() => {
    if (!res) return [];
    return [
      {
        id: "on-time rate",
        color: PG,
        data: res.hourly.map((h) => ({ x: h.hour, y: Math.round(h.on_time_rate * 1000) / 10 })),
      },
      {
        id: "avg click-to-door (min)",
        color: PO,
        data: res.hourly.map((h) => ({ x: h.hour, y: Math.round(h.avg_ctd_min * 10) / 10 })),
      },
    ];
  }, [res]);

  const costData = useMemo(() => {
    if (!k) return [];
    return [
      { item: "Courier", value: Math.round(k.courier_cost_idr / 1e6 * 10) / 10, color: PO },
      { item: "Labor", value: Math.round(k.labor_cost_idr / 1e6 * 10) / 10, color: PB },
      { item: "Rent", value: Math.round(k.rent_cost_idr / 1e6 * 10) / 10, color: PT },
      { item: "Mid-mile", value: Math.round(k.midmile_cost_idr / 1e6 * 10) / 10, color: PG },
      { item: "Store fees", value: Math.round(k.store_fees_idr / 1e6 * 10) / 10, color: "#94a3b8" },
    ];
  }, [k]);

  const mixData = useMemo(() => {
    if (!k) return [];
    return [
      { id: "PX1 on-time", value: Math.round(k.px1_on_time), color: PG },
      { id: "PX1 late", value: Math.round(k.px1_served - k.px1_on_time), color: PR },
      { id: "PX2 downgraded", value: Math.round(k.px2_downgraded), color: PO },
      { id: "Lost", value: Math.round(k.lost_sales), color: "#94a3b8" },
    ].filter((d) => d.value > 0);
  }, [k]);

  const heatData = useMemo(() => {
    if (!res || !sc) return [];
    const cells = Math.round(sc.metro_km);
    // downsample to at most 20 columns for readability
    const stride = Math.max(1, Math.ceil(cells / 20));
    const rows: { id: string; data: { x: string; y: number | null }[] }[] = [];
    for (let zy = cells - 1; zy >= 0; zy -= stride) {
      const row: { x: string; y: number | null }[] = [];
      for (let zx = 0; zx < cells; zx += stride) {
        const z = res.zones.find((q) => q.zx === zx && q.zy === zy);
        row.push({ x: `${zx}`, y: z && z.on_time_rate >= 0 ? Math.round(z.on_time_rate * 100) : null });
      }
      rows.push({ id: `${zy}`, data: row });
    }
    return rows;
  }, [res, sc]);

  const repScatter = useMemo(() => {
    if (!res) return [];
    return [
      {
        id: "replications",
        color: PB,
        data: res.replication_kpis.map((r, i) => ({ x: i + 1, y: Math.round(r.sla_attainment * 1000) / 10 })),
      },
    ];
  }, [res]);

  return (
    <div className="space-y-4">
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-bold text-paragon-800">Results Dashboard</h1>
          <p className="text-sm text-slate-500 mt-1">
            Every number below is an average over independent simulated weeks, shown with its
            95% confidence range — “± X” means the truth is very likely inside that band.
          </p>
        </div>
        <select className="input !w-96" value={runId ?? ""} onChange={(e) => setRunId(parseInt(e.target.value))}>
          {runs.map((r) => (
            <option key={r.id} value={r.id}>
              #{r.id} — {r.scenario_name} ({r.created_at})
            </option>
          ))}
        </select>
      </header>

      {!res || !k ? (
        <div className="card text-sm text-slate-400">Run a simulation first, then explore it here.</div>
      ) : (
        <>
          <ExecSummary k={k} sc={sc} />
          <section className="grid grid-cols-4 gap-3">
            {[
              { l: "Promise kept (SLA)", v: fmtPct(k.sla_attainment), ci: res.ci["sla_attainment"], pctCi: true },
              { l: "Slowest 5% of deliveries", v: fmtMin(k.p95_click_to_door_min), ci: res.ci["p95_click_to_door_min"] },
              { l: "Contribution / day", v: fmtIDR(k.contribution_idr), ci: res.ci["contribution_idr"], idr: true },
              { l: "Stock availability", v: fmtPct(k.fill_rate), ci: res.ci["fill_rate"], pctCi: true },
            ].map((s) => (
              <div className="card !p-4" key={s.l}>
                <div className="stat-label">{s.l}</div>
                <div className="stat-value">{s.v}</div>
                {s.ci && (
                  <div className="text-[10px] text-slate-400">
                    ± {s.pctCi ? fmtPct(s.ci.half_width) : s.idr ? fmtIDR(s.ci.half_width) : s.ci.half_width.toFixed(1)} (CI95, n={res.replication_kpis.length})
                  </div>
                )}
              </div>
            ))}
          </section>

          <section className="grid grid-cols-2 gap-4">
            <div className="card h-[340px]">
              <h3 className="font-bold text-paragon-700 text-sm">Demand profile — orders per hour</h3>
              <div className="h-[260px]">
                <ResponsiveLine
                  data={hourlyData} theme={theme} colors={[PB]}
                  margin={{ top: 16, right: 18, bottom: 36, left: 44 }}
                  axisBottom={{ legend: "hour of day", legendOffset: 30, legendPosition: "middle" }}
                  axisLeft={{ legend: "orders", legendOffset: -38, legendPosition: "middle" }}
                  enablePoints={false} enableArea areaOpacity={0.12} curve="monotoneX"
                  enableGridX={false} isInteractive useMesh
                />
              </div>
              <HowToRead>Two demand peaks drive everything: lunch and the 19–21h live-commerce prime time. Staffing and stock must be sized for the evening peak, not the daily average.</HowToRead>
            </div>
            <div className="card h-[340px]">
              <h3 className="font-bold text-paragon-700 text-sm">Service by hour — on-time % and click-to-door</h3>
              <div className="h-[260px]">
                <ResponsiveLine
                  data={slaData} theme={theme} colors={[PG, PO]}
                  margin={{ top: 16, right: 18, bottom: 36, left: 44 }}
                  axisBottom={{ legend: "hour of day", legendOffset: 30, legendPosition: "middle" }}
                  axisLeft={{ legend: "% / minutes", legendOffset: -38, legendPosition: "middle" }}
                  enablePoints={false} curve="monotoneX" enableGridX={false} useMesh
                  legends={[{ anchor: "top-left", direction: "row", itemWidth: 150, itemHeight: 16, translateY: -14, symbolSize: 8 }]}
                />
              </div>
              <HowToRead>Green line near 100% all day = the promise holds even at rush hour. A dip during 17–20h means queues or traffic are eating the 60-minute budget exactly when most shoppers order.</HowToRead>
            </div>
            <div className="card h-[340px]">
              <h3 className="font-bold text-paragon-700 text-sm">Cost per day (IDR M)</h3>
              <div className="h-[260px]">
                <ResponsiveBar
                  data={costData} theme={theme} keys={["value"]} indexBy="item"
                  colors={({ data }) => (data as { color: string }).color}
                  margin={{ top: 16, right: 18, bottom: 36, left: 44 }}
                  axisLeft={{ legend: "IDR M / day", legendOffset: -38, legendPosition: "middle" }}
                  labelTextColor="white" borderRadius={3} padding={0.35}
                />
              </div>
              <HowToRead>Riders are usually the biggest bar — and they scale with orders, which is healthy. Courier includes the multi-drop same-day batches and Mid-mile is the routed MOSTRANS truck cost (CVRP: savings + 2-opt), not flat fees. Watch rent and labor: those are paid even on quiet days.</HowToRead>
            </div>
            <div className="card h-[340px]">
              <h3 className="font-bold text-paragon-700 text-sm">Order outcome mix (per day)</h3>
              <div className="h-[260px]">
                <ResponsivePie
                  data={mixData} theme={theme}
                  colors={{ datum: "data.color" }}
                  margin={{ top: 24, right: 90, bottom: 24, left: 90 }}
                  innerRadius={0.55} padAngle={1.5} cornerRadius={4}
                  arcLinkLabelsColor={{ from: "color" }} arcLinkLabelsTextColor="#475569"
                  arcLabelsTextColor="white"
                />
              </div>
              <HowToRead>Green should dominate. The grey slice (“Lost”) is demand we turned away — the clearest signal to add store nodes or stock in the areas shown red on the heatmap below.</HowToRead>
            </div>
          </section>

          <section className="grid grid-cols-2 gap-4">
            <div className="card h-[360px]">
              <h3 className="font-bold text-paragon-700 text-sm">Zone service heatmap — on-time % by km cell</h3>
              <div className="h-[290px]">
                <ResponsiveHeatMap
                  data={heatData} theme={theme}
                  margin={{ top: 10, right: 16, bottom: 40, left: 36 }}
                  colors={{ type: "diverging", scheme: "red_yellow_green", minValue: 40, maxValue: 100 }}
                  emptyColor="#f1f5f9" borderRadius={2}
                  axisBottom={{ legend: "km east", legendOffset: 32, legendPosition: "middle" }}
                  axisLeft={{ legend: "km north", legendOffset: -30, legendPosition: "middle" }}
                  enableLabels={false}
                  legends={[{ anchor: "bottom", translateX: 0, translateY: 36, length: 200, thickness: 8, direction: "row", title: "on-time % →" }]}
                />
              </div>
              <HowToRead>Each cell is 1 km² of the metro, colored by on-time %. Red/orange pockets = neighborhoods where the promise fails — the twin’s way of pointing at the next store-node activation. Grey = no instant orders there.</HowToRead>
            </div>
            <div className="card h-[360px]">
              <h3 className="font-bold text-paragon-700 text-sm">Replication stability — SLA per replication</h3>
              <div className="h-[290px]">
                <ResponsiveLine
                  data={repScatter} theme={theme} colors={[PB]}
                  margin={{ top: 16, right: 18, bottom: 40, left: 44 }}
                  axisBottom={{ legend: "replication", legendOffset: 32, legendPosition: "middle", tickValues: res.replication_kpis.map((_, i) => i + 1) }}
                  axisLeft={{ legend: "SLA %", legendOffset: -38, legendPosition: "middle" }}
                  enablePoints pointSize={8} pointColor={PB} lineWidth={0}
                  markers={[{ axis: "y", value: (sc?.sla_target ?? 0.95) * 100, lineStyle: { stroke: PR, strokeDasharray: "4 4" }, legend: "target α", textStyle: { fontSize: 10, fill: PR } }]}
                />
              </div>
              <HowToRead>Each dot is one independently simulated week. Tightly clustered dots above the dashed target = a robust design, not a lucky one. A single dot below target hints the design fails in bad weeks.</HowToRead>
            </div>
          </section>

          <section className="card">
            <h3 className="font-bold text-paragon-700 text-sm mb-3">Node performance — where the work lands</h3>
            <table className="table-base">
              <thead>
                <tr><th>Node</th><th>Type</th><th>Orders/day</th><th>Utilization</th><th>On-time</th><th>Stockout</th></tr>
              </thead>
              <tbody>
                {res.nodes.map((n) => (
                  <tr key={n.id}>
                    <td className="font-mono text-xs">#{n.id} ({n.x_km.toFixed(1)}, {n.y_km.toFixed(1)})</td>
                    <td>{n.kind === "Store" ? "Store node" : n.kind === "Dark" ? "Dark MFC" : "Flagship"}</td>
                    <td>{n.orders.toFixed(0)}</td>
                    <td>
                      <div className="flex items-center gap-2">
                        <div className="w-24 h-2 bg-slate-100 rounded-full overflow-hidden">
                          <div className="h-full bg-paragon-500" style={{ width: `${Math.min(100, n.utilization * 100)}%` }} />
                        </div>
                        {fmtPct(n.utilization)}
                      </div>
                    </td>
                    <td className={n.on_time_rate >= 0.95 ? "text-accent-green" : "text-accent-red"}>{fmtPct(n.on_time_rate)}</td>
                    <td>{fmtPct(n.stockout_rate)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        </>
      )}
    </div>
  );
}


function ExecSummary({ k, sc }: { k: import("@/lib/types").KpiSummary; sc: import("@/lib/types").Scenario | null }) {
  const target = sc?.sla_target ?? 0.95;
  const slaOk = k.sla_attainment >= target;
  const lostShare = k.orders_total > 0 ? k.lost_sales / k.orders_total : 0;
  const costs: [string, number][] = [
    ["riders", k.courier_cost_idr],
    ["pick staff", k.labor_cost_idr],
    ["rent", k.rent_cost_idr],
    ["MOSTRANS trucking", k.midmile_cost_idr],
    ["store fees", k.store_fees_idr],
  ];
  const topCost = costs.reduce((a, b) => (b[1] > a[1] ? b : a));

  let action: string;
  let tone: "good" | "warn" | "bad";
  if (!slaOk) {
    tone = "bad";
    action = `The promise is being missed — first check the Service-by-hour chart for the failing hours, then the heatmap for failing neighborhoods. Usual fixes: more pickers at the busiest node, or one more dark MFC near the red zones.`;
  } else if (lostShare > 0.02) {
    tone = "warn";
    action = `Coverage gap: ${Math.round(k.lost_sales)} orders/day (${fmtPct(lostShare)}) are turned away. The red cells on the heatmap show where — activating consignment stores there (Decision Studio → store ranking) converts that lost demand at near-zero fixed cost.`;
  } else if (k.picker_utilization < 0.25) {
    tone = "warn";
    action = `Staff are busy only ${fmtPct(k.picker_utilization)} of the time — the promise holds with room to spare. The margin lever here is trimming pickers per shift or absorbing more demand on the same network.`;
  } else if (k.picker_utilization > 0.8) {
    tone = "warn";
    action = `Pickers run at ${fmtPct(k.picker_utilization)} utilization — queues will form on any surge. Add staff at the hottest nodes before scaling demand.`;
  } else {
    tone = "good";
    action = `No structural bottleneck visible: promise met, little lost demand, staffing balanced. This design is ready for a bigger demand test or a head-to-head in the Comparison Lab.`;
  }

  const border = tone === "bad" ? "border-red-300 bg-red-50" : tone === "warn" ? "border-amber-300 bg-amber-50" : "border-green-300 bg-green-50";

  return (
    <section className={`card border-2 ${border}`}>
      <h2 className="font-bold text-paragon-800 mb-1.5">What this run says</h2>
      <p className="text-sm text-slate-700 leading-relaxed">
        This network keeps the {sc ? Math.round(sc.sla_min) : 60}-minute promise for{" "}
        <b>{fmtPct(k.sla_attainment)}</b> of instant orders ({slaOk ? "above" : "BELOW"} the {fmtPct(target, 0)} target),
        fulfilling <b>{Math.round(k.px1_served + k.px2_downgraded)} orders/day</b> and contributing{" "}
        <b>{fmtIDR(k.contribution_idr)}/day</b> ({fmtIDR(k.contribution_per_order)} per order) after all fulfillment
        costs — the largest being {topCost[0]} at {fmtIDR(topCost[1])}/day. Logistics run on routed plans, not
        assumptions: {k.midmile_km_per_day.toFixed(0)} truck-km/day of MOSTRANS replenishment and same-day batches
        averaging {k.px2_drops_per_batch.toFixed(1)} drops per rider ({fmtPct(k.px2_batch_savings_pct)} cheaper than
        solo trips). <b>{action}</b>
      </p>
    </section>
  );
}
