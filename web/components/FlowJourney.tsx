"use client";

// Interactive end-to-end flow: online order (TikTok/Shopee) -> promise check
// -> routing decision -> fulfillment center -> last mile -> consumer, with
// the mid-mile replenishment lane (NDC -> hub -> FC via MOSTRANS) underneath.
// Animated dots ride the paths; clicking a stage explains the decision made
// there in business language, with the live parameters of the scenario.

import { useEffect, useMemo, useRef, useState } from "react";
import type { Scenario } from "@/lib/types";

const BLUE = "#215aa9", TEAL = "#0f8b8d", ORANGE = "#d97b29", GREEN = "#2e8b57",
  DARKBG = "#0d2444", GREY = "#64748b";

interface Stage {
  id: string;
  x: number;
  title: string;
  sub: string;
  color: string;
  decision: string;
  detail: (sc: Scenario | null) => string;
}

const STAGES: Stage[] = [
  {
    id: "order", x: 70, title: "Online order", sub: "TikTok · Shopee · brand.com", color: TEAL,
    decision: "Consumer taps “Instant delivery”",
    detail: (sc) =>
      `A shopper checks out on a marketplace or brand.com. Before they pay, ParaExpress already answered one question: “can we deliver this basket within ${sc?.sla_min ?? 60} minutes from a nearby stock point?” Only then is the Instant option shown — a promise we can keep, not a guess.`,
  },
  {
    id: "promise", x: 235, title: "Promise check", sub: "MagPie+ · ADR", color: BLUE,
    decision: "Decision 1 — do we promise 60 minutes?",
    detail: (sc) =>
      `MagPie+ (the order system) checks live availability-to-deliver: is there a fulfillment point near the consumer with the items in stock and a courier reachable in time? Target: deliver within ${sc?.sla_min ?? 60} min for ${((sc?.sla_target ?? 0.95) * 100).toFixed(0)}% of orders. If not feasible, the shopper is offered same-day instead — we never promise what the network cannot do.`,
  },
  {
    id: "routing", x: 400, title: "Smart routing", sub: "ORE · min-cost assignment", color: BLUE,
    decision: "Decision 2 — which location fulfills it?",
    detail: (sc) =>
      `Every ${sc?.wave_seconds ?? 20} seconds, the Order Routing Engine assigns all new orders at once to the cheapest feasible locations — balancing courier fee, queue congestion and remaining stock. This is an exactly-optimal calculation (min-cost flow), not a nearest-store rule of thumb: it prevents one busy store from drowning while a quiet one idles.`,
  },
  {
    id: "fc", x: 565, title: "Fulfillment point", sub: "store node · dark MFC · flagship", color: ORANGE,
    decision: "Decision 3 — pick fast, book the rider early",
    detail: (sc) =>
      `A picker at the chosen location packs the basket in ~${sc?.pick_mean_min ?? 4} minutes. The moment picking starts, ShipGo+ already books the rider (“pre-matching”) — so the rider arrives while the parcel is being sealed, not after. Overlapping these two clocks buys back ~${Math.max(0, (sc?.courier.match_mean_min ?? 6.5) - 2).toFixed(0)} minutes on every order.`,
  },
  {
    id: "lastmile", x: 730, title: "Last mile", sub: "Grab · GoSend", color: ORANGE,
    decision: "Decision 4 — which courier, what price?",
    detail: (sc) =>
      `ShipGo+ quotes ${sc?.courier.dual_carrier ? "both Grab and GoSend and books the better offer, with automatic failover if the first fails" : "Grab"} — riders are rented per delivery (≈ Rp ${Math.round((sc?.courier.fee_base ?? 9000) / 1000)}k base + Rp ${((sc?.courier.fee_per_km ?? 2400) / 1000).toFixed(1)}k/km), so ParagonCorp pays only for orders actually delivered. No idle fleet on the payroll.`,
  },
  {
    id: "consumer", x: 895, title: "Delivered", sub: "≤ 60 min click-to-door", color: GREEN,
    decision: "The promise is kept",
    detail: (sc) =>
      `The parcel arrives within the ${sc?.sla_min ?? 60}-minute promise. Every step above emitted events into the digital twin, so tomorrow's network is planned from today's reality — and every KPI on the Results page traces back to this exact journey.`,
  },
];

const MID_Y = 118;      // consumer-flow lane
const REPL_Y = 258;     // replenishment lane

