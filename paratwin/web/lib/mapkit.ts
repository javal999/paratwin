// Map rendering kit: HiDPI transforms, stylized city base map, and the
// pre-rendered icon sprite atlas (Google-Maps-style POI pins, scooters,
// house pins, trucks). Shared by the scenario preview and the runner.

import type { FulfillNode, NodeKind } from "./types";

export const KIND_COLOR: Record<string, string> = {
  Flagship: "#d97b29",
  Dark: "#215aa9",
  Store: "#0f8b8d",
};

export interface MapTransform {
  s: number;  // px per km (uniform)
  ox: number; // x offset px
  oy: number; // y offset px
  W: number;
  H: number;
  metroKm: number;
}

/** Uniform-scale transform, centered with letterboxing. */
export function makeTransform(W: number, H: number, metroKm: number): MapTransform {
  const s = Math.min(W / metroKm, H / metroKm);
  const ox = (W - s * metroKm) / 2;
  const oy = (H - s * metroKm) / 2;
  return { s, ox, oy, W, H, metroKm };
}
export const tX = (t: MapTransform, km: number) => t.ox + km * t.s;
export const tY = (t: MapTransform, km: number) => t.oy + (t.metroKm - km) * t.s;

// demand clusters — must match engine/src/demand.rs
const CLUSTERS: [number, number, number, number][] = [
  [0.5, 0.55, 0.16, 0.45],
  [0.38, 0.25, 0.2, 0.35],
  [0.78, 0.6, 0.14, 0.2],
];

/** Stylized city base map drawn at device resolution. */
export function drawBase(ctx: CanvasRenderingContext2D, t: MapTransform) {
  const { W, H, metroKm } = t;
  const m = metroKm;
  const x0 = tX(t, 0), y0 = tY(t, m), x1 = tX(t, m), y1 = tY(t, 0);
  const mw = x1 - x0, mh = y1 - y0;

  // outskirts + metro land
  ctx.fillStyle = "#e6eaee";
  ctx.fillRect(0, 0, W, H);
  ctx.fillStyle = "#f2f4f6";
  ctx.fillRect(x0, y0, mw, mh);

  ctx.save();
  ctx.beginPath();
  ctx.rect(x0, y0, mw, mh);
  ctx.clip();

  // parks (soft organic patches)
  const parks: [number, number, number, number][] = [
    [0.22, 0.68, 1.7, 1.1], [0.66, 0.34, 1.25, 0.9], [0.86, 0.82, 1.5, 1.0], [0.12, 0.18, 1.1, 0.8],
  ];
  ctx.fillStyle = "#d9e9d4";
  for (const [fx, fy, rx, ry] of parks) {
    ctx.beginPath();
    ctx.ellipse(x0 + fx * mw, y0 + fy * mh, rx * t.s, ry * t.s, 0.45, 0, Math.PI * 2);
    ctx.fill();
  }

  // river with soft bank
  const river = (w: number, col: string) => {
    ctx.strokeStyle = col;
    ctx.lineWidth = w;
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.moveTo(x0 + 0.58 * mw, y1 + 10);
    ctx.bezierCurveTo(
      x0 + 0.5 * mw, y0 + 0.72 * mh,
      x0 + 0.68 * mw, y0 + 0.48 * mh,
      x0 + 0.6 * mw, y0 + 0.26 * mh,
    );
    ctx.bezierCurveTo(
      x0 + 0.55 * mw, y0 + 0.12 * mh,
      x0 + 0.62 * mw, y0 + 0.04 * mh,
      x0 + 0.6 * mw, y0 - 10,
    );
    ctx.stroke();
  };
  river(Math.max(6, 0.34 * t.s), "#c3d9ec");
  river(Math.max(3, 0.2 * t.s), "#d4e6f5");

  // minor grid (1 km) — hairline
  ctx.strokeStyle = "rgba(255,255,255,0.9)";
  ctx.lineWidth = 1;
  for (let k = 1; k < m; k++) {
    ctx.beginPath(); ctx.moveTo(tX(t, k), y0); ctx.lineTo(tX(t, k), y1); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(x0, tY(t, k)); ctx.lineTo(x1, tY(t, k)); ctx.stroke();
  }
  // arterials with casing
  const step = Math.max(3, Math.round(m / 4));
  const artery = (a: number, vertical: boolean) => {
    const p = vertical
      ? [tX(t, a), y0, tX(t, a), y1]
      : [x0, tY(t, a), x1, tY(t, a)];
    ctx.strokeStyle = "#d3d9df";
    ctx.lineWidth = 4.5;
    ctx.beginPath(); ctx.moveTo(p[0], p[1]); ctx.lineTo(p[2], p[3]); ctx.stroke();
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 2.8;
    ctx.beginPath(); ctx.moveTo(p[0], p[1]); ctx.lineTo(p[2], p[3]); ctx.stroke();
  };
  for (let k = step; k < m; k += step) { artery(k, true); artery(k, false); }

  // ring road
  const ri = 0.16;
  ctx.strokeStyle = "#cdd5dd";
  ctx.lineWidth = 5.5;
  ctx.strokeRect(x0 + ri * mw, y0 + ri * mh, (1 - 2 * ri) * mw, (1 - 2 * ri) * mh);
  ctx.strokeStyle = "#fbfcfd";
  ctx.lineWidth = 3.2;
  ctx.strokeRect(x0 + ri * mw, y0 + ri * mh, (1 - 2 * ri) * mw, (1 - 2 * ri) * mh);

  // demand heat: layered radial gradients (smooth, print-quality).
  // Cluster sigma is a fraction of metro size (matches engine); render ~3σ.
  for (const [fx, fy, sig, w] of CLUSTERS) {
    const cx = x0 + fx * mw, cy = y0 + (1 - fy) * mh;
    const R = sig * m * 3 * t.s; // 3 sigma in px
    const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, R);
    const peak = 0.34 * (w / 0.45);
    g.addColorStop(0, `rgba(33,90,169,${peak.toFixed(3)})`);
    g.addColorStop(0.55, `rgba(33,90,169,${(peak * 0.35).toFixed(3)})`);
    g.addColorStop(1, "rgba(33,90,169,0)");
    ctx.fillStyle = g;
    ctx.fillRect(x0, y0, mw, mh);
  }

  ctx.restore();

  // metro boundary
  ctx.strokeStyle = "#c2cbd4";
  ctx.lineWidth = 1.5;
  ctx.strokeRect(x0, y0, mw, mh);
}

