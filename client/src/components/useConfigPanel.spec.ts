import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useConfigPanel } from "./useConfigPanel";

afterEach(() => { vi.restoreAllMocks(); });

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

  it("isDirty is true when draft differs from config", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(baseConfig)));
    const { result } = renderHook(() => useConfigPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setDraft({ ...baseConfig, workersCount: 8 }); });
    expect(result.current.isDirty).toBeTruthy();
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
    vi.useRealTimers();
  });
});
