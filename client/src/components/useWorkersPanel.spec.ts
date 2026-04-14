import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
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

  it("extracts error from JSON body when fetch returns non-ok with JSON error object", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false,
      text: () => Promise.resolve('{"error":"workers service crashed"}'),
    }));
    const { result } = renderHook(() => useWorkersPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBeNull();
    // parseError extracts the message — user sees "workers service crashed" not raw JSON
    expect(result.current.error).toBe("workers service crashed");
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

  it("sets error when fetch throws a network error directly", async () => {
    // fetch() itself rejects — exercises catch(e) with e instanceof Error path
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("connection reset")));
    const { result } = renderHook(() => useWorkersPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBe("connection reset");
  });

  it("sets error via String() when fetch throws a non-Error value", async () => {
    // exercises the `e instanceof Error ? e.message : String(e)` catch branch
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue("workers network failure"));
    const { result } = renderHook(() => useWorkersPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBe("workers network failure");
  });

  it("polling clears error when daemon recovers after a failed poll", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(workerData), text: () => Promise.resolve("") })
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve("service down") })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(workerData), text: () => Promise.resolve("") });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useWorkersPanel());
    await waitFor(() => expect(result.current.data).toEqual(workerData));
    act(() => { vi.advanceTimersByTime(5000); });
    await waitFor(() => expect(result.current.error).toBe("service down"));
    act(() => { vi.advanceTimersByTime(5000); });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    expect(result.current.error).toBeNull();
    expect(result.current.data).toEqual(workerData);
  });

  it("returns raw text when response JSON has no error field (parseError branch 2)", async () => {
    // parseError has three branches:
    //   1. JSON.parse succeeds AND typeof parsed.error === "string" → return parsed.error (tested above)
    //   2. JSON.parse succeeds BUT parsed.error is not a string → return raw text   ← this test
    //   3. JSON.parse throws → return raw text
    // Branch 2 fires when an API returns valid JSON without a top-level "error" string —
    // the user sees the raw JSON rather than a formatted message, which is the correct fallback.
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false,
      text: () => Promise.resolve('{"status":"workers crashed"}'),
    }));
    const { result } = renderHook(() => useWorkersPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBeNull();
    // No .error string field → parseError returns the raw JSON text unchanged
    expect(result.current.error).toBe('{"status":"workers crashed"}');
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