export default function FlowJourney({ scenario }: { scenario: Scenario | null }) {
  const [sel, setSel] = useState<string>("routing");
  const dotsRef = useRef<SVGGElement>(null);
  const raf = useRef<number>(0);

  // animation: order dots along the top lane, parcels along the mid-mile lane
  useEffect(() => {
    const start = performance.now();
    const tick = (t: number) => {
      const g = dotsRef.current;
      if (g) {
        const T = (t - start) / 1000;
        const orderDots = 5;
        for (let i = 0; i < orderDots; i++) {
          const el = g.querySelector<SVGCircleElement>(`#odot${i}`);
          if (!el) continue;
          const p = ((T * 0.12 + i / orderDots) % 1);
          el.setAttribute("cx", String(70 + p * (895 - 70)));
          el.setAttribute("cy", String(MID_Y));
          el.setAttribute("opacity", p < 0.02 || p > 0.98 ? "0" : "0.9");
        }
        for (let i = 0; i < 3; i++) {
          const el = g.querySelector<SVGCircleElement>(`#mdot${i}`);
          if (!el) continue;
          const p = ((T * 0.05 + i / 3) % 1);
          el.setAttribute("cx", String(120 + p * (565 - 120)));
          el.setAttribute("cy", String(REPL_Y));
          el.setAttribute("opacity", p < 0.02 || p > 0.96 ? "0" : "0.85");
        }
      }
      raf.current = requestAnimationFrame(tick);
    };
    raf.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf.current);
  }, []);

  const stage = useMemo(() => STAGES.find((s) => s.id === sel)!, [sel]);

  return (
    <div className="card !p-0 overflow-hidden">
      <div className="px-5 pt-4 pb-1 flex items-baseline justify-between">
        <div>
          <h2 className="font-bold text-paragon-800">How an instant order flows through ParaExpress</h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Top lane: one consumer order, click to checkout in ≤ 60 minutes. Bottom lane: how the
            shelves got stocked before the order ever arrived. <b>Click any step</b> to see the
            decision made there.
          </p>
        </div>
      </div>

      <svg viewBox="0 0 1000 330" className="w-full block select-none">
        {/* ---- consumer flow lane ---- */}
        <line x1={70} y1={MID_Y} x2={895} y2={MID_Y} stroke={BLUE} strokeWidth={2.5} strokeDasharray="1 6" strokeLinecap="round" opacity={0.5} />
        {/* moving order dots */}
        <g ref={dotsRef}>
          {[0, 1, 2, 3, 4].map((i) => (
            <circle key={i} id={`odot${i}`} r={4.5} fill={ORANGE} stroke="white" strokeWidth={1.2} />
          ))}
          {[0, 1, 2].map((i) => (
            <circle key={i} id={`mdot${i}`} r={5.5} fill={BLUE} stroke="white" strokeWidth={1.2} />
          ))}
        </g>

        {/* stage nodes */}
        {STAGES.map((s, i) => {
          const active = s.id === sel;
          return (
            <g key={s.id} className="stage-btn" onClick={() => setSel(s.id)}>
              {/* connector arrowhead */}
              {i > 0 && (
                <polygon
                  points={`${s.x - 46},${MID_Y - 4} ${s.x - 38},${MID_Y} ${s.x - 46},${MID_Y + 4}`}
                  fill={BLUE} opacity={0.55}
                />
              )}
              <circle cx={s.x} cy={MID_Y} r={active ? 30 : 26} fill={active ? s.color : "white"}
                stroke={s.color} strokeWidth={active ? 3 : 2} />
              <StageGlyph id={s.id} x={s.x} y={MID_Y} active={active} color={s.color} />
              <text x={s.x} y={MID_Y - 44} textAnchor="middle" fontSize={13} fontWeight={700}
                fill={active ? s.color : "#1e293b"}>{s.title}</text>
              <text x={s.x} y={MID_Y - 30} textAnchor="middle" fontSize={9.5} fill={GREY}>{s.sub}</text>
              {/* decision badge for decision stages */}
              {s.decision.startsWith("Decision") && (
                <g>
                  <rect x={s.x - 34} y={MID_Y + 36} width={68} height={16} rx={8}
                    fill={active ? s.color : "#eef2f7"} />
                  <text x={s.x} y={MID_Y + 47} textAnchor="middle" fontSize={9} fontWeight={700}
                    fill={active ? "white" : GREY}>
                    DECISION {s.decision[9]}
                  </text>
                </g>
              )}
            </g>
          );
        })}

        {/* ---- mid-mile replenishment lane ---- */}
        <rect x={30} y={REPL_Y - 44} width={940} height={86} rx={10} fill="#f1f5f9" />
        <text x={48} y={REPL_Y - 26} fontSize={10.5} fontWeight={700} fill={GREY} letterSpacing={1}>
          BEFORE THE ORDER — REPLENISHMENT (MID MILE, RUNS DAILY OVERNIGHT)
        </text>
        <line x1={120} y1={REPL_Y} x2={565} y2={REPL_Y} stroke={BLUE} strokeWidth={2} strokeDasharray="6 5" opacity={0.6} />

        {/* factory / NDC / hub / FC glyphs */}
        <g>
          <rect x={95} y={REPL_Y - 13} width={50} height={26} rx={4} fill={DARKBG} />
          <text x={120} y={REPL_Y + 4} textAnchor="middle" fontSize={9.5} fontWeight={700} fill="white">NDC</text>
          <text x={120} y={REPL_Y + 30} textAnchor="middle" fontSize={9} fill={GREY}>national stock · Jakarta</text>
        </g>
        <g>
          <rect x={305} y={REPL_Y - 13} width={60} height={26} rx={4} fill="white" stroke={BLUE} strokeWidth={1.6} />
          <text x={335} y={REPL_Y + 4} textAnchor="middle" fontSize={9.5} fontWeight={700} fill={BLUE}>Regional hub</text>
          <text x={335} y={REPL_Y + 30} textAnchor="middle" fontSize={9} fill={GREY}>existing B2B DCs, cross-dock</text>
          <text x={228} y={REPL_Y - 8} textAnchor="middle" fontSize={9} fill={TEAL} fontWeight={600}>MOSTRANS trucks</text>
          <text x={462} y={REPL_Y - 8} textAnchor="middle" fontSize={9} fill={TEAL} fontWeight={600}>daily wave, 96% on-time</text>
        </g>
        <g>
          <rect x={540} y={REPL_Y - 13} width={50} height={26} rx={4} fill={ORANGE} opacity={0.9} />
          <text x={565} y={REPL_Y + 4} textAnchor="middle" fontSize={9.5} fontWeight={700} fill="white">FC shelf</text>
          <text x={565} y={REPL_Y + 30} textAnchor="middle" fontSize={9} fill={GREY}>stocked before demand peaks</text>
        </g>
        {/* connector: replenishment feeds the FC stage above */}
        <path d={`M ${565} ${REPL_Y - 16} C ${565} ${200}, ${565} ${175}, ${565} ${MID_Y + 34}`}
          stroke={ORANGE} strokeWidth={1.8} strokeDasharray="4 4" fill="none" opacity={0.6} />
        <polygon points={`${561},${MID_Y + 40} ${565},${MID_Y + 32} ${569},${MID_Y + 40}`} fill={ORANGE} opacity={0.8} />
      </svg>

      {/* stage detail panel */}
      <div className="mx-5 mb-4 rounded-lg border px-4 py-3"
        style={{ borderColor: stage.color + "55", background: stage.color + "0d" }}>
        <div className="text-sm font-bold" style={{ color: stage.color }}>{stage.decision}</div>
        <p className="text-[13px] text-slate-600 leading-relaxed mt-1">{stage.detail(scenario)}</p>
      </div>
    </div>
  );
}

