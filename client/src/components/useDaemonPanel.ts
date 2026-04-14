/* eslint-disable single-export/single-export */
import { useCallback, useEffect, useRef, useState } from "react";

export interface DaemonInfo {
  version: string;
  uptimeSeconds: number;
  configPath: string;
  workersCount: number;
}

export class ApiError extends Error {}

function parseError(raw: string): string {
  try {
    // eslint-disable-next-line no-restricted-syntax
    const parsed = JSON.parse(raw) as { error?: string };
    return typeof parsed.error === "string" ? parsed.error : raw;
  } catch { return raw; }
}

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
      if (!res.ok) throw new ApiError(parseError(await res.text()));
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
      const res = await fetch("/api/daemon/reload", { method: "POST" });
      if (!res.ok) throw new ApiError(parseError(await res.text()));
      await fetchInfo();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setReloading(false);
    }
  };

  const handleShutdown = async () => {
    if (!confirm("Shut down the daemon? It will need to be restarted manually.")) return;
    setShuttingDown(true);
    try {
      const res = await fetch("/api/daemon/shutdown", { method: "POST" });
      if (!res.ok) throw new ApiError(parseError(await res.text()));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setShuttingDown(false);
    }
  };

  return { info, error, loading, reloading, shuttingDown, handleReload, handleShutdown };
}
