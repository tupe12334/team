"use client";

import { useCallback, useEffect, useRef, useState } from "react";

interface Task {
  id: string;
  issueRef: string;
  agent: string;
  status: "QUEUED" | "RUNNING" | "COMPLETED" | "FAILED";
  priority: number;
  createdAt: string | null;
  updatedAt: string | null;
}

const STATUS_STYLE: Record<Task["status"], string> = {
  QUEUED:    "bg-zinc-800 text-zinc-300 border border-zinc-600/40",
  RUNNING:   "bg-blue-950/60 text-blue-300 border border-blue-700/50",
  COMPLETED: "bg-emerald-950/60 text-emerald-300 border border-emerald-700/50",
  FAILED:    "bg-red-950/60 text-red-300 border border-red-700/50",
};

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

export default function QueuePanel() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [issueRef, setIssueRef] = useState("");
  const [agent, setAgent] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchTasks = useCallback(async () => {
    try {
      const res = await fetch("/api/queue");
      if (!res.ok) throw new Error(await res.text());
      setTasks(await res.json());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTasks();
    intervalRef.current = setInterval(fetchTasks, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [fetchTasks]);

  const handleEnqueue = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!issueRef.trim()) return;
    setSubmitting(true);
    try {
      const res = await fetch("/api/queue", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ issueRef: issueRef.trim(), agent: agent.trim() || undefined }),
      });
      if (!res.ok) throw new Error(await res.text());
      setIssueRef("");
      setAgent("");
      await fetchTasks();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: string) => {
    setDeletingId(id);
    try {
      await fetch(`/api/queue/${id}`, { method: "DELETE" });
      setTasks((prev) => prev.filter((t) => t.id !== id));
    } finally {
      setDeletingId(null);
    }
  };

  const live = (
    <span className="flex items-center gap-1.5 font-mono text-[10px] text-[#6e7681]">
      <span className="animate-pulse-dot w-1.5 h-1.5 rounded-full bg-orange-500/70" />
      live · 5s
    </span>
  );

  return (
    <section className="border border-[#1c2736] rounded-lg bg-[#0d1117] p-6 animate-fade-up" style={{ animationDelay: "60ms" }}>
      <SectionHeader right={live}>Queue</SectionHeader>

      {/* ── Add task form ──────────────────────────────────────────────────── */}
      <form onSubmit={handleEnqueue} className="flex gap-2 mb-5 flex-wrap">
        <input
          type="text"
          value={issueRef}
          onChange={(e) => setIssueRef(e.target.value)}
          placeholder="issue-ref"
          required
          className="font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] placeholder-[#6e7681] focus:outline-none focus:border-orange-500/60 w-44"
        />
        <input
          type="text"
          value={agent}
          onChange={(e) => setAgent(e.target.value)}
          placeholder="agent (optional)"
          className="font-mono text-xs bg-[#07090c] border border-[#1c2736] rounded px-3 py-2 text-[#c9d1d9] placeholder-[#6e7681] focus:outline-none focus:border-orange-500/60 w-44"
        />
        <button
          type="submit"
          disabled={submitting}
          className="font-mono text-xs px-4 py-2 rounded bg-orange-500/10 border border-orange-500/40 text-orange-400 hover:bg-orange-500/20 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {submitting ? "Adding…" : "+ Enqueue"}
        </button>
      </form>

      {error && (
        <p className="font-mono text-xs text-red-400 bg-red-950/30 border border-red-900/50 rounded px-3 py-2 mb-4">
          {error}
        </p>
      )}

      {/* ── Task table ────────────────────────────────────────────────────── */}
      {loading ? (
        <div className="flex items-center gap-2 text-[#6e7681] font-mono text-xs py-4">
          <span className="animate-spin-slow inline-block w-3 h-3 border border-[#6e7681] border-t-transparent rounded-full" />
          Loading…
        </div>
      ) : tasks.length === 0 ? (
        <p className="font-mono text-xs text-[#6e7681] py-4 text-center border border-dashed border-[#1c2736] rounded">
          Queue is empty
        </p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-[#1c2736]">
                {["Issue Ref", "Agent", "Status", "Priority", ""].map((h) => (
                  <th key={h} className="font-mono text-[10px] tracking-widest uppercase text-[#6e7681] pb-2 pr-4 font-normal">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="stagger">
              {tasks.map((task) => (
                <tr key={task.id} className="border-b border-[#1c2736]/50 hover:bg-white/[0.02] transition-colors">
                  <td className="font-mono text-xs text-[#c9d1d9] py-2.5 pr-4 max-w-[160px] truncate">
                    {task.issueRef}
                  </td>
                  <td className="font-mono text-xs text-[#6e7681] py-2.5 pr-4">
                    {task.agent || <span className="opacity-30">—</span>}
                  </td>
                  <td className="py-2.5 pr-4">
                    <span className={`font-mono text-[10px] tracking-wider px-2 py-0.5 rounded ${STATUS_STYLE[task.status]}`}>
                      {task.status}
                    </span>
                  </td>
                  <td className="font-mono text-xs text-[#6e7681] py-2.5 pr-4">
                    {task.priority}
                  </td>
                  <td className="py-2.5">
                    <button
                      onClick={() => handleDelete(task.id)}
                      disabled={deletingId === task.id}
                      className="font-mono text-[10px] text-[#6e7681] hover:text-red-400 transition-colors disabled:opacity-40"
                    >
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
