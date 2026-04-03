/* eslint-disable single-export/single-export */
import { useCallback, useEffect, useRef, useState } from "react";
import type { SyntheticEvent } from "react";

export class ApiError extends Error {}
export type IssueProvider = "GITHUB" | "JIRA" | "CENTY" | "LINK";

export type IssueRef =
  | { provider: "GITHUB"; ref: "repoIssue"; repoIssue: { organization: string; repository: string; number: string } }
  | { provider: "JIRA" | "CENTY"; ref: "id"; id: string }
  | { provider: "LINK"; ref: "link"; url: string };

export interface Task {
  id: string;
  issueRef: IssueRef | null;
  agent: string;
  status: "QUEUED" | "RUNNING" | "COMPLETED" | "FAILED";
  priority: number;
  createdAt: string | null;
  updatedAt: string | null;
}

export function buildIssueRef(
  provider: IssueProvider, org: string, repo: string,
  number: string, id: string, url: string
): IssueRef | null {
  if (provider === "GITHUB") {
    if (!org.trim() || !repo.trim() || !number.trim()) return null;
    return { provider: "GITHUB", ref: "repoIssue", repoIssue: { organization: org.trim(), repository: repo.trim(), number: number.trim() } };
  }
  if (provider === "LINK") {
    if (!url.trim()) return null;
    return { provider: "LINK", ref: "link", url: url.trim() };
  }
  if (!id.trim()) return null;
  return { provider, ref: "id", id: id.trim() };
}

export function useQueuePanel() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [provider, setProvider] = useState<IssueProvider>("GITHUB");
  const [org, setOrg] = useState("");
  const [repo, setRepo] = useState("");
  const [number, setNumber] = useState("");
  const [issueId, setIssueId] = useState("");
  const [url, setUrl] = useState("");
  const [agent, setAgent] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchTasks = useCallback(async () => {
    try {
      const res = await fetch("/api/queue");
      if (!res.ok) throw new ApiError(await res.text());
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      setTasks(await res.json());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchTasks();
    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    intervalRef.current = setInterval(fetchTasks, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [fetchTasks]);

  const handleEnqueue = async (e: SyntheticEvent<HTMLFormElement>) => {
    e.preventDefault();
    const issueRef = buildIssueRef(provider, org, repo, number, issueId, url);
    if (!issueRef) return;
    setSubmitting(true);
    try {
      const res = await fetch("/api/queue", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ issueRef, agent: agent.trim() || undefined }),
      });
      if (!res.ok) throw new ApiError(await res.text());
      setOrg(""); setRepo(""); setNumber(""); setIssueId(""); setUrl(""); setAgent("");
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

  return {
    tasks, error, loading, provider, setProvider,
    org, setOrg, repo, setRepo, number, setNumber,
    issueId, setIssueId, url, setUrl, agent, setAgent,
    submitting, deletingId, handleEnqueue, handleDelete,
  };
}
