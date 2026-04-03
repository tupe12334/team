"use client";

import SectionHeader from "@/components/SectionHeader";
import { useWorkersPanel } from "@/components/useWorkersPanel";

const live = (
  <span className="workers-panel__live flex items-center gap-1.5 font-mono text-[10px] text-[#6e7681]">
    <span className="animate-pulse-dot w-1.5 h-1.5 rounded-full bg-orange-500/70" />
    live · 5s
  </span>
);

export default function WorkersPanel() {
  const { data, error, loading } = useWorkersPanel();

  return (
    <section className="border border-[#1c2736] rounded-lg bg-[#0d1117] p-6 animate-fade-up" style={{ animationDelay: "120ms" }}>
      <SectionHeader right={live}>Workers</SectionHeader>

      {loading && (
        <div className="workers-panel__loading flex items-center gap-2 text-[#6e7681] font-mono text-xs py-4">
          <span className="animate-spin-slow inline-block w-3 h-3 border border-[#6e7681] border-t-transparent rounded-full" />
          Loading…
        </div>
      )}

      {error && (
        <p className="workers-panel__error font-mono text-xs text-red-400 bg-red-950/30 border border-red-900/50 rounded px-3 py-2">
          {error}
        </p>
      )}

      {data && (
        <div className="workers-panel__body space-y-6 animate-fade-up">
          <div className="workers-panel__stats grid grid-cols-3 gap-4">
            {[
              { label: "Total",  value: data.total, color: "text-[#c9d1d9]" },
              { label: "Busy",   value: data.busy,  color: "text-amber-400" },
              { label: "Idle",   value: data.idle,  color: "text-emerald-400" },
            ].map(({ label, value, color }) => (
              <div key={label} className="workers-stat flex flex-col items-center border border-[#1c2736] rounded-lg py-4 gap-1 bg-[#07090c]/40">
                <span className={`workers-stat__value font-mono text-2xl font-semibold ${color}`}>{value}</span>
                <span className="workers-stat__label font-mono text-[10px] tracking-widest uppercase text-[#6e7681]">{label}</span>
              </div>
            ))}
          </div>

          {data.workers.length === 0 ? (
            <p className="workers-panel__empty font-mono text-xs text-[#6e7681] py-4 text-center border border-dashed border-[#1c2736] rounded">
              No workers registered
            </p>
          ) : (
            <ul className="space-y-2 stagger">
              {data.workers.map((w) => (
                <li key={w.workerId}
                  className="worker-item flex flex-wrap items-center gap-x-4 gap-y-1 border border-[#1c2736]/60 rounded px-4 py-2.5 bg-[#07090c]/30 hover:bg-white/[0.015] transition-colors">
                  <span className={`worker-item__status font-mono text-[10px] tracking-wider px-2 py-0.5 rounded shrink-0 ${
                    w.status === "IDLE"
                      ? "bg-emerald-950/60 text-emerald-300 border border-emerald-700/50"
                      : "bg-amber-950/60 text-amber-300 border border-amber-700/50"
                  }`}>{w.status}</span>
                  <span className="worker-item__id font-mono text-xs text-[#c9d1d9] truncate">{w.workerId}</span>
                  {w.status === "BUSY" && w.currentTaskId && (
                    <span className="worker-item__task font-mono text-xs text-[#6e7681] truncate">
                      task: <span className="worker-item__task-id text-blue-300">{w.currentTaskId}</span>
                      {w.currentAgent && (
                        <> · agent: <span className="worker-item__agent text-[#c9d1d9]">{w.currentAgent}</span></>
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
