"use client";

// Business-friendly explainers: an (i) hover tooltip and a chart caption.

export function Info({ text }: { text: string }) {
  return (
    <span className="info-dot" tabIndex={0}>
      i<span className="info-pop">{text}</span>
    </span>
  );
}

/** One-line "how to read this" caption placed under charts and tables. */
export function HowToRead({ children }: { children: React.ReactNode }) {
  return (
    <p className="mt-2 text-[11px] leading-relaxed text-slate-500 bg-slate-50 border-l-2 border-paragon-300 rounded-r px-2.5 py-1.5">
      <span className="font-semibold text-paragon-700">How to read this: </span>
      {children}
    </p>
  );
}

/** KPI stat card with a plain-language subtitle and optional tooltip. */
export function Stat({
  label,
  value,
  sub,
  tone = "default",
  help,
}: {
  label: string;
  value: React.ReactNode;
  sub?: React.ReactNode;
  tone?: "default" | "good" | "bad";
  help?: string;
}) {
  const color =
    tone === "good" ? "text-accent-green" : tone === "bad" ? "text-accent-red" : "text-paragon-700";
  return (
    <div className="card !p-4">
      <div className="stat-label flex items-center gap-1.5">
        {label}
        {help && <Info text={help} />}
      </div>
      <div className={`text-2xl font-bold ${color}`}>{value}</div>
      {sub && <div className="text-[10px] text-slate-400 mt-1 leading-relaxed">{sub}</div>}
    </div>
  );
}
