// TypeScript mirror of the Rust engine model (engine/src/model.rs)

export type NodeKind = "Store" | "Dark" | "Flagship";

export interface FulfillNode {
  id: number;
  kind: NodeKind;
  x_km: number;
  y_km: number;
  pickers: number;
  assortment_cover: number;
  base_stock_days: number;
}

export interface CourierCfg {
  speed_kmh: number;
  match_mean_min: number;
  match_cv: number;
  primary_fail_prob: number;
  failover_extra_min: number;
  dual_carrier: boolean;
  fee_base: number;
  fee_per_km: number;
  prematch: boolean;
}

export interface MidMileCfg {
  period_days: number;
  otif: number;
  cost_per_drop: number;
  truck_fixed_idr: number;
  truck_per_km_idr: number;
  truck_capacity_units: number;
}

export interface Scenario {
  name: string;
  metro_km: number;
  orders_per_day: number;
  days: number;
  replications: number;
  seed: number;
  sla_min: number;
  sla_target: number;
  wave_seconds: number;
  pick_mean_min: number;
  aov_idr: number;
  margin_pct: number;
  delivery_fee_idr: number;
  downgrade_accept: number;
  wage_per_hour: number;
  rent_dark_m: number;
  rent_flag_m: number;
  store_fee_per_order: number;
  pos_latency_min: number;
  store_buffer: number;
  courier: CourierCfg;
  midmile: MidMileCfg;
  nodes: FulfillNode[];
}

export interface KpiSummary {
  orders_total: number;
  px1_served: number;
  px1_on_time: number;
  px2_downgraded: number;
  lost_sales: number;
  oversell_events: number;
  sla_attainment: number;
  p50_click_to_door_min: number;
  p95_click_to_door_min: number;
  avg_pick_wait_min: number;
  delay_prob: number;
  avg_courier_match_min: number;
  avg_travel_min: number;
  avg_distance_km: number;
  fill_rate: number;
  revenue_margin_idr: number;
  courier_cost_idr: number;
  labor_cost_idr: number;
  rent_cost_idr: number;
  midmile_cost_idr: number;
  store_fees_idr: number;
  contribution_idr: number;
  contribution_per_order: number;
  promise_shown_rate: number;
  picker_utilization: number;
  midmile_km_per_day: number;
  midmile_trucks_per_day: number;
  px2_rider_km_per_day: number;
  px2_drops_per_batch: number;
  px2_batch_savings_pct: number;
}

export interface HourPoint {
  hour: number;
  orders: number;
  on_time_rate: number;
  avg_ctd_min: number;
  queue_len: number;
}

export interface NodeStat {
  id: number;
  kind: NodeKind;
  x_km: number;
  y_km: number;
  orders: number;
  utilization: number;
  on_time_rate: number;
  stockout_rate: number;
}

export interface ZoneStat {
  zx: number;
  zy: number;
  demand: number;
  on_time_rate: number;
  covered: boolean;
}

export interface CiStat {
  mean: number;
  half_width: number;
}

export interface RunResult {
  kpi: KpiSummary;
  ci: Record<string, CiStat>;
  hourly: HourPoint[];
  nodes: NodeStat[];
  zones: ZoneStat[];
  replication_kpis: KpiSummary[];
}

export interface Frame {
  minute: number;
  couriers: [number, number, number][];
  queues: number[];
  drops: [number, number, number][];
  orders_so_far: number;
  on_time_so_far: number;
  arrived_so_far: number;
  lost_so_far: number;
  px2_so_far: number;
  waited_so_far: number;
  ctd_sum_min: number;
  contrib_idr: number;
  promise_now: number;
  busy_pickers: number;
  total_pickers: number;
}

export interface ScenarioRow {
  id: number;
  name: string;
  updated_at: string;
  scenario: Scenario;
}

export interface RunRow {
  id: number;
  scenario_id: number;
  scenario_name: string;
  created_at: string;
  sla: number;
  contribution_idr: number;
  orders_per_day: number;
  n_nodes: number;
  frame_count: number;
}

export interface ValidationRow {
  law: string;
  reference: string;
  analytic: number;
  simulated: number;
  rel_err: number;
  pass: boolean;
}

export interface SweepPoint {
  n_nodes: number;
  sla: number;
  contribution_idr: number;
  lost_per_day: number;
  promise_shown: number;
}

export interface PartnershipOption {
  name: string;
  description: string;
  sla: number;
  contribution_idr: number;
  courier_cost_idr: number;
  p95_min: number;
  recommended: boolean;
}

export interface StoreRankRow {
  rank: number;
  x_km: number;
  y_km: number;
  marginal_coverage: number;
  cumulative_coverage: number;
}
