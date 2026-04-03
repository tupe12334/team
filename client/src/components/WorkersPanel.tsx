"use client";

import { useCallback, useEffect, useRef, useState } from "react";

interface WorkerInfo {
  workerId: string;
  status: "IDLE" | "BUSY";
  currentTaskId: string;
  currentAgent: string;
  taskStartedAt: string | null;
}

interface WorkerStatusData {
  total: number;
  busy: number;
  idle: number;
  workers: WorkerInfo[];
}

function SectionHeader({ children, right }: { children: React.ReactNode; right?: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3 mb-5">
      <div className="flex items-center gap-3">
        <div className="w-0.5 h-5 bg-orange-500 rounded-full shrink-0" />
        <h2 className="font-sans font-bold text-xs tracking-widest uppercase text-[#c9d1d9]">
          {children}
        </h2>
      </div>
      {right}
    </div>
  );
}

export default function WorkersPanel() {
  const [data, setData] = useState<WorkerStatusData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const res = await fetch("/api/workers");
      if (!res.ok) throw new Error(await res.text());
      setData(await res.json());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStatus();
    intervalRef.current = setInterval(fetchStatus, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [fetchStatus]);

  const live = (
    <span className="flex items-center gap-1.5 font-mono text-[10px] text-[#6e7681]">
      <span className="animate-pulse-dot w-1.5 h-1.5 rounded-full bg-orange-500/70" />
      live · 5s
    </span>
  );

  return (
    <section className="border border-[#1c2736] rounded-lg bg-[#0d1117] p-6 animate-fade-up" style={{ animationDelay: "120ms" }}>
      <SectionHeader right={live}>Workers</SectionHeader>

      {loading && (
        <div className="flex items-center gap-2 text-[#6e7681] font-mono text-xs py-4">
          <span className="animate-spin-slow inline-block w-3 h-3 border border-[#6e7681] border-t-transparent rounded-full" />
          Loading…
        </div>
      )}

      {error && (
        <p className="font-mono text-xs text-red-400 bg-red-950/30 border border-red-900/50 rounded px-3 py-2">
          {error}
        </p>
      )}

      {data && (
        <div className="space-y-6 animate-fade-up">
          {/* ── Summary stats ─────────────────────────────────────────────── */}
          <div className="grid grid-cols-3 gap-4">
            {[
              { label: "Total",  value: data.total, color: "text-[#c9d1d9]" },
              { label: "Busy",   value: data.busy,  color: "text-amber-400" },
              { label: "Idle",   value: data.idle,  color: "text-emerald-400" },
            ].map(({ label, value, color }) => (
              <div key={label} className="flex flex-col items-center border border-[#1c2736] rounded-lg py-4 gap-1 bg-[#07090c]/40">
                <span className={`font-mono text-2xl font-semibold ${color}`}>
                  {value}
                </span>
                <span className="font-mono text-[10px] tracking-widest uppercase text-[#6e7681]">
                  {label}
                </span>
              </div>
            ))}
          </div>

          {/* ── Worker list ───────────────────────────────────────────────── */}
          {data.workers.length === 0 ? (
            <p className="font-mono text-xs text-[#6e7681] py-4 text-center border border-dashed border-[#1c2736] rounded">
              No workers registered
            </p>
          ) : (
            <ul className="space-y-2 stagger">
              {data.workers.map((w) => (
                <li
                  key={w.workerId}
                  className="flex flex-wrap items-center gap-x-4 gap-y-1 border border-[#1c2736]/60 rounded px-4 py-2.5 bg-[#07090c]/30 hover:bg-white/[0.015] transition-colors"
                >
                  {/* Status badge */}
                  <span
                    className={`font-mono text-[10px] tracking-wider px-2 py-0.5 rounded shrink-0 ${
                      w.status === "IDLE"
                        ? "bg-emerald-950/60 text-emerald-300 border border-emerald-700/50"
                        : "bg-amber-950/60 text-amber-300 border border-amber-700/50"
                    }`}
                  >
                    {w.status}
                  </span>

                  {/* Worker ID */}
                  <span className="font-mono text-xs text-[#c9d1d9] truncate">
                    {w.workerId}
                  </span>

                  {/* Task info when busy */}
                  {w.status === "BUSY" && w.currentTaskId && (
                    <span className="font-mono text-xs text-[#6e7681] truncate">
                      task:{" "}
                      <span className="text-blue-300">{w.currentTaskId}</span>
                      {w.currentAgent && (
                        <> · agent: <span className="text-[#c9d1d9]">{w.currentAgent}</span></>
                      )}
                    </span>
                  )}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
