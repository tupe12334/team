"use client";

import SectionHeader from "@/components/SectionHeader";
import { useQueuePanel, type Task } from "@/components/useQueuePanel";

const STATUS_STYLE: Record<Task["status"], string> = {
  QUEUED:    "bg-zinc-800 text-zinc-300 border border-zinc-600/40",
  RUNNING:   "bg-blue-950/60 text-blue-300 border border-blue-700/50",
  COMPLETED: "bg-emerald-950/60 text-emerald-300 border border-emerald-700/50",
  FAILED:    "bg-red-950/60 text-red-300 border border-red-700/50",
};

function formatIssueRef(ref: Task["issueRef"]): string {
  if (ref.ref === "repoIssue") {
    return `${ref.repoIssue.organization}/${ref.repoIssue.repository}#${ref.repoIssue.number}`;
  }
  return `${ref.provider}:${ref.id}`;
}

const live = (
  <span className="flex items-center gap-1.5 font-mono text-[10px] text-[#6e7681]">
    <span className="animate-pulse-dot w-1.5 h-1.5 rounded-full bg-orange-500/70" />
    live · 5s
  </span>
);

export default function QueuePanel() {
  const {
    tasks, error, loading, provider, setProvider,
    org, setOrg, repo, setRepo, number, setNumber,
    issueId, setIssueId, agent, setAgent,
    submitting, deletingId, handleEnqueue, handleDelete,
  } = useQueuePanel();

  return (
    <section className="border border-[#1c2736] rounded-lg bg-[#0d1117] p-6 animate-fade-up" style={{ animationDelay: "60ms" }}>
      <SectionHeader right={live}>Queue</SectionHeader>

      <form onSubmit={handleEnqueue} className="flex gap-2 mb-5 flex-wrap items-center">
        <select value={provider} onChange={(e) => { setProvider(e.target.value as typeof provider); }}
          className="font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] focus:outline-none focus:border-orange-500/60">
          <option value="GITHUB">GitHub</option>
          <option value="JIRA">Jira</option>
          <option value="CENTY">Centy</option>
        </select>

        {provider === "GITHUB" ? (
          <>
            <input type="text" value={org} onChange={(e) => { setOrg(e.target.value); }} placeholder="org" required
              className="font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] placeholder-[#6e7681] focus:outline-none focus:border-orange-500/60 w-28" />
            <input type="text" value={repo} onChange={(e) => { setRepo(e.target.value); }} placeholder="repo" required
              className="font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] placeholder-[#6e7681] focus:outline-none focus:border-orange-500/60 w-28" />
            <input type="text" value={number} onChange={(e) => { setNumber(e.target.value); }} placeholder="#" required
              className="font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] placeholder-[#6e7681] focus:outline-none focus:border-orange-500/60 w-16" />
          </>
        ) : (
          <input type="text" value={issueId} onChange={(e) => { setIssueId(e.target.value); }} placeholder="issue id" required
            className="font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] placeholder-[#6e7681] focus:outline-none focus:border-orange-500/60 w-44" />
        )}

        <input type="text" value={agent} onChange={(e) => { setAgent(e.target.value); }} placeholder="agent (optional)"
          className="font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] placeholder-[#6e7681] focus:outline-none focus:border-orange-500/60 w-36" />
        <button type="submit" disabled={submitting}
          className="font-mono text-xs px-4 py-2 rounded bg-orange-500/10 border border-orange-500/40 text-orange-400 hover:bg-orange-500/20 transition-colors disabled:opacity-40 disabled:cursor-not-allowed">
          {submitting ? "Adding…" : "+ Enqueue"}
        </button>
      </form>

      {error && (
        <p className="font-mono text-xs text-red-400 bg-red-950/30 border border-red-900/50 rounded px-3 py-2 mb-4">{error}</p>
      )}

      {loading ? (
        <div className="flex items-center gap-2 text-[#6e7681] font-mono text-xs py-4">
          <span className="animate-spin-slow inline-block w-3 h-3 border border-[#6e7681] border-t-transparent rounded-full" />
          Loading…
        </div>
      ) : tasks.length === 0 ? (
        <p className="font-mono text-xs text-[#6e7681] py-4 text-center border border-dashed border-[#1c2736] rounded">Queue is empty</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-[#1c2736]">
                {["Issue Ref", "Agent", "Status", "Priority", ""].map((h) => (
                  <th key={h} className="font-mono text-[10px] tracking-widest uppercase text-[#6e7681] pb-2 pr-4 font-normal">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="stagger">
              {tasks.map((task) => (
                <tr key={task.id} className="border-b border-[#1c2736]/50 hover:bg-white/[0.02] transition-colors">
                  <td className="font-mono text-xs text-[#c9d1d9] py-2.5 pr-4 max-w-[200px] truncate">{formatIssueRef(task.issueRef)}</td>
                  <td className="font-mono text-xs text-[#6e7681] py-2.5 pr-4">{task.agent || <span className="opacity-30">—</span>}</td>
                  <td className="py-2.5 pr-4">
                    <span className={`font-mono text-[10px] tracking-wider px-2 py-0.5 rounded ${STATUS_STYLE[task.status]}`}>{task.status}</span>
                  </td>
                  <td className="font-mono text-xs text-[#6e7681] py-2.5 pr-4">{task.priority}</td>
                  <td className="py-2.5">
                    <button onClick={() => { handleDelete(task.id); }} disabled={deletingId === task.id}
                      className="font-mono text-[10px] text-[#6e7681] hover:text-red-400 transition-colors disabled:opacity-40">
                      {deletingId === task.id ? "…" : "✕"}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
