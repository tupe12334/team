import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useDaemonPanel } from "./useDaemonPanel";

afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); });

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
});
