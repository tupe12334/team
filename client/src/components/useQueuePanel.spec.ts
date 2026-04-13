import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useQueuePanel } from "./useQueuePanel";

afterEach(() => { vi.restoreAllMocks(); });

const tasks = [
  { id: "t1", status: 1, priority: 5, agent: "review" },
  { id: "t2", status: 3, priority: 0, agent: "" },
];

const agents = [{ name: "review", description: "Code review" }, { name: "qa", description: "QA" }];

function makeFetch(queueData: unknown, agentsData: unknown) {
  return vi.fn().mockImplementation((url: string) => {
    if (url === "/api/queue") {
      return Promise.resolve({ ok: true, json: () => Promise.resolve(queueData), text: () => Promise.resolve("") });
    }
    if (url === "/api/agents") {
      return Promise.resolve({ ok: true, json: () => Promise.resolve(agentsData) });
    }
    return Promise.resolve({ ok: false, json: () => Promise.resolve(null), text: () => Promise.resolve("not found") });
  });
}

describe("useQueuePanel", () => {
  it("fetches tasks and agents on mount", async () => {
    vi.stubGlobal("fetch", makeFetch(tasks, agents));
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.tasks).toEqual(tasks);
    expect(result.current.agents).toEqual(agents);
    expect(result.current.error).toBeNull();
  });

  it("sets error when queue fetch fails", async () => {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url === "/api/agents") {
        return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      }
      return Promise.resolve({ ok: false, text: () => Promise.resolve("queue unavailable"), json: () => Promise.resolve(null) });
    }));
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe("queue unavailable");
    expect(result.current.tasks).toEqual([]);
  });

  it("polls queue every 5 seconds", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const fetchMock = makeFetch(tasks, agents);
    vi.stubGlobal("fetch", fetchMock);
    renderHook(() => useQueuePanel());
    await waitFor(() => {
      const queueCalls = fetchMock.mock.calls.filter((c: unknown[]) => c[0] === "/api/queue");
      expect(queueCalls.length).toBeGreaterThanOrEqual(1);
    });
    const callsBefore = fetchMock.mock.calls.filter((c: unknown[]) => c[0] === "/api/queue").length;
    vi.advanceTimersByTime(5000);
    await waitFor(() => {
      const callsAfter = fetchMock.mock.calls.filter((c: unknown[]) => c[0] === "/api/queue").length;
      expect(callsAfter).toBeGreaterThan(callsBefore);
    });
    vi.useRealTimers();
  });
});
