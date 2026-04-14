/* eslint-disable single-export/single-export */
import { useCallback, useEffect, useRef, useState } from "react";

export class ApiError extends Error {}

function parseError(raw: string): string {
  try {
    // eslint-disable-next-line no-restricted-syntax
    const parsed = JSON.parse(raw) as { error?: string };
    return typeof parsed.error === "string" ? parsed.error : raw;
  } catch { return raw; }
}

export interface WorkerInfo {
  workerId: string;
  status: "IDLE" | "BUSY";
  currentTaskId: string;
  currentAgent: string;
  taskStartedAt: string | null;
}

export interface WorkerStatusData {
  total: number;
  busy: number;
  idle: number;
  workers: WorkerInfo[];
}

export function useWorkersPanel() {
  const [data, setData] = useState<WorkerStatusData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const res = await fetch("/api/workers");
      if (!res.ok) throw new ApiError(parseError(await res.text()));
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      setData(await res.json());
      setError(null);
    } catch (e) {
      setData(null); // clear stale data so UI shows the error, not outdated values
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchStatus();
    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    intervalRef.current = setInterval(fetchStatus, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [fetchStatus]);

  return { data, error, loading };
}
