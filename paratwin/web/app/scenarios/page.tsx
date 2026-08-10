"use client";

import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api";
import type { FulfillNode, Scenario, ScenarioRow } from "@/lib/types";
import MapPreview from "@/components/MapPreview";

function Num({
  label, value, onChange, step = 1, min, max, suffix, help,
}: {
  label: string; value: number; onChange: (v: number) => void;
  step?: number; min?: number; max?: number; suffix?: string; help?: string;
}) {
  return (
    <div>
      <label className="label" title={help}>{label}{suffix ? <span className="text-slate-400 normal-case"> ({suffix})</span> : null}</label>
      <input
        className="input"
        type="number"
        value={Number.isFinite(value) ? value : 0}
        step={step} min={min} max={max}
        onChange={(e) => onChange(parseFloat(e.target.value))}
      />
    </div>
  );
}

function Toggle({ label, value, onChange, help }: {
  label: string; value: boolean; onChange: (v: boolean) => void; help?: string;
}) {
  return (
    <button
      type="button"
      title={help}
      onClick={() => onChange(!value)}
      className={`flex items-center justify-between w-full rounded-lg border px-3 py-2 text-sm ${
        value ? "border-paragon-400 bg-paragon-50 text-paragon-700 font-semibold" : "border-slate-300 text-slate-500"
      }`}
    >
      {label}
      <span className={`w-9 h-5 rounded-full relative transition-colors ${value ? "bg-paragon-500" : "bg-slate-300"}`}>
        <span className={`absolute top-0.5 w-4 h-4 bg-white rounded-full transition-all ${value ? "left-[18px]" : "left-0.5"}`} />
      </span>
    </button>
  );
}

