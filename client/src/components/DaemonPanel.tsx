"use client";

import { useCallback, useEffect, useState } from "react";

interface DaemonInfo {
  version: string;
  uptimeSeconds: string;
  configPath: string;
  workersCount: number;
}

function formatUptime(raw: string): string {
  const s = parseInt(raw, 10) || 0;
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const parts: string[] = [];
  if (h) parts.push(`${h}h`);
  if (m) parts.push(`${m}m`);
  parts.push(`${sec}s`);
  return parts.join(" ");
}

function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3 mb-5">
      <div className="w-0.5 h-5 bg-orange-500 rounded-full shrink-0" />
      <h2 className="font-sans font-bold text-xs tracking-widest uppercase text-[#c9d1d9]">
        {children}
      </h2>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <dt className="font-mono text-[10px] tracking-widest uppercase text-[#6e7681]">
        {label}
      </dt>
      <dd className="font-mono text-sm text-[#c9d1d9] truncate">{value}</dd>
    </div>
  );
}

export default function DaemonPanel() {
  const [info, setInfo] = useState<DaemonInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [reloading, setReloading] = useState(false);
  const [shuttingDown, setShuttingDown] = useState(false);

  const fetchInfo = useCallback(async () => {
    try {
      const res = await fetch("/api/daemon/info");
      if (!res.ok) throw new Error(await res.text());
      setInfo(await res.json());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchInfo(); }, [fetchInfo]);

  const handleReload = async () => {
    setReloading(true);
    try {
      await fetch("/api/daemon/reload", { method: "POST" });
      await fetchInfo();
    } finally {
      setReloading(false);
    }
  };

  const handleShutdown = async () => {
    if (!confirm("Shut down the daemon? It will need to be restarted manually.")) return;
    setShuttingDown(true);
    try {
      await fetch("/api/daemon/shutdown", { method: "POST" });
    } finally {
      setShuttingDown(false);
    }
  };

  return (
    <section className="border border-[#1c2736] rounded-lg bg-[#0d1117] p-6 animate-fade-up">
      <SectionHeader>Daemon</SectionHeader>

      {loading && (
        <div className="flex items-center gap-2 text-[#6e7681] font-mono text-xs py-4">
          <span className="animate-spin-slow inline-block w-3 h-3 border border-[#6e7681] border-t-transparent rounded-full" />
          Connecting...
        </div>
      )}

      {error && (
        <p className="font-mono text-xs text-red-400 bg-red-950/30 border border-red-900/50 rounded px-3 py-2">
          {error}
        </p>
      )}

      {info && (
        <div className="space-y-6 animate-fade-up">
          <dl className="grid grid-cols-2 sm:grid-cols-4 gap-5">
            <Stat label="Version" value={`v${info.version}`} />
            <Stat label="Uptime" value={formatUptime(info.uptimeSeconds)} />
            <Stat label="Workers" value={info.workersCount} />
            <Stat label="Config" value={<span title={info.configPath} className="block truncate">{info.configPath || "—"}</span>} />
          </dl>

          <div className="flex gap-3 pt-1">
            <button
              onClick={handleReload}
              disabled={reloading}
              className="font-mono text-xs px-4 py-2 rounded border border-[#1c2736] text-[#c9d1d9] hover:border-orange-500/60 hover:text-orange-400 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            >
              {reloading ? "Reloading…" : "Reload Config"}
            </button>
            <button
              onClick={handleShutdown}
              disabled={shuttingDown}
              className="font-mono text-xs px-4 py-2 rounded border border-red-900/60 text-red-400 hover:bg-red-950/30 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            >
              {shuttingDown ? "Shutting down…" : "Shutdown"}
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
