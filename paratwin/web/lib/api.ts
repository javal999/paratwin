// Typed command API — the single source of truth for the IPC contract.
// Argument keys are snake_case and MUST match the Rust command signatures
// (all Tauri commands are declared with rename_all = "snake_case", and the
// devserver mirrors the same names over HTTP).

import { invoke } from "./bridge";
import type {
  Frame,
  FulfillNode,
  PartnershipOption,
  RunResult,
  RunRow,
  Scenario,
  ScenarioRow,
  StoreRankRow,
  SweepPoint,
  ValidationRow,
} from "./types";

export const api = {
  // scenarios
  listScenarios: () => invoke<ScenarioRow[]>("list_scenarios"),
  getScenario: (id: number) => invoke<Scenario>("get_scenario", { id }),
  saveScenario: (id: number | null, scenario: Scenario) =>
    invoke<{ id: number }>("save_scenario", { id, scenario }),
  deleteScenario: (id: number) => invoke<void>("delete_scenario", { id }),
  presetScenario: (name: "baseline" | "pilot" | "scaleout") =>
    invoke<Scenario>("preset_scenario", { name }),
  autoLayout: (args: {
    metro_km: number;
    n_dark: number;
    n_store: number;
    flagship: boolean;
    orders_per_day: number;
  }) => invoke<FulfillNode[]>("auto_layout", args),

  // runs
  runSimulation: (scenario_id: number) =>
    invoke<{ run_id: number; result: RunResult }>("run_simulation", { scenario_id }),
  listRuns: () => invoke<RunRow[]>("list_runs"),
  getRun: (run_id: number) => invoke<RunResult>("get_run", { run_id }),
  getRunScenario: (run_id: number) => invoke<Scenario>("get_run_scenario", { run_id }),
  getFrames: (run_id: number, from_min: number, to_min: number) =>
    invoke<Frame[]>("get_frames", { run_id, from_min, to_min }),
  deleteRun: (run_id: number) => invoke<void>("delete_run", { run_id }),

  // decision tools
  validateMath: () => invoke<ValidationRow[]>("validate_math"),
  networkSweep: (scenario_id: number, n_from: number, n_to: number) =>
    invoke<SweepPoint[]>("network_sweep", { scenario_id, n_from, n_to }),
  comparePartnerships: (scenario_id: number) =>
    invoke<PartnershipOption[]>("compare_partnerships", { scenario_id }),
  rankStores: (scenario_id: number, take: number) =>
    invoke<StoreRankRow[]>("rank_stores", { scenario_id, take }),
};

export type Api = typeof api;
