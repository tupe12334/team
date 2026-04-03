import { useCallback, useEffect, useState } from "react";

export interface DaemonConfig {
  workersCount: number;
  logLevel: string;
}

export function useConfigPanel() {
  const [config, setConfig] = useState<DaemonConfig | null>(null);
  const [draft, setDraft] = useState<DaemonConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const fetchConfig = useCallback(async () => {
    try {
      const res = await fetch("/api/daemon/config");
      if (!res.ok) throw new Error(await res.text());
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

  useEffect(() => { fetchConfig(); }, [fetchConfig]);

  const isDirty = draft && config && (
    draft.workersCount !== config.workersCount ||
    draft.logLevel !== config.logLevel
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
      if (!res.ok) throw new Error(await res.text());
      const updated: DaemonConfig = await res.json();
      setConfig(updated);
      setDraft(updated);
      setError(null);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => { if (config) setDraft(config); };

  return { draft, setDraft, error, loading, saving, saved, isDirty, handleSave, handleReset };
}