// ---------------- sprite atlas ----------------

export interface Sprites {
  dpr: number;
  pin: Record<NodeKind, HTMLCanvasElement>;
  bikeGrey: HTMLCanvasElement;
  bikeOrange: HTMLCanvasElement;
  houseGreen: HTMLCanvasElement;
  houseRed: HTMLCanvasElement;
  truck: HTMLCanvasElement;
}

function sprite(size: number, dpr: number, draw: (c: CanvasRenderingContext2D, s: number) => void): HTMLCanvasElement {
  const cv = document.createElement("canvas");
  cv.width = size * dpr;
  cv.height = size * dpr;
  const c = cv.getContext("2d")!;
  c.scale(dpr, dpr);
  draw(c, size);
  return cv;
}

/** Google-Maps-style POI pin: white circular badge, colored ring, glyph. */
function facilityPin(kind: NodeKind, dpr: number): HTMLCanvasElement {
  const S = kind === "Flagship" ? 46 : kind === "Dark" ? 40 : 34;
  return sprite(S, dpr, (c, s) => {
    const x = s / 2, y = s / 2;
    const r = s / 2 - 3;
    const col = KIND_COLOR[kind];
    // soft shadow
    c.save();
    c.shadowColor = "rgba(13,36,68,0.30)";
    c.shadowBlur = 4;
    c.shadowOffsetY = 1.5;
    c.beginPath();
    c.arc(x, y, r, 0, Math.PI * 2);
    c.fillStyle = kind === "Flagship" ? col : "#ffffff";
    c.fill();
    c.restore();
    // ring
    c.beginPath();
    c.arc(x, y, r, 0, Math.PI * 2);
    c.strokeStyle = kind === "Flagship" ? "#ffffff" : col;
    c.lineWidth = 2.4;
    c.stroke();
    // glyph
    const g = kind === "Flagship" ? "#ffffff" : col;
    c.strokeStyle = g;
    c.fillStyle = g;
    c.lineWidth = 1.8;
    c.lineJoin = "round";
    c.lineCap = "round";
    const u = r / 10;
    if (kind === "Flagship") {
      // warehouse: roof + body + door
      c.beginPath();
      c.moveTo(x - 6.5 * u, y - 1.5 * u); c.lineTo(x, y - 6 * u); c.lineTo(x + 6.5 * u, y - 1.5 * u);
      c.lineTo(x + 6.5 * u, y + 5.5 * u); c.lineTo(x - 6.5 * u, y + 5.5 * u); c.closePath();
      c.stroke();
      c.fillRect(x - 2 * u, y + 1.2 * u, 4 * u, 4.3 * u);
      c.beginPath();
      c.moveTo(x - 6.5 * u, y - 1.2 * u); c.lineTo(x + 6.5 * u, y - 1.2 * u);
      c.stroke();
    } else if (kind === "Dark") {
      // MFC: building with shelf rows
      c.strokeRect(x - 5.2 * u, y - 6 * u, 10.4 * u, 12 * u);
      for (const dy of [-3.2, 0.2]) {
        c.fillRect(x - 3.4 * u, y + dy * u, 2.5 * u, 1.8 * u);
        c.fillRect(x + 1.0 * u, y + dy * u, 2.5 * u, 1.8 * u);
      }
      c.fillRect(x - 1.2 * u, y + 3.2 * u, 2.4 * u, 2.8 * u);
    } else {
      // shop: awning + body + door
      c.beginPath();
      c.moveTo(x - 6 * u, y - 1.6 * u); c.lineTo(x - 5 * u, y - 5.4 * u);
      c.lineTo(x + 5 * u, y - 5.4 * u); c.lineTo(x + 6 * u, y - 1.6 * u);
      c.closePath();
      c.fill();
      c.strokeRect(x - 4.6 * u, y - 1.6 * u, 9.2 * u, 7.4 * u);
      c.clearRect(x - 4.6 * u + 1, y - 1.6 * u + 1, 9.2 * u - 2, 7.4 * u - 2);
      c.strokeRect(x - 4.6 * u, y - 1.6 * u, 9.2 * u, 7.4 * u);
      c.fillRect(x - 1 * u, y + 2 * u, 2 * u, 3.8 * u);
    }
  });
}

