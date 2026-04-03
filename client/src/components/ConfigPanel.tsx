"use client";

import { useCallback, useEffect, useState } from "react";

interface DaemonConfig {
  workersCount: number;
  logLevel: string;
}

const LOG_LEVELS = ["error", "warn", "info", "debug", "trace"];

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

export default function ConfigPanel() {
  const [config, setConfig] = useState<DaemonConfig | null>(null);
  const [draft, setDraft] = useState<DaemonConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const fetchConfig = useCallback(async () => {
    try {
      const res = await fetch("/api/daemon/config");
      if (!res.ok) throw new Error(await res.text());
      const data: DaemonConfig = await res.json();
      setConfig(data);
      setDraft(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchConfig(); }, [fetchConfig]);

  const isDirty = draft && config && (
    draft.workersCount !== config.workersCount ||
    draft.logLevel !== config.logLevel
  );

  const handleSave = async () => {
    if (!draft) return;
    setSaving(true);
    try {
      const res = await fetch("/api/daemon/config", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(draft),
      });
      if (!res.ok) throw new Error(await res.text());
      const updated: DaemonConfig = await res.json();
      setConfig(updated);
      setDraft(updated);
      setError(null);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    if (config) setDraft(config);
  };

  return (
    <section className="border border-[#1c2736] rounded-lg bg-[#0d1117] p-6 animate-fade-up">
      <SectionHeader>Config</SectionHeader>

      {loading && (
        <div className="flex items-center gap-2 text-[#6e7681] font-mono text-xs py-4">
          <span className="animate-spin-slow inline-block w-3 h-3 border border-[#6e7681] border-t-transparent rounded-full" />
          Loading…
        </div>
      )}

      {error && (
        <p className="font-mono text-xs text-red-400 bg-red-950/30 border border-red-900/50 rounded px-3 py-2 mb-4">
          {error}
        </p>
      )}

      {draft && (
        <div className="space-y-5 animate-fade-up">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
            <div className="flex flex-col gap-2">
              <label className="font-mono text-[10px] tracking-widest uppercase text-[#6e7681]">
                Workers Count
              </label>
              <input
                type="number"
                min={1}
                value={draft.workersCount}
                onChange={(e) =>
                  setDraft({ ...draft, workersCount: Math.max(1, parseInt(e.target.value) || 1) })
                }
                className="font-mono text-sm bg-[#0d1117] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] focus:outline-none focus:border-orange-500/60 w-full"
              />
            </div>

            <div className="flex flex-col gap-2">
              <label className="font-mono text-[10px] tracking-widest uppercase text-[#6e7681]">
                Log Level
              </label>
              <select
                value={draft.logLevel}
                onChange={(e) => setDraft({ ...draft, logLevel: e.target.value })}
                className="font-mono text-sm bg-[#0d1117] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] focus:outline-none focus:border-orange-500/60 w-full"
              >
                {LOG_LEVELS.map((level) => (
                  <option key={level} value={level}>
                    {level}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div className="flex gap-3 pt-1">
            <button
              onClick={handleSave}
              disabled={saving || !isDirty}
              className="font-mono text-xs px-4 py-2 rounded border border-[#1c2736] text-[#c9d1d9] hover:border-orange-500/60 hover:text-orange-400 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            >
              {saving ? "Saving…" : saved ? "Saved" : "Save"}
            </button>
            {isDirty && (
              <button
                onClick={handleReset}
                className="font-mono text-xs px-4 py-2 rounded border border-[#1c2736] text-[#6e7681] hover:text-[#c9d1d9] transition-colors"
              >
                Reset
              </button>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
