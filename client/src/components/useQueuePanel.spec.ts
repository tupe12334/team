import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import type { SyntheticEvent } from "react";
import { useQueuePanel } from "./useQueuePanel";

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers(); });

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

  it("handleDelete removes task on success", async () => {
    const fetchMock = makeFetch(tasks, agents);
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/queue" && !init) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      }
      if (url === "/api/agents") {
        return Promise.resolve({ ok: true, json: () => Promise.resolve(agents) });
      }
      // DELETE
      return Promise.resolve({ ok: true });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleDelete("t1"); });
    expect(result.current.tasks.find((t) => t.id === "t1")).toBeUndefined();
  });

  it("handleDelete surfaces error and refetches on failure", async () => {
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue") return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      // DELETE fails with a JSON error body
      return Promise.resolve({ ok: false, text: () => Promise.resolve('{"error":"delete not allowed"}') });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleDelete("t1"); });
    // error message extracted from JSON body
    expect(result.current.error).toBe("delete not allowed");
    // task should still be present (refetched)
    expect(result.current.tasks.find((t) => t.id === "t1")).toBeDefined();
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
  });

  it("handleEnqueue posts and clears form on success", async () => {
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve(agents) });
      if (url === "/api/queue" && (init?.method !== "POST"))
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      // POST /api/queue
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ id: "t3", status: 1, priority: 5 }) });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setOrg("myorg"); result.current.setRepo("myrepo"); result.current.setNumber("42"); });
    const preventDefault = vi.fn();
    const mockEvent = { preventDefault } as unknown as SyntheticEvent<HTMLFormElement>;
    const handleEnqueue = result.current.handleEnqueue;
    await act(async () => { await handleEnqueue(mockEvent); });
    expect(preventDefault).toHaveBeenCalled();
    expect(result.current.org).toBe("");
    expect(result.current.repo).toBe("");
    expect(result.current.number).toBe("");
    expect(result.current.submitting).toBe(false);
  });

  it("handleEnqueue sets error when POST fails", async () => {
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve(agents) });
      if (url === "/api/queue" && (init?.method !== "POST"))
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      // POST fails
      return Promise.resolve({ ok: false, text: () => Promise.resolve("enqueue failed") });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setOrg("myorg"); result.current.setRepo("myrepo"); result.current.setNumber("42"); });
    const mockEvent = { preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>;
    const handleEnqueue = result.current.handleEnqueue;
    await act(async () => { await handleEnqueue(mockEvent); });
    expect(result.current.error).toBe("enqueue failed");
    expect(result.current.submitting).toBe(false);
  });

  it("handleEnqueue with CENTY provider sends centy issueRef", async () => {
    let capturedBody = "";
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "POST")
        return Promise.resolve({ ok: true, json: () => Promise.resolve([]), text: () => Promise.resolve("") });
      const b = init?.body; capturedBody = typeof b === "string" ? b : "";
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ id: "t5", status: 0, priority: 0 }) });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setProvider("CENTY"); result.current.setOrg("acme"); result.current.setRepo("proj"); result.current.setNumber("5"); });
    await act(async () => { await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>); });
    expect(capturedBody).toContain('"centy"');
    expect(capturedBody).toContain('"acme"');
    expect(capturedBody).not.toContain('"github"');
    expect(result.current.org).toBe("");
  });

  it("surfaces error when agents endpoint fails", async () => {
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/agents") {
        return Promise.resolve({ ok: false, text: () => Promise.resolve("agents unavailable") });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // agents array remains empty; an error is set so users understand why
    expect(result.current.agents).toEqual([]);
    await waitFor(() => expect(result.current.error).toBe("agents unavailable"));
  });

  it("handleEnqueue with LINK provider sends link issueRef", async () => {
    let capturedBody = "";
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "POST")
        return Promise.resolve({ ok: true, json: () => Promise.resolve([]), text: () => Promise.resolve("") });
      const b = init?.body; capturedBody = typeof b === "string" ? b : "";
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ id: "t6", status: 0, priority: 0 }) });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setProvider("LINK"); result.current.setUrl("https://github.com/org/repo/issues/42"); });
    await act(async () => { await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>); });
    expect(capturedBody).toContain('"link"');
    expect(capturedBody).toContain("github.com/org/repo/issues/42");
    expect(result.current.url).toBe("");
  });
});
