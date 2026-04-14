/* eslint-disable single-export/single-export */
import { useCallback, useEffect, useRef, useState } from "react";
import type { SyntheticEvent } from "react";
import type { IssueRef, IssueRefInput } from "@/gen/queue";
import type { AgentInfo } from "@/lib/grpc/client";

export class ApiError extends Error {}
export type IssueProvider = "GITHUB" | "CENTY" | "LINK";

function parseError(raw: string): string {
  try {
    // eslint-disable-next-line no-restricted-syntax
    const parsed = JSON.parse(raw) as { error?: string };
    return typeof parsed.error === "string" ? parsed.error : raw;
  } catch { return raw; }
}

export interface Task {
  id: string; issueRef?: IssueRef; agent?: string;
  status: number; priority: number; createdAt?: string; updatedAt?: string;
}

function buildIssueRef(provider: IssueProvider, org: string, repo: string, number: string, url: string): IssueRefInput | null {
  if (provider === "GITHUB" || provider === "CENTY") {
    if (!org.trim() || !repo.trim() || !number.trim()) return null;
    const ref = { organization: org.trim(), repository: repo.trim(), number: number.trim() };
    return provider === "GITHUB" ? { github: ref } : { centy: ref };
  }
  if (!url.trim()) return null;
  return { link: { url: url.trim() } };
}

async function loadAgents(): Promise<AgentInfo[]> {
  const r = await fetch("/api/agents");
  if (!r.ok) throw new Error(parseError(await r.text()));
  // eslint-disable-next-line @typescript-eslint/no-unsafe-return
  return r.json();
}

export function useQueuePanel() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [provider, setProvider] = useState<IssueProvider>("GITHUB");
  const [org, setOrg] = useState(""); const [repo, setRepo] = useState(""); const [number, setNumber] = useState("");
  const [url, setUrl] = useState("");
  const [agent, setAgent] = useState(""); const [priority, setPriority] = useState(0);
  const [submitting, setSubmitting] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchTasks = useCallback(async () => {
    try {
      const res = await fetch("/api/queue");
      if (!res.ok) throw new ApiError(parseError(await res.text()));
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      setTasks(await res.json()); setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally { setLoading(false); }
  }, []);

  useEffect(() => {
    void loadAgents().then(setAgents).catch((e: unknown) => {
      setError(e instanceof Error ? e.message : "failed to load agents");
    });
    void fetchTasks();
    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    intervalRef.current = setInterval(fetchTasks, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [fetchTasks]);

  const handleEnqueue = async (e: SyntheticEvent<HTMLFormElement>) => {
    e.preventDefault();
    const issueRef = buildIssueRef(provider, org, repo, number, url);
    if (!issueRef) return;
    setSubmitting(true);
    try {
      const res = await fetch("/api/queue", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ issueRef, agent: agent.trim() || undefined, priority: priority || undefined }),
      });
      if (!res.ok) throw new ApiError(parseError(await res.text()));
      setOrg(""); setRepo(""); setNumber(""); setUrl(""); setAgent(""); setPriority(0);
      await fetchTasks();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally { setSubmitting(false); }
  };

  const handleDelete = async (id: string) => {
    setDeletingId(id);
    try {
      const res = await fetch(`/api/queue/${id}`, { method: "DELETE" });
      if (res.ok) {
        setTasks((prev) => prev.filter((t) => t.id !== id));
      } else {
        const errMsg = parseError(await res.text());
        await fetchTasks();
        setError(errMsg);
      }
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e);
      await fetchTasks();
      setError(errMsg);
    } finally { setDeletingId(null); }
  };

  return {
    tasks, agents, error, loading, provider, setProvider,
    org, setOrg, repo, setRepo, number, setNumber,
    url, setUrl, agent, setAgent,
    priority, setPriority, submitting, deletingId, handleEnqueue, handleDelete,
  };
}
