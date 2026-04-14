import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useDaemonPanel } from "./useDaemonPanel";

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers(); });

function okFetch(data: unknown) {
  return { ok: true, json: () => Promise.resolve(data), text: () => Promise.resolve("") };
}

describe("useDaemonPanel", () => {
  it("fetches daemon info on mount", async () => {
    const info = { version: "0.1.0", uptimeSeconds: 60, configPath: "/etc/d.toml", workersCount: 4 };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(okFetch(info)));
    const { result } = renderHook(() => useDaemonPanel());
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.info).toEqual(info);
    expect(result.current.error).toBeNull();
  });

  it("sets error when fetch returns non-ok", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false, text: () => Promise.resolve("bad gateway"),
    }));
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.info).toBeNull();
    expect(result.current.error).toBe("bad gateway");
  });

  it("extracts error from JSON body when fetch returns non-ok with JSON error object", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false, text: () => Promise.resolve('{"error":"daemon crashed"}'),
    }));
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.info).toBeNull();
    // parseError extracts the message — user sees "daemon crashed" not raw JSON
    expect(result.current.error).toBe("daemon crashed");
  });

  it("clears info when daemon becomes unreachable after initial load", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(info))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve("daemon down") });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.info).toEqual(info));
    vi.advanceTimersByTime(5000);
    await waitFor(() => expect(result.current.error).toBe("daemon down"));
    expect(result.current.info).toBeNull();
  });

  it("sets error and clears info when fetch throws a network error", async () => {
    // fetch() itself rejects — exercises catch(e) with e instanceof Error path in fetchInfo.
    // Existing tests only cover the ok:false path where ApiError is thrown inside the try block;
    // this test exercises the rejection path that comes from a network-level failure.
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network failure")));
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.info).toBeNull();
    expect(result.current.error).toBe("network failure");
  });

  it("polling clears error and restores info when daemon recovers", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(info))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve("daemon down") })
      .mockResolvedValueOnce(okFetch(info));
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.info).toEqual(info));
    act(() => { vi.advanceTimersByTime(5000); });
    await waitFor(() => expect(result.current.error).toBe("daemon down"));
    expect(result.current.info).toBeNull();
    act(() => { vi.advanceTimersByTime(5000); });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    expect(result.current.error).toBeNull();
    expect(result.current.info).toEqual(info);
  });

  it("polls every 5 seconds", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    const fetchMock = vi.fn().mockResolvedValue(okFetch(info));
    vi.stubGlobal("fetch", fetchMock);
    renderHook(() => useDaemonPanel());
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    vi.advanceTimersByTime(5000);
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
  });

  it("handleReload calls POST and re-fetches", async () => {
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(info))
      .mockResolvedValueOnce(okFetch({}))
      .mockResolvedValueOnce(okFetch(info));
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleReload(); });
    expect(fetchMock).toHaveBeenCalledWith("/api/daemon/reload", { method: "POST" });
  });

  it("handleReload sets error when POST fails", async () => {
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(info))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve("reload failed") });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleReload(); });
    expect(result.current.error).toBe("reload failed");
  });

  it("handleShutdown sets error when POST fails", async () => {
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(true));
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(info))
      .mockResolvedValueOnce({ ok: false, text: () => Promise.resolve("shutdown error") });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleShutdown(); });
    expect(result.current.error).toBe("shutdown error");
  });

  it("handleShutdown calls POST and resets shuttingDown on success", async () => {
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(true));
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(info))
      .mockResolvedValueOnce({ ok: true, text: () => Promise.resolve("") });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleShutdown(); });
    expect(fetchMock).toHaveBeenCalledWith("/api/daemon/shutdown", { method: "POST" });
    expect(result.current.error).toBeNull();
    expect(result.current.shuttingDown).toBe(false);
  });

  it("handleShutdown does nothing when user cancels the confirmation dialog", async () => {
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(false));
    const fetchMock = vi.fn().mockResolvedValue(okFetch(info));
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    const callsBefore = fetchMock.mock.calls.length;
    await act(async () => { await result.current.handleShutdown(); });
    // No extra fetch beyond initial load
    expect(fetchMock.mock.calls.length).toBe(callsBefore);
    expect(result.current.shuttingDown).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("sets error via String() when fetchInfo rejects with a non-Error value", async () => {
    // exercises `e instanceof Error ? e.message : String(e)` in fetchInfo catch — the String(e) branch
    // (existing "network failure" test uses `new Error` which hits the e.message branch)
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue("daemon unavailable"));
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.info).toBeNull();
    expect(result.current.error).toBe("daemon unavailable");
  });

  it("handleReload sets error via String() when fetch rejects with a non-Error value", async () => {
    // exercises `e instanceof Error ? e.message : String(e)` in handleReload catch — the String(e) branch
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(info))
      .mockRejectedValueOnce("reload network failure");
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleReload(); });
    expect(result.current.error).toBe("reload network failure");
    expect(result.current.reloading).toBe(false);
  });

  it("handleShutdown sets error via String() when fetch rejects with a non-Error value", async () => {
    // exercises `e instanceof Error ? e.message : String(e)` in handleShutdown catch — the String(e) branch
    const info = { version: "0.1.0", uptimeSeconds: 10, configPath: "/etc/d.toml", workersCount: 2 };
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(true));
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(okFetch(info))
      .mockRejectedValueOnce("shutdown network failure");
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useDaemonPanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleShutdown(); });
    expect(result.current.error).toBe("shutdown network failure");
    expect(result.current.shuttingDown).toBe(false);
  });
});