/** Delivery scooter with rider, subtle and crisp. */
function scooter(color: string, dpr: number): HTMLCanvasElement {
  return sprite(20, dpr, (c, s) => {
    const x = s / 2, y = s / 2;
    // halo
    c.beginPath(); c.arc(x, y, 8.6, 0, Math.PI * 2);
    c.fillStyle = "rgba(255,255,255,0.92)";
    c.fill();
    c.strokeStyle = "rgba(13,36,68,0.10)";
    c.lineWidth = 1;
    c.stroke();
    c.strokeStyle = color;
    c.fillStyle = color;
    c.lineWidth = 1.5;
    c.lineCap = "round";
    // wheels
    c.beginPath(); c.arc(x - 3.6, y + 3, 2.1, 0, Math.PI * 2); c.stroke();
    c.beginPath(); c.arc(x + 3.8, y + 3, 2.1, 0, Math.PI * 2); c.stroke();
    // deck + steering column
    c.beginPath();
    c.moveTo(x - 3.6, y + 3); c.lineTo(x - 1.2, y + 1.2); c.lineTo(x + 2.2, y + 1.2);
    c.moveTo(x + 2.6, y + 1.2); c.lineTo(x + 4.2, y - 2.4); c.lineTo(x + 5.2, y - 2.6);
    c.stroke();
    // rider: head + torso
    c.beginPath(); c.arc(x - 0.4, y - 4.2, 1.7, 0, Math.PI * 2); c.fill();
    c.beginPath();
    c.moveTo(x - 0.4, y - 2.4); c.lineTo(x - 1.0, y + 1.0);
    c.stroke();
    // delivery box
    c.fillRect(x - 4.6, y - 3.4, 3.1, 3.1);
  });
}

