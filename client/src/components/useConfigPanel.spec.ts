import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useConfigPanel } from "./useConfigPanel";

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers(); });

function okFetch(data: unknown) {
  return { ok: true, json: () => Promise.resolve(data), text: () => Promise.resolve("") };
}

const baseConfig = { workersCount: 4, logLevel: "info", enabledAgents: [] as string[] };

describe("useConfigPanel", () => {
  it("loads config on mount", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(baseConfig)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.draft).toEqual(baseConfig);
    expect(result.current.error).toBeNull();
  });

  it("sets error when fetch fails", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false, text: () => Promise.resolve("forbidden"),
    }));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe("forbidden");
  });

  it("extracts error from JSON body when initial fetch returns non-ok with JSON error object", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false, text: () => Promise.resolve('{"error":"config unavailable"}'),
    }));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // parseError in fetchConfig extracts the message — user sees "config unavailable" not raw JSON
    expect(result.current.error).toBe("config unavailable");
  });

  it("returns raw text when response JSON has no error field (parseError branch 2)", async () => {
    // parseError has three branches:
    //   1. JSON.parse succeeds AND typeof parsed.error === "string" → return parsed.error (tested above)
    //   2. JSON.parse succeeds BUT parsed.error is not a string → return raw text   ← this test
    //   3. JSON.parse throws → return raw text
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false, text: () => Promise.resolve('{"status":"config unavailable"}'),
    }));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // No .error string field → parseError returns the raw JSON text unchanged
    expect(result.current.error).toBe('{"status":"config unavailable"}');
  });

  it("handleSave does nothing when draft is null (fetch not yet resolved)", async () => {
    // draft starts null; fetchConfig never resolves → handleSave hits `if (!draft) return`
    // eslint-disable-next-line @typescript-eslint/no-empty-function
    vi.stubGlobal("fetch", vi.fn().mockImplementation(() => new Promise<never>(() => {})));
    const { result } = renderHook(() => useConfigPanel());
    // draft is still null — loading hasn't completed
    await act(async () => { await result.current.handleSave(); });
    expect(result.current.saving).toBe(false);
    expect(result.current.saved).toBe(false);
  });

  it("isDirty is false when draft matches config", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(baseConfig)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.isDirty).toBeFalsy();
  });

  it("isDirty is true when workersCount differs from config", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(baseConfig)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, workersCount: 8 }); });
    expect(result.current.isDirty).toBeTruthy();
  });

  it("isDirty is true when logLevel differs from config", async () => {
    // The isDirty condition uses three || arms: workersCount, logLevel, enabledAgents.
    // "isDirty is true when workersCount differs" covers the first arm;
    // "isDirty is true when enabledAgents differs" covers the third arm.
    // This test covers the second arm specifically, ensuring logLevel changes are detected.
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(baseConfig)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, logLevel: "debug" }); });
    expect(result.current.isDirty).toBeTruthy();
  });

  it("isDirty is true when enabledAgents differs from config", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(baseConfig)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, enabledAgents: ["review", "qa"] }); });
    expect(result.current.isDirty).toBeTruthy();
  });

  it("isDirty is false when enabledAgents order and content match config", async () => {
    const configWithAgents = { ...baseConfig, enabledAgents: ["review", "qa"] };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(configWithAgents)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // draft is initialized to the same value as config
    expect(result.current.isDirty).toBeFalsy();
  });

  it("handleReset restores draft to config", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(baseConfig)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, workersCount: 8 }); });
    act(() => { result.current.handleReset(); });
    expect(result.current.draft).toEqual(baseConfig);
  });

  it("handleReset does nothing when config is null (if (config) false branch)", async () => {
    // When the initial fetch fails, config stays null. Calling handleReset() must be a
    // no-op — `if (config)` is false, so setDraft is never called and draft remains null.
    // The existing "handleReset restores draft to config" test only covers the truthy arm
    // (config is loaded successfully → setDraft(config) fires).
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, text: () => Promise.resolve("forbidden") }));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // config is null (fetch failed); draft is also null
    expect(result.current.draft).toBeNull();
    act(() => { result.current.handleReset(); });
    // handleReset must not change draft — no setDraft call when config is null
    expect(result.current.draft).toBeNull();
  });

  it("handleSave PATCHes config and sets saved flag", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const updated = { workersCount: 8, logLevel: "debug", enabledAgents: ["review"] };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockResolvedValueOnce(okFetch(updated));
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft(updated); });
    await act(async () => { await result.current.handleSave(); });
    expect(result.current.saved).toBe(true);
    expect(fetchMock).toHaveBeenCalledWith("/api/daemon/config", expect.objectContaining({ method: "PATCH" }));
    act(() => { vi.advanceTimersByTime(2001); });
    expect(result.current.saved).toBe(false);
  });

  it("polls config every 10 seconds without overwriting unsaved draft edits", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const updated = { ...baseConfig, workersCount: 8 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))  // initial load
      .mockResolvedValueOnce(okFetch(updated));      // first poll
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // Simulate user editing draft
    act(() => { result.current.setDraft({ ...baseConfig, workersCount: 99 }); });
    // Advance timer to trigger poll
    vi.advanceTimersByTime(10000);
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    // Draft preserves user edits; config reflects server state
    expect(result.current.draft?.workersCount).toBe(99);
  });

  it("handleSave sets error when PATCH fails", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve("save failed") });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, workersCount: 8 }); });
    await act(async () => { await result.current.handleSave(); });
    expect(result.current.error).toBe("save failed");
    expect(result.current.saved).toBe(false);
    expect(result.current.saving).toBe(false);
  });

  it("handleSave extracts error from JSON body when PATCH fails (parseError branch 1)", async () => {
    // parseError branch 1: JSON body with .error string → extract the message
    // (existing "sets error when PATCH fails" hits branch 3 — plain text that fails JSON.parse)
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve('{"error":"workers_count must be positive"}') });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, workersCount: 8 }); });
    await act(async () => { await result.current.handleSave(); });
    // parseError extracts the message — user sees "workers_count must be positive" not raw JSON
    expect(result.current.error).toBe("workers_count must be positive");
    expect(result.current.saving).toBe(false);
    expect(result.current.saved).toBe(false);
  });

  it("handleSave returns raw JSON when PATCH fails with JSON that has no error field (parseError branch 2)", async () => {
    // parseError branch 2: JSON.parse succeeds BUT parsed.error is not a string → return raw text
    // Branch 1 above covers .error string extraction; this exercises the fallback path
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve('{"status":"config locked"}') });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, workersCount: 8 }); });
    await act(async () => { await result.current.handleSave(); });
    // No .error field → parseError returns raw JSON text unchanged
    expect(result.current.error).toBe('{"status":"config locked"}');
    expect(result.current.saving).toBe(false);
    expect(result.current.saved).toBe(false);
  });

  it("isDirty is false when enabledAgents differ only in order", async () => {
    const configWithAgents = { ...baseConfig, enabledAgents: ["review", "qa"] };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(configWithAgents)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // Reorder agents in draft — same set, different order
    act(() => { result.current.setDraft({ ...configWithAgents, enabledAgents: ["qa", "review"] }); });
    expect(result.current.isDirty).toBeFalsy();
  });

  it("isDirty is true when enabledAgents have different content despite same length", async () => {
    const configWithAgents = { ...baseConfig, enabledAgents: ["review", "qa"] };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(configWithAgents)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...configWithAgents, enabledAgents: ["review", "ship"] }); });
    expect(result.current.isDirty).toBeTruthy();
  });

  it("polling sets error when daemon becomes unavailable during background refresh", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve(JSON.stringify({ error: "daemon unavailable" })) });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBeNull();
    act(() => { vi.advanceTimersByTime(10000); });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(result.current.error).toBe("daemon unavailable");
    expect(result.current.draft).toEqual(baseConfig);
  });

  it("polling sets error when fetch throws a network error (catch path)", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    // initial fetchConfig succeeds; poll tick: fetch() itself rejects → polling catch(e) fires
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockRejectedValueOnce(new Error("network failure"));
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBeNull();
    act(() => { vi.advanceTimersByTime(10000); });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(result.current.error).toBe("network failure");
    // draft is preserved — polling catch does not clear it
    expect(result.current.draft).toEqual(baseConfig);
  });

  it("polling clears error when daemon recovers after a failed poll", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve(JSON.stringify({ error: "daemon down" })) })
      .mockResolvedValueOnce(okFetch(baseConfig));
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { vi.advanceTimersByTime(10000); });
    await waitFor(() => expect(result.current.error).toBe("daemon down"));
    act(() => { vi.advanceTimersByTime(10000); });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    expect(result.current.error).toBeNull();
  });

  it("sets error via String() when fetchConfig rejects with a non-Error value", async () => {
    // exercises `e instanceof Error ? e.message : String(e)` in fetchConfig catch — the String(e) branch
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue("config network failure"));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe("config network failure");
    expect(result.current.draft).toBeNull();
  });

  it("sets error via String() when polling fetch rejects with a non-Error value", async () => {
    // exercises `e instanceof Error ? e.message : String(e)` in the setInterval catch — the String(e) branch
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockRejectedValueOnce("poll network failure");
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { vi.advanceTimersByTime(10000); });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(result.current.error).toBe("poll network failure");
    // draft is preserved — polling catch does not clear it
    expect(result.current.draft).toEqual(baseConfig);
  });

  it("handleSave sets error via String() when fetch rejects with a non-Error value", async () => {
    // exercises `e instanceof Error ? e.message : String(e)` in handleSave catch — the String(e) branch
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(baseConfig))
      .mockRejectedValueOnce("save network failure");
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, workersCount: 8 }); });
    await act(async () => { await result.current.handleSave(); });
    expect(result.current.error).toBe("save network failure");
    expect(result.current.saving).toBe(false);
    expect(result.current.saved).toBe(false);
  });
});