function StageGlyph({ id, x, y, active, color }: {
  id: string; x: number; y: number; active: boolean; color: string;
}) {
  const c = active ? "white" : color;
  switch (id) {
    case "order": // phone
      return (
        <g>
          <rect x={x - 7} y={y - 11} width={14} height={22} rx={3} fill="none" stroke={c} strokeWidth={2} />
          <line x1={x - 3} y1={y + 7} x2={x + 3} y2={y + 7} stroke={c} strokeWidth={2} strokeLinecap="round" />
        </g>
      );
    case "promise": // checked clock
      return (
        <g>
          <circle cx={x} cy={y} r={10} fill="none" stroke={c} strokeWidth={2} />
          <path d={`M ${x} ${y - 6} L ${x} ${y} L ${x + 5} ${y + 3}`} stroke={c} strokeWidth={2} fill="none" strokeLinecap="round" />
        </g>
      );
    case "routing": // branching arrows
      return (
        <g stroke={c} strokeWidth={2} fill="none" strokeLinecap="round">
          <path d={`M ${x - 9} ${y} H ${x - 1}`} />
          <path d={`M ${x - 1} ${y} L ${x + 8} ${y - 7}`} />
          <path d={`M ${x - 1} ${y} L ${x + 8} ${y + 7}`} />
          <circle cx={x + 9} cy={y - 8} r={2.2} fill={c} stroke="none" />
          <circle cx={x + 9} cy={y + 8} r={2.2} fill={c} stroke="none" />
        </g>
      );
    case "fc": // box
      return (
        <g stroke={c} strokeWidth={2} fill="none" strokeLinejoin="round">
          <rect x={x - 9} y={y - 7} width={18} height={15} rx={2} />
          <line x1={x - 9} y1={y - 1} x2={x + 9} y2={y - 1} />
          <line x1={x} y1={y - 7} x2={x} y2={y - 1} />
        </g>
      );
    case "lastmile": // motorbike-ish
      return (
        <g stroke={c} strokeWidth={2} fill="none" strokeLinecap="round">
          <circle cx={x - 6} cy={y + 5} r={4} />
          <circle cx={x + 7} cy={y + 5} r={4} />
          <path d={`M ${x - 6} ${y + 5} L ${x - 1} ${y - 4} H ${x + 5} L ${x + 7} ${y + 5}`} />
          <path d={`M ${x - 1} ${y - 4} L ${x - 4} ${y - 7}`} />
        </g>
      );
    default: // home
      return (
        <g stroke={c} strokeWidth={2} fill="none" strokeLinejoin="round">
          <path d={`M ${x - 9} ${y + 1} L ${x} ${y - 8} L ${x + 9} ${y + 1}`} />
          <path d={`M ${x - 6} ${y} V ${y + 8} H ${x + 6} V ${y}`} />
        </g>
      );
  }
}