/** Consumer house pin. */
function housePin(color: string, dpr: number): HTMLCanvasElement {
  return sprite(14, dpr, (c, s) => {
    const x = s / 2, y = s / 2;
    c.fillStyle = color;
    c.strokeStyle = "#ffffff";
    c.lineWidth = 1.2;
    c.lineJoin = "round";
    c.beginPath();
    c.moveTo(x - 4.6, y - 0.4);
    c.lineTo(x, y - 5);
    c.lineTo(x + 4.6, y - 0.4);
    c.lineTo(x + 3.3, y - 0.4);
    c.lineTo(x + 3.3, y + 4.4);
    c.lineTo(x - 3.3, y + 4.4);
    c.lineTo(x - 3.3, y - 0.4);
    c.closePath();
    c.fill();
    c.stroke();
  });
}

/** Mid-mile box truck. */
function boxTruck(dpr: number): HTMLCanvasElement {
  return sprite(24, dpr, (c, s) => {
    const x = s / 2, y = s / 2;
    c.beginPath(); c.arc(x, y, 10.5, 0, Math.PI * 2);
    c.fillStyle = "rgba(255,255,255,0.92)"; c.fill();
    c.strokeStyle = "rgba(13,36,68,0.12)"; c.lineWidth = 1; c.stroke();
    c.fillStyle = "#0d2444";
    c.strokeStyle = "#ffffff";
    c.lineWidth = 1;
    c.beginPath(); c.roundRect(x - 7.6, y - 4.6, 10, 7.6, 1.2); c.fill(); c.stroke();
    c.fillStyle = "#d97b29";
    c.beginPath(); c.roundRect(x + 2.6, y - 2.2, 5, 5.2, 1.2); c.fill(); c.stroke();
    c.fillStyle = "#1e293b";
    c.beginPath(); c.arc(x - 4, y + 3.9, 1.7, 0, Math.PI * 2); c.fill();
    c.beginPath(); c.arc(x + 4.6, y + 3.9, 1.7, 0, Math.PI * 2); c.fill();
  });
}

export function makeSprites(dpr: number): Sprites {
  return {
    dpr,
    pin: {
      Flagship: facilityPin("Flagship", dpr),
      Dark: facilityPin("Dark", dpr),
      Store: facilityPin("Store", dpr),
    },
    bikeGrey: scooter("#7c8794", dpr),
    bikeOrange: scooter("#d97b29", dpr),
    houseGreen: housePin("#2e8b57", dpr),
    houseRed: housePin("#c0392b", dpr),
    truck: boxTruck(dpr),
  };
}

export function blit(
  ctx: CanvasRenderingContext2D,
  sp: HTMLCanvasElement,
  dpr: number,
  x: number,
  y: number,
  alpha = 1,
) {
  const w = sp.width / dpr, h = sp.height / dpr;
  if (alpha < 1) ctx.globalAlpha = alpha;
  ctx.drawImage(sp, x - w / 2, y - h / 2, w, h);
  if (alpha < 1) ctx.globalAlpha = 1;
}

/** Facility pin + notification-style queue badge. */
export function drawFacility(
  ctx: CanvasRenderingContext2D,
  sprites: Sprites,
  n: FulfillNode,
  t: MapTransform,
  queue?: number,
  selected?: boolean,
) {
  const x = tX(t, n.x_km), y = tY(t, n.y_km);
  if (selected) {
    ctx.beginPath();
    ctx.arc(x, y, 26, 0, Math.PI * 2);
    ctx.fillStyle = "rgba(13,36,68,0.08)";
    ctx.fill();
    ctx.strokeStyle = "#0d2444";
    ctx.lineWidth = 1.8;
    ctx.stroke();
  }
  blit(ctx, sprites.pin[n.kind], sprites.dpr, x, y);
  if (queue !== undefined && queue > 0) {
    const bx = x + (n.kind === "Flagship" ? 15 : n.kind === "Dark" ? 13 : 11);
    const by = y - (n.kind === "Flagship" ? 15 : n.kind === "Dark" ? 13 : 11);
    const wide = queue > 9;
    ctx.beginPath();
    ctx.roundRect(bx - (wide ? 10 : 7.5), by - 7.5, wide ? 20 : 15, 15, 7.5);
    ctx.fillStyle = queue > 8 ? "#c0392b" : "#0d2444";
    ctx.fill();
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1.6;
    ctx.stroke();
    ctx.fillStyle = "#ffffff";
    ctx.font = "bold 9.5px -apple-system, sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(String(queue), bx, by + 0.5);
    ctx.textBaseline = "alphabetic";
  }
}
