// Command bridge: uses Tauri IPC inside the desktop app, and falls back to
// the paratwin-devserver HTTP API when running in a plain browser (dev/test).

const DEV_URL = "http://127.0.0.1:8787";

function hasTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (hasTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  const res = await fetch(`${DEV_URL}/rpc/${cmd}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args),
  });
  if (!res.ok) throw new Error(await res.text());
  return (await res.json()) as T;
}

export function fmtIDR(v: number): string {
  const abs = Math.abs(v);
  const sign = v < 0 ? "-" : "";
  if (abs >= 1e9) return `${sign}Rp ${(abs / 1e9).toFixed(2)} B`;
  if (abs >= 1e6) return `${sign}Rp ${(abs / 1e6).toFixed(1)} M`;
  if (abs >= 1e3) return `${sign}Rp ${(abs / 1e3).toFixed(0)} k`;
  return `${sign}Rp ${abs.toFixed(0)}`;
}

export function fmtPct(v: number, digits = 1): string {
  return `${(v * 100).toFixed(digits)}%`;
}

export function fmtMin(v: number): string {
  return `${v.toFixed(1)} min`;
}
