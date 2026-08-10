"use client";

import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";

const items = [
  { href: "/", label: "Dashboard", icon: "◆" },
  { href: "/scenarios/", label: "Scenarios", icon: "⚙" },
  { href: "/runner/", label: "Simulation Runner", icon: "▶" },
  { href: "/results/", label: "Results", icon: "▤" },
  { href: "/decisions/", label: "Decision Studio", icon: "✓" },
  { href: "/math/", label: "Model Validation", icon: "∑" },
];

export default function Nav() {
  const path = usePathname();
  return (
    <aside className="w-60 shrink-0 bg-paragon-900 text-white flex flex-col sticky top-0 h-screen">
      <div className="px-5 py-6 flex items-center gap-3 border-b border-paragon-800">
        <div className="bg-white rounded-lg p-1.5">
          <Image src="/paragon.jpeg" alt="ParagonCorp" width={34} height={34} />
        </div>
        <div>
          <div className="font-bold leading-tight">ParaTwin</div>
          <div className="text-[10px] uppercase tracking-widest text-paragon-300">
            ParaExpress Twin
          </div>
        </div>
      </div>
      <nav className="flex-1 py-4">
        {items.map((it) => {
          const active =
            it.href === "/" ? path === "/" : path.startsWith(it.href.replace(/\/$/, ""));
          return (
            <Link
              key={it.href}
              href={it.href}
              className={`flex items-center gap-3 px-5 py-2.5 text-sm transition-colors ${
                active
                  ? "bg-paragon-700 text-white font-semibold border-l-4 border-accent-orange"
                  : "text-paragon-200 hover:bg-paragon-800 hover:text-white border-l-4 border-transparent"
              }`}
            >
              <span className="w-4 text-center">{it.icon}</span>
              {it.label}
            </Link>
          );
        })}
      </nav>
      <div className="px-5 py-4 text-[10px] text-paragon-400 border-t border-paragon-800 leading-relaxed">
        ParaTech Platform R&amp;D
        <br />
        DES · Agent-Based · Monte Carlo
        <br />
        v1.3.1 — Aug 2026
      </div>
    </aside>
  );
}
