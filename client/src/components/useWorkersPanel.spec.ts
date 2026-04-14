import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useWorkersPanel } from "./useWorkersPanel";

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers(); });

const workerData = {
  total: 2,
  busy: 1,
  idle: 1,
  workers: [
    { workerId: "w1", status: "BUSY" as const, currentTaskId: "t1", currentAgent: "review", taskStartedAt: "2026-01-01T00:00:00.000Z" },
    { workerId: "w2", status: "IDLE" as const, currentTaskId: "", currentAgent: "", taskStartedAt: null },
  ],
};

describe("useWorkersPanel", () => {
  it("fetches worker status on mount", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(workerData),
      text: () => Promise.resolve(""),
    }));
    const { result } = renderHook(() => useWorkersPanel());
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toEqual(workerData);
    expect(result.current.error).toBeNull();
  });

  it("sets error when fetch fails", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false,
      text: () => Promise.resolve("service unavailable"),
    }));
    const { result } = renderHook(() => useWorkersPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBe("service unavailable");
  });

  it("clears data when workers service becomes unreachable after initial load", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(workerData), text: () => Promise.resolve("") })
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve("service down") });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useWorkersPanel());
    await waitFor(() => expect(result.current.data).toEqual(workerData));
    vi.advanceTimersByTime(5000);
    await waitFor(() => expect(result.current.error).toBe("service down"));
    expect(result.current.data).toBeNull();
  });

  it("polls every 5 seconds", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(workerData),
      text: () => Promise.resolve(""),
    });
    vi.stubGlobal("fetch", fetchMock);
    renderHook(() => useWorkersPanel());
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    vi.advanceTimersByTime(5000);
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
  });
});