export default function Scenarios() {
  const [rows, setRows] = useState<ScenarioRow[]>([]);
  const [sel, setSel] = useState<number | null>(null);
  const [sc, setSc] = useState<Scenario | null>(null);
  const [dirty, setDirty] = useState(false);
  const [layout, setLayout] = useState({ n_dark: 2, n_store: 15, flagship: true });
  const [msg, setMsg] = useState("");

  const refresh = async (keepSel = true) => {
    const r = await api.listScenarios();
    setRows(r);
    if (!keepSel || sel === null) {
      if (r.length > 0) { setSel(r[0].id); setSc(r[0].scenario); }
    }
  };
  useEffect(() => { refresh(false).catch(() => setMsg("Engine not reachable")); /* eslint-disable-next-line */ }, []);

  const pick = (row: ScenarioRow) => { setSel(row.id); setSc(structuredClone(row.scenario)); setDirty(false); };

  const set = (patch: Partial<Scenario>) => { if (sc) { setSc({ ...sc, ...patch }); setDirty(true); } };
  const setCourier = (patch: Partial<Scenario["courier"]>) =>
    sc && set({ courier: { ...sc.courier, ...patch } });
  const setMid = (patch: Partial<Scenario["midmile"]>) =>
    sc && set({ midmile: { ...sc.midmile, ...patch } });

  const counts = useMemo(() => {
    const c = { Store: 0, Dark: 0, Flagship: 0 } as Record<string, number>;
    sc?.nodes.forEach((n) => (c[n.kind] += 1));
    return c;
  }, [sc]);

  const autoLayout = async () => {
    if (!sc) return;
    const nodes = await api.autoLayout({
      metro_km: sc.metro_km,
      n_dark: layout.n_dark,
      n_store: layout.n_store,
      flagship: layout.flagship,
      orders_per_day: sc.orders_per_day,
    });
    set({ nodes });
    setMsg(`Auto-layout: ${nodes.length} nodes placed by greedy coverage (Cor. 1)`);
  };

  const save = async (asNew = false) => {
    if (!sc) return;
    const id = await api.saveScenario(asNew ? null : sel, sc);
    setMsg(asNew ? `Saved as new scenario #${id.id}` : "Saved");
    setDirty(false);
    await refresh();
    if (asNew) setSel(id.id);
  };

  const addPreset = async (name: string) => {
    const p = await api.presetScenario(name as "baseline" | "pilot" | "scaleout");
    const id = await api.saveScenario(null, p);
    await refresh(false);
    setSel(id.id); setSc(p); setMsg(`Preset "${p.name}" added`);
  };

  const del = async () => {
    if (sel === null) return;
    await api.deleteScenario(sel);
    setSel(null); setSc(null);
    await refresh(false);
  };

  return (
    <div className="space-y-4">
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-bold text-paragon-800">Scenario Builder</h1>
          <p className="text-sm text-slate-500 mt-1">
            Configure the network, partners and demand. Every field maps to a parameter of the
            formal model in the ParaExpress report.
          </p>
        </div>
        <div className="flex gap-2">
          <button className="btn-ghost" onClick={() => addPreset("baseline")}>+ Phase 0</button>
          <button className="btn-ghost" onClick={() => addPreset("pilot")}>+ Phase 1 pilot</button>
          <button className="btn-ghost" onClick={() => addPreset("scaleout")}>+ Phase 2 scale-out</button>
        </div>
      </header>

      {msg && <div className="text-sm text-paragon-600">{msg}</div>}

      <div className="grid grid-cols-[240px_1fr] gap-4">
        {/* list */}
        <div className="card p-0 overflow-hidden self-start">
          {rows.map((r) => (
            <button
              key={r.id}
              onClick={() => pick(r)}
              className={`block w-full text-left px-4 py-3 border-b border-slate-100 text-sm ${
                sel === r.id ? "bg-paragon-50 border-l-4 border-l-paragon-500 font-semibold text-paragon-700" : "hover:bg-slate-50"
              }`}
            >
              <div className="truncate">{r.name}</div>
              <div className="text-[10px] text-slate-400">{r.scenario.nodes.length} nodes · {Math.round(r.scenario.orders_per_day)} orders/d</div>
            </button>
          ))}
          {rows.length === 0 && <div className="p-4 text-sm text-slate-400">No scenarios yet — add a preset.</div>}
        </div>

        {/* editor */}
        {sc ? (
          <div className="space-y-4">
            <div className="card">
              <div className="flex items-center justify-between mb-4">
                <input
                  className="input !text-base font-semibold max-w-md"
                  value={sc.name}
                  onChange={(e) => set({ name: e.target.value })}
                />
                <div className="flex gap-2">
                  <button className="btn-primary" onClick={() => save(false)} disabled={!dirty}>Save</button>
                  <button className="btn-ghost" onClick={() => save(true)}>Save as new</button>
                  <button className="btn-danger" onClick={del}>Delete</button>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-6">
                <div>
                  <h3 className="font-bold text-paragon-700 text-sm mb-3">Demand &amp; promise (Def. 2–3)</h3>
                  <div className="grid grid-cols-2 gap-3">
                    <Num label="Orders / day (Λ)" value={sc.orders_per_day} onChange={(v) => set({ orders_per_day: v })} step={100} min={50} />
                    <Num label="Metro size" suffix="km" value={sc.metro_km} onChange={(v) => set({ metro_km: v })} step={1} min={5} max={40} />
                    <Num label="SLA τ" suffix="min" value={sc.sla_min} onChange={(v) => set({ sla_min: v })} step={5} min={20} />
                    <Num label="SLA target α" value={sc.sla_target} onChange={(v) => set({ sla_target: v })} step={0.01} min={0.5} max={0.99} />
                    <Num label="AOV" suffix="IDR" value={sc.aov_idr} onChange={(v) => set({ aov_idr: v })} step={10000} />
                    <Num label="Gross margin" value={sc.margin_pct} onChange={(v) => set({ margin_pct: v })} step={0.05} min={0.1} max={0.7} />
                  </div>
                </div>

                <div>
                  <h3 className="font-bold text-paragon-700 text-sm mb-3">Node operations (Prop. 3–4)</h3>
                  <div className="grid grid-cols-2 gap-3">
                    <Num label="Pick mean" suffix="min" value={sc.pick_mean_min} onChange={(v) => set({ pick_mean_min: v })} step={0.5} min={1} />
                    <Num label="Routing wave" suffix="s" value={sc.wave_seconds} onChange={(v) => set({ wave_seconds: v })} step={5} min={5} max={60} />
                    <Num label="POS post latency" suffix="min" value={sc.pos_latency_min} onChange={(v) => set({ pos_latency_min: v })} step={1} min={0} />
                    <Num label="Store ATP buffer s" value={sc.store_buffer} onChange={(v) => set({ store_buffer: v })} step={1} min={0} />
                    <Num label="Picker wage" suffix="IDR/h" value={sc.wage_per_hour} onChange={(v) => set({ wage_per_hour: v })} step={1000} />
                    <Num label="Downgrade accept" value={sc.downgrade_accept} onChange={(v) => set({ downgrade_accept: v })} step={0.05} min={0} max={1} />
                  </div>
                </div>

                <div>
                  <h3 className="font-bold text-paragon-700 text-sm mb-3">Last mile — Grab / GoSend</h3>
                  <div className="grid grid-cols-2 gap-3">
                    <Num label="Rider speed" suffix="km/h" value={sc.courier.speed_kmh} onChange={(v) => setCourier({ speed_kmh: v })} step={1} min={10} />
                    <Num label="Match mean" suffix="min" value={sc.courier.match_mean_min} onChange={(v) => setCourier({ match_mean_min: v })} step={0.5} min={1} />
                    <Num label="Fee base" suffix="IDR" value={sc.courier.fee_base} onChange={(v) => setCourier({ fee_base: v })} step={500} />
                    <Num label="Fee per km" suffix="IDR" value={sc.courier.fee_per_km} onChange={(v) => setCourier({ fee_per_km: v })} step={100} />
                    <Num label="Delivery fee charged" suffix="IDR" value={sc.delivery_fee_idr} onChange={(v) => set({ delivery_fee_idr: v })} step={1000} />
                    <Num label="Primary fail prob" value={sc.courier.primary_fail_prob} onChange={(v) => setCourier({ primary_fail_prob: v })} step={0.01} min={0} max={0.5} />
                    <div className="col-span-2 grid grid-cols-2 gap-2">
                      <Toggle label="Dual carrier failover" value={sc.courier.dual_carrier} onChange={(v) => setCourier({ dual_carrier: v })} help="ShipGo+ rate shop: Grab primary, GoSend failover" />
                      <Toggle label="Courier pre-matching" value={sc.courier.prematch} onChange={(v) => setCourier({ prematch: v })} help="Book rider at pick start (overlap clocks)" />
                    </div>
                  </div>
                </div>

                <div>
                  <h3 className="font-bold text-paragon-700 text-sm mb-3">Mid-mile — MOSTRANS &amp; economics</h3>
                  <div className="grid grid-cols-2 gap-3">
                    <Num label="Wave period" suffix="days" value={sc.midmile.period_days} onChange={(v) => setMid({ period_days: v })} step={0.5} min={0.5} />
                    <Num label="OTIF" value={sc.midmile.otif} onChange={(v) => setMid({ otif: v })} step={0.01} min={0.5} max={1} />
                    <Num label="Truck fixed / wave" suffix="IDR" value={sc.midmile.truck_fixed_idr} onChange={(v) => setMid({ truck_fixed_idr: v })} step={25000} help="Fixed cost per MOSTRANS truck dispatched in a wave (CVRP decides how many trucks)" />
                    <Num label="Truck cost per km" suffix="IDR" value={sc.midmile.truck_per_km_idr} onChange={(v) => setMid({ truck_per_km_idr: v })} step={500} />
                    <Num label="Truck capacity" suffix="units" value={sc.midmile.truck_capacity_units} onChange={(v) => setMid({ truck_capacity_units: v })} step={50} />
                    <Num label="Rent dark MFC" suffix="IDR M/mo" value={sc.rent_dark_m} onChange={(v) => set({ rent_dark_m: v })} step={5} />
                    <Num label="Rent flagship" suffix="IDR M/mo" value={sc.rent_flag_m} onChange={(v) => set({ rent_flag_m: v })} step={5} />
                    <Num label="Store fee / order" suffix="IDR" value={sc.store_fee_per_order} onChange={(v) => set({ store_fee_per_order: v })} step={500} />
                  </div>
                </div>

                <div>
                  <h3 className="font-bold text-paragon-700 text-sm mb-3">Monte Carlo</h3>
                  <div className="grid grid-cols-3 gap-3">
                    <Num label="Days" value={sc.days} onChange={(v) => set({ days: Math.max(1, Math.round(v)) })} step={1} min={1} max={30} />
                    <Num label="Replications" value={sc.replications} onChange={(v) => set({ replications: Math.max(1, Math.round(v)) })} step={1} min={1} max={50} />
                    <Num label="Seed" value={sc.seed} onChange={(v) => set({ seed: Math.round(v) })} step={1} />
                  </div>
                </div>
              </div>
            </div>

            {/* network */}
            <div className="card">
              <div className="flex items-center justify-between mb-3 flex-wrap gap-3">
                <h3 className="font-bold text-paragon-700 text-sm whitespace-nowrap">
                  Network — {sc.nodes.length} nodes
                  <span className="ml-3 text-xs font-normal text-slate-500">
                    {counts.Flagship} flagship · {counts.Dark} dark MFC · {counts.Store} store nodes
                  </span>
                </h3>
                <div className="flex items-end gap-2">
                  <Num label="Dark MFCs" value={layout.n_dark} onChange={(v) => setLayout({ ...layout, n_dark: Math.max(0, Math.round(v)) })} min={0} max={20} />
                  <Num label="Store nodes" value={layout.n_store} onChange={(v) => setLayout({ ...layout, n_store: Math.max(0, Math.round(v)) })} min={0} max={60} />
                  <div className="w-36">
                    <Toggle label="Flagship" value={layout.flagship} onChange={(v) => setLayout({ ...layout, flagship: v })} />
                  </div>
                  <button className="btn-primary whitespace-nowrap" onClick={autoLayout}>
                    ⚡ Auto-layout (greedy coverage)
                  </button>
                </div>
              </div>
              <MapPreview metroKm={sc.metro_km} nodes={sc.nodes} />
              <p className="text-xs text-slate-400 mt-2">
                Placement uses the greedy of Theorem 2 on the demand field — each node is dropped
                where it adds the largest marginal covered demand, with marginals recomputed after
                every activation. No worst-case guarantee applies once node throughput caps bind.
              </p>
            </div>
          </div>
        ) : (
          <div className="card text-sm text-slate-400">Select or create a scenario.</div>
        )}
      </div>
    </div>
  );
}
