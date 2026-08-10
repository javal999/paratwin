#!/usr/bin/env node
// End-to-end test: exercises EVERY command of the ParaTwin contract against
// the devserver (which shares the host + engine code paths with the desktop
// app; all Tauri commands are declared rename_all="snake_case" so the two
// transports accept identical payloads).
//
// Usage: node scripts/e2e.mjs   (devserver must be running on :8787)

const BASE = "http://127.0.0.1:8787/rpc";
let failures = 0;
let passed = 0;

async function rpc(cmd, args = {}) {
  const res = await fetch(`${BASE}/${cmd}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args),
  });
  if (!res.ok) throw new Error(`${cmd}: HTTP ${res.status} ${await res.text()}`);
  return res.json();
}

function check(name, cond, detail = "") {
  if (cond) {
    passed++;
    console.log(`  ✓ ${name}`);
  } else {
    failures++;
    console.error(`  ✗ ${name} ${detail}`);
  }
}

function approx(a, b, tol) {
  return Math.abs(a - b) <= tol;
}

console.log("ParaTwin E2E — full command contract\n");

// ---------- scenarios ----------
console.log("scenarios:");
const scenarios = await rpc("list_scenarios");
check("list_scenarios returns seeded presets", scenarios.length >= 3);
check(
  "scenario rows have {id, name, scenario}",
  scenarios.every((r) => r.id && r.name && r.scenario && Array.isArray(r.scenario.nodes)),
);

const preset = await rpc("preset_scenario", { name: "pilot" });
check("preset_scenario(pilot) has flagship + stores",
  preset.nodes.some((n) => n.kind === "Flagship") &&
  preset.nodes.filter((n) => n.kind === "Store").length >= 10);

const layout = await rpc("auto_layout", {
  metro_km: 20, n_dark: 3, n_store: 8, flagship: true, orders_per_day: 1500,
});
check("auto_layout returns 12 nodes", layout.length === 12, `got ${layout.length}`);
{
  // separation invariant from the refactor: no two nodes within 1.1 km
  let minD = 1e9;
  for (let i = 0; i < layout.length; i++)
    for (let j = i + 1; j < layout.length; j++)
      minD = Math.min(minD, Math.hypot(layout[i].x_km - layout[j].x_km, layout[i].y_km - layout[j].y_km));
  check("auto_layout keeps >= 1.1 km node separation", minD >= 1.1, `min ${minD.toFixed(2)} km`);
}

// save / get / delete round trip
const toSave = structuredClone(preset);
toSave.name = "E2E temp scenario";
toSave.days = 2;
toSave.replications = 2;
const saved = await rpc("save_scenario", { id: null, scenario: toSave });
check("save_scenario returns id", Number.isInteger(saved.id));
const fetched = await rpc("get_scenario", { id: saved.id });
check("get_scenario round-trips name/days", fetched.name === "E2E temp scenario" && fetched.days === 2);

// validation errors must be rejected, not persisted
let rejected = false;
try {
  const bad = structuredClone(toSave);
  bad.nodes = [];
  await rpc("save_scenario", { id: null, scenario: bad });
} catch {
  rejected = true;
}
check("save_scenario rejects invalid scenario (no nodes)", rejected);

// ---------- simulation ----------
console.log("simulation:");
const t0 = Date.now();
const run = await rpc("run_simulation", { scenario_id: saved.id });
const dt = (Date.now() - t0) / 1000;
check(`run_simulation completes (${dt.toFixed(1)}s)`, Number.isInteger(run.run_id));
const k = run.result.kpi;
check("KPI: orders in sane range", k.orders_total > 500 && k.orders_total < 4000, `got ${k.orders_total}`);
check("KPI: SLA in [0,1]", k.sla_attainment >= 0 && k.sla_attainment <= 1);
check("KPI: p50 <= p95", k.p50_click_to_door_min <= k.p95_click_to_door_min);
check(
  "KPI: accounting identity holds",
  approx(
    k.contribution_idr,
    k.revenue_margin_idr - k.courier_cost_idr - k.labor_cost_idr - k.rent_cost_idr -
      k.midmile_cost_idr - k.store_fees_idr,
    1.0,
  ),
);
check("result has 24 hourly points", run.result.hourly.length === 24);
check("result has per-node stats", run.result.nodes.length === toSave.nodes.length);
check("result has CI map", "sla_attainment" in run.result.ci);
check("replication KPIs = R", run.result.replication_kpis.length === 2);
check("VRP: mid-mile routed km > 0", k.midmile_km_per_day > 0, `got ${k.midmile_km_per_day}`);
check("VRP: trucks per day >= 1", k.midmile_trucks_per_day >= 1);
check("VRP: batch savings in [0,1]", k.px2_batch_savings_pct >= 0 && k.px2_batch_savings_pct <= 1);
check("VRP: drops per batch sane", k.px2_drops_per_batch === 0 || (k.px2_drops_per_batch >= 1 && k.px2_drops_per_batch <= 8.01));

const runs = await rpc("list_runs");
check("list_runs includes new run", runs.some((r) => r.id === run.run_id));

const back = await rpc("get_run", { run_id: run.run_id });
check("get_run round-trips KPIs", approx(back.kpi.orders_total, k.orders_total, 1e-9));
const runSc = await rpc("get_run_scenario", { run_id: run.run_id });
check("get_run_scenario returns the source scenario", runSc.name === "E2E temp scenario");

// frames from RocksDB
const frames = await rpc("get_frames", { run_id: run.run_id, from_min: 0, to_min: 600 });
check("get_frames returns frames", frames.length > 400, `got ${frames.length}`);
check("frames are minute-ordered", frames.every((f, i) => i === 0 || f.minute >= frames[i - 1].minute));
check("frames carry queues for every node", frames[0].queues.length === toSave.nodes.length);
const windowed = await rpc("get_frames", { run_id: run.run_id, from_min: 100, to_min: 200 });
check("frame range query respects bounds",
  windowed.every((f) => f.minute >= 100 && f.minute <= 200) && windowed.length > 0);

// determinism: same scenario (same seed) -> identical KPIs
const run2 = await rpc("run_simulation", { scenario_id: saved.id });
check(
  "determinism: same seed reproduces KPIs exactly",
  JSON.stringify(run2.result.kpi) === JSON.stringify(run.result.kpi),
);

// ---------- decision tools ----------
console.log("decision tools:");
const sweep = await rpc("network_sweep", { scenario_id: saved.id, n_from: 2, n_to: 4 });
check("network_sweep returns 3 points", sweep.length === 3);
check("sweep points have KPIs", sweep.every((p) => p.sla > 0 && Number.isFinite(p.contribution_idr)));

const partners = await rpc("compare_partnerships", { scenario_id: saved.id });
check("compare_partnerships returns 3 options", partners.length === 3);
check("exactly one option recommended", partners.filter((p) => p.recommended).length === 1);

const stores = await rpc("rank_stores", { scenario_id: saved.id, take: 10 });
check("rank_stores returns ranked list", stores.length === 10);
check(
  "marginal coverage is non-increasing (submodularity)",
  stores.every((s, i) => i === 0 || s.marginal_coverage <= stores[i - 1].marginal_coverage + 1e-9),
);
check(
  "cumulative coverage is non-decreasing",
  stores.every((s, i) => i === 0 || s.cumulative_coverage >= stores[i - 1].cumulative_coverage - 1e-9),
);

// ---------- math gate ----------
console.log("math validation gate:");
const rows = await rpc("validate_math", {});
check("validation gate: all laws PASS", rows.every((r) => r.pass),
  rows.filter((r) => !r.pass).map((r) => r.law).join("; "));

// ---------- cleanup ----------
await rpc("delete_run", { run_id: run.run_id });
await rpc("delete_run", { run_id: run2.run_id });
await rpc("delete_scenario", { id: saved.id });
const after = await rpc("list_scenarios");
check("cleanup: temp scenario deleted", !after.some((r) => r.id === saved.id));

console.log(`\n${passed} passed, ${failures} failed`);
process.exit(failures === 0 ? 0 : 1);
