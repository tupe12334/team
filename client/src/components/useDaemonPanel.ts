/* eslint-disable single-export/single-export */
import { useCallback, useEffect, useRef, useState } from "react";

export interface DaemonInfo {
  version: string;
  uptimeSeconds: number;
  configPath: string;
  workersCount: number;
}

export class ApiError extends Error {}

export function useDaemonPanel() {
  const [info, setInfo] = useState<DaemonInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [reloading, setReloading] = useState(false);
  const [shuttingDown, setShuttingDown] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchInfo = useCallback(async () => {
    try {
      const res = await fetch("/api/daemon/info");
      if (!res.ok) throw new ApiError(await res.text());
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      setInfo(await res.json());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchInfo();
    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    intervalRef.current = setInterval(fetchInfo, 5000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [fetchInfo]);

  const handleReload = async () => {
    setReloading(true);
    try {
      await fetch("/api/daemon/reload", { method: "POST" });
      await fetchInfo();
    } finally {
      setReloading(false);
    }
  };

  const handleShutdown = async () => {
    if (!confirm("Shut down the daemon? It will need to be restarted manually.")) return;
    setShuttingDown(true);
    try {
      await fetch("/api/daemon/shutdown", { method: "POST" });
    } finally {
      setShuttingDown(false);
    }
  };

  return { info, error, loading, reloading, shuttingDown, handleReload, handleShutdown };
}
