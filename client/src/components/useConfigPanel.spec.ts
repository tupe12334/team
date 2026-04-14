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
});
