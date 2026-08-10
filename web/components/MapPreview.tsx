"use client";

// Static network preview (Scenario Builder): city base map + facility pins,
// rendered at device resolution via the shared map kit.
import { useEffect, useRef, useState } from "react";
import type { FulfillNode } from "@/lib/types";
import {
  drawBase, drawFacility, makeSprites, makeTransform,
  type Sprites, KIND_COLOR,
} from "@/lib/mapkit";

export { KIND_COLOR };

export default function MapPreview({
  metroKm,
  nodes,
  heightClass = "h-[400px]",
}: {
  metroKm: number;
  nodes: FulfillNode[];
  heightClass?: string;
}) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const ref = useRef<HTMLCanvasElement>(null);
  const spritesRef = useRef<Sprites | null>(null);
  const [size, setSize] = useState<{ w: number; h: number; dpr: number }>({ w: 0, h: 0, dpr: 1 });

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

  useEffect(() => {
    const cv = ref.current;
    if (!cv || size.w === 0) return;
    const { w, h, dpr } = size;
    cv.width = Math.round(w * dpr);
    cv.height = Math.round(h * dpr);
    const ctx = cv.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (!spritesRef.current || spritesRef.current.dpr !== dpr) {
      spritesRef.current = makeSprites(dpr);
    }
    const t = makeTransform(w, h, metroKm);
    ctx.clearRect(0, 0, w, h);
    drawBase(ctx, t);
    nodes.forEach((n) => drawFacility(ctx, spritesRef.current!, n, t));
  }, [size, metroKm, nodes]);

  return (
    <div>
      <div ref={wrapRef} className={`relative w-full ${heightClass}`}>
        <canvas ref={ref} className="absolute inset-0 w-full h-full rounded-xl border border-slate-200 bg-white" />
      </div>
      <div className="flex gap-4 mt-2 text-xs text-slate-500 flex-wrap">
        <span><span className="inline-block w-3 h-3 rounded-full align-middle mr-1 border-2" style={{ background: KIND_COLOR.Flagship, borderColor: "white", boxShadow: "0 0 0 1px " + KIND_COLOR.Flagship }} /> Flagship MFC</span>
        <span><span className="inline-block w-3 h-3 rounded-full align-middle mr-1 bg-white border-2" style={{ borderColor: KIND_COLOR.Dark }} /> Dark MFC</span>
        <span><span className="inline-block w-3 h-3 rounded-full align-middle mr-1 bg-white border-2" style={{ borderColor: KIND_COLOR.Store }} /> Consignment store</span>
        <span className="text-slate-400 ml-auto">blue shading = consumer demand · green = parks · blue line = river</span>
      </div>
    </div>
  );
}
