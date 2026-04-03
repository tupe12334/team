import { useCallback, useEffect, useState } from "react";

export interface DaemonInfo {
  version: string;
  uptimeSeconds: string;
  configPath: string;
  workersCount: number;
}

export function useDaemonPanel() {
  const [info, setInfo] = useState<DaemonInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [reloading, setReloading] = useState(false);
  const [shuttingDown, setShuttingDown] = useState(false);

  const fetchInfo = useCallback(async () => {
    try {
      const res = await fetch("/api/daemon/info");
      if (!res.ok) throw new Error(await res.text());
      setInfo(await res.json());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchInfo(); }, [fetchInfo]);

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
