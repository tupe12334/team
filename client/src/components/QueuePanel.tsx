"use client";

import type React from "react";
import SectionHeader from "@/components/SectionHeader";
import type { IssueRef } from "@/gen/queue";
import { useQueuePanel } from "@/components/useQueuePanel";

const STATUS: Record<number, { style: string; name: string }> = {
  0: { style: "bg-zinc-800 text-zinc-300 border border-zinc-600/40",              name: "QUEUED" },
  1: { style: "bg-blue-950/60 text-blue-300 border border-blue-700/50",           name: "RUNNING" },
  2: { style: "bg-emerald-950/60 text-emerald-300 border border-emerald-700/50",  name: "COMPLETED" },
  3: { style: "bg-red-950/60 text-red-300 border border-red-700/50",              name: "FAILED" },
};

function formatIssueRef(ref: IssueRef): string {
  if (ref.github) return `${ref.github.organization}/${ref.github.repository}#${ref.github.number}`;
  if (ref.centy) return `${ref.centy.organization}/${ref.centy.repository}#${ref.centy.number}`;
  return ref.jira ? `JIRA:${ref.jira.id}` : "unknown";
}

function agentOrDash(agent: string | undefined): React.ReactNode {
  if (agent) return agent;
  return dash;
}

const dash = <span className="queue-table__dash opacity-30">—</span>;
const live = <span className="queue-panel__live flex items-center gap-1.5 font-mono text-[10px] text-[#6e7681]"><span className="animate-pulse-dot w-1.5 h-1.5 rounded-full bg-orange-500/70" /> live · 5s</span>;
const inputCls = "queue-panel__input font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] placeholder-[#6e7681] focus:outline-none focus:border-orange-500/60";

function EnqueueForm({ panel }: { panel: ReturnType<typeof useQueuePanel> }) {
  const { provider, setProvider, org, setOrg, repo, setRepo, number, setNumber,
    issueId, setIssueId, url, setUrl, agent, setAgent, agents, priority, setPriority,
    submitting, handleEnqueue } = panel;
  const selectCls = "font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] focus:outline-none focus:border-orange-500/60";
  return (
    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    <form onSubmit={handleEnqueue} className="queue-panel__form flex gap-2 mb-5 flex-wrap items-center">
      {/* eslint-disable-next-line no-restricted-syntax */}
      <select value={provider} onChange={(e) => { setProvider(e.target.value as typeof provider); }}
        className={`queue-panel__provider ${selectCls}`}>
        <option className="queue-panel__option" value="GITHUB">GitHub</option><option className="queue-panel__option" value="CENTY">Centy</option>
        <option className="queue-panel__option" value="JIRA">Jira</option><option className="queue-panel__option" value="LINK">Link</option>
      </select>
      {(provider === "GITHUB" || provider === "CENTY") ? (
        <>
          <input type="text" value={org} onChange={(e) => { setOrg(e.target.value); }} placeholder="org" required className={`${inputCls} w-28`} />
          <input type="text" value={repo} onChange={(e) => { setRepo(e.target.value); }} placeholder="repo" required className={`${inputCls} w-28`} />
          <input type="text" value={number} onChange={(e) => { setNumber(e.target.value); }} placeholder="#" required className={`${inputCls} w-16`} />
        </>
      ) : provider === "LINK" ? (
        <input type="url" value={url} onChange={(e) => { setUrl(e.target.value); }} placeholder="url" required className={`${inputCls} w-64`} />
      ) : (
        <input type="text" value={issueId} onChange={(e) => { setIssueId(e.target.value); }} placeholder="issue id" required className={`${inputCls} w-44`} />
      )}
      <select value={agent} onChange={(e) => { setAgent(e.target.value); }}
        className={`queue-panel__agent ${selectCls} w-48`} title={agents.find((a) => a.name === agent)?.description}>
        <option className="queue-panel__option" value="">agent (none)</option>
        {agents.map((a) => (
          <option className="queue-panel__option" key={a.name} value={a.name} title={a.description}>{a.name}</option>
        ))}
      </select>
      <input type="number" value={priority || ""} onChange={(e) => { setPriority(Math.trunc(Number(e.target.value)) || 0); }} placeholder="priority" className={`${inputCls} w-24`} min={0} step={1} />
      <button type="submit" disabled={submitting} className="queue-panel__submit font-mono text-xs px-4 py-2 rounded bg-orange-500/10 border border-orange-500/40 text-orange-400 hover:bg-orange-500/20 transition-colors disabled:opacity-40 disabled:cursor-not-allowed">
        {submitting ? "Adding…" : "+ Enqueue"}
      </button>
    </form>
  );
}

export default function QueuePanel() {
  const panel = useQueuePanel();
  const { tasks, error, loading, handleDelete, deletingId } = panel;
  return (
    <section className="border border-[#1c2736] rounded-lg bg-[#0d1117] p-6 animate-fade-up" style={{ animationDelay: "60ms" }}>
      <SectionHeader right={live}>Queue</SectionHeader>
      <EnqueueForm panel={panel} />
      {error && <p className="queue-panel__error font-mono text-xs text-red-400 bg-red-950/30 border border-red-900/50 rounded px-3 py-2 mb-4">{error}</p>}
      {loading ? (
        <div className="queue-panel__loading flex items-center gap-2 text-[#6e7681] font-mono text-xs py-4"><span className="animate-spin-slow inline-block w-3 h-3 border border-[#6e7681] border-t-transparent rounded-full" /> Loading…</div>
      ) : tasks.length === 0 ? (
        <p className="queue-panel__empty font-mono text-xs text-[#6e7681] py-4 text-center border border-dashed border-[#1c2736] rounded">Queue is empty</p>
      ) : (
        <div className="queue-panel__table-wrap overflow-x-auto"><table className="queue-table w-full text-left border-collapse">
          <thead className="queue-table__head"><tr className="queue-table__header-row border-b border-[#1c2736]">
            {["Issue Ref", "Agent", "Status", "Priority", ""].map((h) => (
              <th key={h} className="queue-table__th font-mono text-[10px] tracking-widest uppercase text-[#6e7681] pb-2 pr-4 font-normal">{h}</th>
            ))}
          </tr></thead>
          <tbody className="stagger">{tasks.map((task) => {
            const s = STATUS[task.status];
            return (
              <tr key={task.id} className="queue-table__row border-b border-[#1c2736]/50 hover:bg-white/[0.02] transition-colors">
                <td className="queue-table__td font-mono text-xs text-[#c9d1d9] py-2.5 pr-4 max-w-[200px] truncate">{task.issueRef ? formatIssueRef(task.issueRef) : dash}</td>
                <td className="queue-table__td font-mono text-xs text-[#6e7681] py-2.5 pr-4">{agentOrDash(task.agent)}</td>
                <td className="queue-table__td py-2.5 pr-4"><span className={`font-mono text-[10px] tracking-wider px-2 py-0.5 rounded ${s.style}`}>{s.name}</span></td>
                <td className="queue-table__td font-mono text-xs text-[#6e7681] py-2.5 pr-4">{task.priority}</td>
                <td className="queue-table__td py-2.5"><button onClick={() => { void handleDelete(task.id); }} disabled={deletingId === task.id} className="queue-table__delete font-mono text-[10px] text-[#6e7681] hover:text-red-400 transition-colors disabled:opacity-40">{deletingId === task.id ? "…" : "✕"}</button></td>
              </tr>
            );
          })}</tbody>
        </table></div>
      )}
    </section>
  );
}
