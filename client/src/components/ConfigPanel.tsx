"use client";

import SectionHeader from "@/components/SectionHeader";
import { useConfigPanel } from "@/components/useConfigPanel";
import type { DaemonConfig } from "@/components/useConfigPanel";

const LOG_LEVELS = ["error", "warn", "info", "debug", "trace"];
const inputCls = "config-panel__input font-mono text-sm bg-[#0d1117] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] focus:outline-none focus:border-orange-500/60 w-full";

function ConfigForm({ draft, setDraft, saving, saved, isDirty, handleSave, handleReset }: {
  draft: DaemonConfig;
  setDraft: (d: DaemonConfig) => void;
  saving: boolean; saved: boolean; isDirty: boolean | null;
  handleSave: () => Promise<void>;
  handleReset: () => void;
}) {
  return (
    <div className="config-panel__body space-y-5 animate-fade-up">
      <div className="config-panel__grid grid grid-cols-1 sm:grid-cols-2 gap-5">
        <div className="config-panel__field flex flex-col gap-2">
          <label className="config-panel__label font-mono text-[10px] tracking-widest uppercase text-[#6e7681]">Workers Count</label>
          <input type="number" min={1} value={draft.workersCount}
            onChange={(e) => { setDraft({ ...draft, workersCount: Math.max(1, parseInt(e.target.value) || 1) }); }}
            className={inputCls} />
        </div>
        <div className="config-panel__field flex flex-col gap-2">
          <label className="config-panel__label font-mono text-[10px] tracking-widest uppercase text-[#6e7681]">Log Level</label>
          <select value={draft.logLevel} onChange={(e) => { setDraft({ ...draft, logLevel: e.target.value }); }}
            className="config-panel__select font-mono text-sm bg-[#0d1117] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] focus:outline-none focus:border-orange-500/60 w-full">
            {LOG_LEVELS.map((level) => <option className="config-panel__option" key={level} value={level}>{level}</option>)}
          </select>
        </div>
      </div>
      <div className="config-panel__field flex flex-col gap-2">
        <label className="config-panel__label font-mono text-[10px] tracking-widest uppercase text-[#6e7681]">
          Enabled Agents <span className="config-panel__label-hint normal-case opacity-50">(comma-separated; empty = all)</span>
        </label>
        <input type="text" value={draft.enabledAgents.join(", ")}
          onChange={(e) => { setDraft({ ...draft, enabledAgents: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) }); }}
          placeholder="e.g. review, qa, ship" className={`${inputCls} placeholder-[#6e7681]`} />
      </div>
      <div className="config-panel__actions flex gap-3 pt-1">
        <button onClick={() => { void handleSave(); }} disabled={saving || !isDirty}
          className="config-panel__save-btn font-mono text-xs px-4 py-2 rounded border border-[#1c2736] text-[#c9d1d9] hover:border-orange-500/60 hover:text-orange-400 transition-colors disabled:opacity-40 disabled:cursor-not-allowed">
          {saving ? "Saving…" : saved ? "Saved" : "Save"}
        </button>
        {isDirty && (
          <button onClick={handleReset}
            className="config-panel__reset-btn font-mono text-xs px-4 py-2 rounded border border-[#1c2736] text-[#6e7681] hover:text-[#c9d1d9] transition-colors">
            Reset
          </button>
        )}
      </div>
    </div>
  );
}

export default function ConfigPanel() {
  const { draft, setDraft, error, loading, saving, saved, isDirty, handleSave, handleReset } = useConfigPanel();
  return (
    <section className="border border-[#1c2736] rounded-lg bg-[#0d1117] p-6 animate-fade-up">
      <SectionHeader>Config</SectionHeader>
      {loading && (
        <div className="config-panel__loading flex items-center gap-2 text-[#6e7681] font-mono text-xs py-4">
          <span className="animate-spin-slow inline-block w-3 h-3 border border-[#6e7681] border-t-transparent rounded-full" />
          Loading…
        </div>
      )}
      {error && <p className="config-panel__error font-mono text-xs text-red-400 bg-red-950/30 border border-red-900/50 rounded px-3 py-2 mb-4">{error}</p>}
      {draft && <ConfigForm draft={draft} setDraft={setDraft} saving={saving} saved={saved} isDirty={isDirty} handleSave={handleSave} handleReset={handleReset} />}
    </section>
  );
}
