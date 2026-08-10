"use client";

// Animated twin playback at device resolution: courier scooters, facility
// pins, house-pin deliveries, coverage rings, and the mid-mile truck overlay.
// The canvas fills its container and re-renders crisply on resize/zoom.
import { useCallback, useEffect, useRef, useState } from "react";
import type { Frame, FulfillNode } from "@/lib/types";
import {
  blit, drawBase, drawFacility, makeSprites, makeTransform,
  tX, tY, type Sprites, KIND_COLOR,
} from "@/lib/mapkit";

export default function SimCanvas({
  metroKm,
  nodes,
  frames,
  cursor,
  coverageKm = 0,
  showCoverage = false,
  showMidMile = false,
  selectedNode = null,
  onSelectNode,
  heightClass = "h-[540px]",
}: {
  metroKm: number;
  nodes: FulfillNode[];
  frames: Frame[];
  cursor: number;
  coverageKm?: number;
  showCoverage?: boolean;
  showMidMile?: boolean;
  selectedNode?: number | null;
  onSelectNode?: (idx: number | null) => void;
  heightClass?: string;
}) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const ref = useRef<HTMLCanvasElement>(null);
  const baseRef = useRef<HTMLCanvasElement | null>(null);
  const spritesRef = useRef<Sprites | null>(null);
  const [size, setSize] = useState<{ w: number; h: number; dpr: number }>({ w: 0, h: 0, dpr: 1 });

  // observe container size; render at devicePixelRatio
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const update = () => {
      const dpr = Math.min(2.5, window.devicePixelRatio || 1);
      setSize({ w: el.clientWidth, h: el.clientHeight, dpr });
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // (re)build base map + sprites when geometry changes
  useEffect(() => {
    if (size.w === 0) return;
    const { w, h, dpr } = size;
    const off = document.createElement("canvas");
    off.width = w * dpr;
    off.height = h * dpr;
    const ctx = off.getContext("2d")!;
    ctx.scale(dpr, dpr);
    drawBase(ctx, makeTransform(w, h, metroKm));
    baseRef.current = off;
    if (!spritesRef.current || spritesRef.current.dpr !== dpr) {
      spritesRef.current = makeSprites(dpr);
    }
  }, [size, metroKm]);

  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!onSelectNode || size.w === 0) return;
      const rect = ref.current!.getBoundingClientRect();
      const px = e.clientX - rect.left;
      const py = e.clientY - rect.top;
      const t = makeTransform(size.w, size.h, metroKm);
      let best = -1, bestD = 24 * 24;
      nodes.forEach((n, i) => {
        const d = (tX(t, n.x_km) - px) ** 2 + (tY(t, n.y_km) - py) ** 2;
        if (d < bestD) { bestD = d; best = i; }
      });
      onSelectNode(best >= 0 ? best : null);
    },
    [nodes, metroKm, onSelectNode, size],
  );

  useEffect(() => {
    const cv = ref.current;
    const sprites = spritesRef.current;
    if (!cv || frames.length === 0 || size.w === 0 || !sprites) return;
    const { w, h, dpr } = size;
    if (cv.width !== Math.round(w * dpr)) {
      cv.width = Math.round(w * dpr);
      cv.height = Math.round(h * dpr);
    }
    const ctx = cv.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const t = makeTransform(w, h, metroKm);

    const i = Math.min(Math.floor(cursor), frames.length - 1);
    const f = frames[i];
    const fnext = frames[Math.min(i + 1, frames.length - 1)];
    const alpha = cursor - i;

    ctx.clearRect(0, 0, w, h);
    if (baseRef.current) ctx.drawImage(baseRef.current, 0, 0, w, h);

    // coverage rings (60-min promise reach)
    if (showCoverage && coverageKm > 0) {
      const rpx = coverageKm * t.s;
      nodes.forEach((n, idx) => {
        const x = tX(t, n.x_km), y = tY(t, n.y_km);
        ctx.beginPath();
        ctx.arc(x, y, rpx, 0, Math.PI * 2);
        if (idx === selectedNode) {
          ctx.fillStyle = KIND_COLOR[n.kind] + "14";
          ctx.fill();
        }
        ctx.strokeStyle = KIND_COLOR[n.kind] + (idx === selectedNode ? "99" : "38");
        ctx.setLineDash([6, 6]);
        ctx.lineWidth = idx === selectedNode ? 1.8 : 1.1;
        ctx.stroke();
        ctx.setLineDash([]);
      });
    }

    // mid-mile overlay with trucks during the 00:00-02:00 wave
    if (showMidMile) {
      const hubX = t.ox + 26, hubY = tY(t, 0) - 26;
      const minuteOfDay = f.minute % 1440;
      const replActive = minuteOfDay < 120;
      nodes.forEach((n, ni) => {
        if (n.kind === "Store") return;
        const ex = tX(t, n.x_km), ey = tY(t, n.y_km);
        const mx = (hubX + ex) / 2, my = (hubY + ey) / 2 - 0.16 * h;
        ctx.beginPath();
        ctx.moveTo(hubX + 26, hubY - 6);
        ctx.quadraticCurveTo(mx, my, ex, ey);
        ctx.strokeStyle = replActive ? "rgba(13,36,68,0.5)" : "rgba(33,90,169,0.26)";
        ctx.setLineDash([7, 7]);
        ctx.lineWidth = replActive ? 2 : 1.4;
        ctx.stroke();
        ctx.setLineDash([]);
        if (replActive) {
          const tt = Math.min(1, minuteOfDay / 110 + ni * 0.07);
          const qx = (1 - tt) ** 2 * (hubX + 26) + 2 * (1 - tt) * tt * mx + tt * tt * ex;
          const qy = (1 - tt) ** 2 * (hubY - 6) + 2 * (1 - tt) * tt * my + tt * tt * ey;
          blit(ctx, sprites.truck, dpr, qx, qy);
        }
      });
      // gateway card
      ctx.save();
      ctx.shadowColor = "rgba(13,36,68,0.3)";
      ctx.shadowBlur = 8;
      ctx.fillStyle = "rgba(13,36,68,0.92)";
      ctx.beginPath();
      ctx.roundRect(hubX - 12, hubY - 24, 118, 46, 8);
      ctx.fill();
      ctx.restore();
      blit(ctx, sprites.truck, dpr, hubX + 4, hubY - 6);
      ctx.fillStyle = "#ffffff";
      ctx.font = "bold 10px -apple-system, sans-serif";
      ctx.textAlign = "left";
      ctx.fillText("NDC / HUB", hubX + 20, hubY - 8);
      ctx.font = "8.5px -apple-system, sans-serif";
      ctx.fillStyle = "#9fb4d0";
      ctx.fillText("MOSTRANS wave 00–02h", hubX + 20, hubY + 3);
      ctx.fillText(replActive ? "trucks en route…" : "next wave at midnight", hubX + 20, hubY + 14);
    }

    // deliveries: fading house pins
    for (let k = Math.max(0, i - 10); k <= i; k++) {
      const age = (i - k) / 10;
      for (const [dx, dy, ok] of frames[k].drops) {
        blit(ctx, ok === 1 ? sprites.houseGreen : sprites.houseRed, dpr,
          tX(t, dx), tY(t, dy), (1 - age) * 0.95);
      }
    }

    // riders: interpolated scooters
    const n2 = new Map<number, [number, number, number]>();
    fnext.couriers.forEach((c, idx) => n2.set(idx, c));
    f.couriers.forEach((c, idx) => {
      const nx = n2.get(idx);
      const x = nx ? c[0] + (nx[0] - c[0]) * alpha : c[0];
      const y = nx ? c[1] + (nx[1] - c[1]) * alpha : c[1];
      blit(ctx, c[2] === 0 ? sprites.bikeGrey : sprites.bikeOrange, dpr, tX(t, x), tY(t, y));
    });

    // facilities on top
    nodes.forEach((n, idx) =>
      drawFacility(ctx, sprites, n, t, f.queues[idx] ?? 0, idx === selectedNode),
    );

    // clock chip
    const minute = f.minute % 1440;
    const hh = String(Math.floor(minute / 60)).padStart(2, "0");
    const mm = String(minute % 60).padStart(2, "0");
    const rate = f.orders_so_far > 0 ? (100 * f.on_time_so_far) / f.orders_so_far : 100;
    ctx.save();
    ctx.shadowColor = "rgba(13,36,68,0.35)";
    ctx.shadowBlur = 10;
    ctx.fillStyle = "rgba(13,36,68,0.92)";
    ctx.beginPath();
    ctx.roundRect(w - 208, 14, 192, 58, 10);
    ctx.fill();
    ctx.restore();
    ctx.fillStyle = "#ffffff";
    ctx.font = "bold 21px ui-monospace, SFMono-Regular, monospace";
    ctx.textAlign = "left";
    ctx.fillText(`Day ${Math.floor(f.minute / 1440) + 1}   ${hh}:${mm}`, w - 194, 38);
    ctx.font = "11px -apple-system, sans-serif";
    ctx.fillStyle = "#c7d4e6";
    ctx.fillText(`${f.orders_so_far.toLocaleString()} delivered · ${rate.toFixed(1)}% on-time`, w - 194, 58);
  }, [frames, cursor, nodes, metroKm, coverageKm, showCoverage, showMidMile, selectedNode, size]);

  return (
    <div ref={wrapRef} className={`relative w-full ${heightClass}`}>
      <canvas
        ref={ref}
        onClick={handleClick}
        className="absolute inset-0 w-full h-full rounded-xl border border-slate-200 bg-white cursor-pointer"
      />
    </div>
  );
}
