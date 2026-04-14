/* eslint-disable single-export/single-export */
import { useCallback, useEffect, useRef, useState } from "react";

export interface DaemonConfig {
  workersCount: number;
  logLevel: string;
  enabledAgents: string[];
}

export class ApiError extends Error {}

// Routes return { error: string } JSON; extract the message so users don't see raw JSON.
function parseError(raw: string): string {
  try {
    // eslint-disable-next-line no-restricted-syntax
    const parsed = JSON.parse(raw) as { error?: string };
    return typeof parsed.error === "string" ? parsed.error : raw;
  } catch { return raw; }
}

export function useConfigPanel() {
  const [config, setConfig] = useState<DaemonConfig | null>(null);
  const [draft, setDraft] = useState<DaemonConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchConfig = useCallback(async () => {
    try {
      const res = await fetch("/api/daemon/config");
      if (!res.ok) throw new ApiError(parseError(await res.text()));
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const data: DaemonConfig = await res.json();
      setConfig(data);
      setDraft(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchConfig();
    // Poll every 10 s — updates config (server source of truth) but NOT draft,
    // so unsaved user edits are preserved across background refreshes.
    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    intervalRef.current = setInterval(async () => {
      try {
        const res = await fetch("/api/daemon/config");
        if (!res.ok) return;
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        const data: DaemonConfig = await res.json();
        setConfig(data);
        setError(null);
      } catch { /* ignore transient poll errors */ }
    }, 10000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [fetchConfig]);

  const isDirty = draft && config && (
    draft.workersCount !== config.workersCount ||
    draft.logLevel !== config.logLevel ||
    draft.enabledAgents.join(",") !== config.enabledAgents.join(",")
  );

  const handleSave = async () => {
    if (!draft) return;
    setSaving(true);
    try {
      const res = await fetch("/api/daemon/config", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(draft),
      });
      if (!res.ok) throw new ApiError(parseError(await res.text()));
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const updated: DaemonConfig = await res.json();
      setConfig(updated);
      setDraft(updated);
      setError(null);
      setSaved(true);
      setTimeout(() => { setSaved(false); }, 2000);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => { if (config) setDraft(config); };

  return { draft, setDraft, error, loading, saving, saved, isDirty, handleSave, handleReset };
}
