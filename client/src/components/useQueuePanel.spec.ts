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

  it("returns raw text when queue fetch response JSON has no error field (parseError branch 2)", async () => {
    // parseError has three branches:
    //   1. JSON.parse succeeds AND typeof parsed.error === "string" → return parsed.error (tested via handleDelete)
    //   2. JSON.parse succeeds BUT parsed.error is not a string → return raw text   ← this test
    //   3. JSON.parse throws → return raw text (tested by "sets error when queue fetch fails")
    vi.stubGlobal("fetch", vi.fn().mockImplementation((url: string) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      return Promise.resolve({ ok: false, text: () => Promise.resolve('{"status":"queue service error"}'), json: () => Promise.resolve(null) });
    }));
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // No .error string field → parseError returns the raw JSON text unchanged
    expect(result.current.error).toBe('{"status":"queue service error"}');
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

  it("handleEnqueue extracts error from JSON body when POST fails (parseError branch 1)", async () => {
    // parseError branch 1: JSON body with .error string → extract the message
    // (existing "sets error when POST fails" hits branch 3 — plain text that fails JSON.parse)
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "POST")
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      return Promise.resolve({ ok: false, text: () => Promise.resolve('{"error":"agent is disabled"}') });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setOrg("myorg"); result.current.setRepo("myrepo"); result.current.setNumber("42"); });
    await act(async () => {
      await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>);
    });
    // parseError extracts the message — user sees "agent is disabled" not the raw JSON string
    expect(result.current.error).toBe("agent is disabled");
    expect(result.current.submitting).toBe(false);
  });

  it("handleEnqueue returns raw JSON when POST fails with JSON that has no error field (parseError branch 2)", async () => {
    // parseError branch 2: JSON.parse succeeds BUT parsed.error is not a string → return raw text
    // Branch 1 tests above cover the .error string extraction; this exercises the fallback
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "POST")
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      return Promise.resolve({ ok: false, text: () => Promise.resolve('{"status":"queue overloaded"}') });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setOrg("myorg"); result.current.setRepo("myrepo"); result.current.setNumber("42"); });
    await act(async () => {
      await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>);
    });
    // No .error field → parseError returns raw JSON text unchanged
    expect(result.current.error).toBe('{"status":"queue overloaded"}');
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

  it("handleDelete catches thrown network errors and sets error", async () => {
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "DELETE")
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      // DELETE throws — simulates a network-level failure (not an HTTP error response)
      return Promise.reject(new Error("network failure"));
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleDelete("t1"); });
    expect(result.current.error).toBe("network failure");
    // Task list is refetched after a catch so stale data is replaced
    expect(result.current.deletingId).toBeNull();
  });

  it("sets hardcoded fallback message when agents fetch throws a non-Error value", async () => {
    // loadAgents().catch: e instanceof Error is false → "failed to load agents"
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
      if (url === "/api/agents") return Promise.reject("not an error object");
      return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.agents).toEqual([]);
    expect(result.current.error).toBe("failed to load agents");
  });

  it("loadAgents extracts error from JSON body when agents endpoint fails (parseError branch 1)", async () => {
    // loadAgents: r.ok false → r.text() → parseError branch 1 (JSON with .error string)
    // → ApiError(message) → caught in useEffect catch → setError(e.message)
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/agents")
        return Promise.resolve({ ok: false, text: () => Promise.resolve('{"error":"agents service disabled"}') });
      return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.agents).toEqual([]);
    // parseError extracts the message — user sees "agents service disabled" not raw JSON
    expect(result.current.error).toBe("agents service disabled");
  });

  it("loadAgents returns raw JSON when agents endpoint fails with JSON that has no error field (parseError branch 2)", async () => {
    // loadAgents: r.ok false → r.text() → parseError branch 2 (JSON.parse succeeds but
    // parsed.error is not a string) → raw text returned → ApiError(rawText) → caught → setError
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/agents")
        return Promise.resolve({ ok: false, text: () => Promise.resolve('{"status":"agents degraded"}') });
      return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.agents).toEqual([]);
    // No .error field → parseError returns raw JSON text unchanged
    expect(result.current.error).toBe('{"status":"agents degraded"}');
  });

  it("handleEnqueue sets error via String() when fetch throws a non-Error value", async () => {
    // exercises the `e instanceof Error ? e.message : String(e)` catch branch
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "POST")
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      // POST rejects with a non-Error value
      // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
      return Promise.reject("enqueue network failure");
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setOrg("myorg"); result.current.setRepo("myrepo"); result.current.setNumber("42"); });
    await act(async () => {
      await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>);
    });
    expect(result.current.error).toBe("enqueue network failure");
    expect(result.current.submitting).toBe(false);
  });

  it("handleEnqueue does nothing when org/repo/number are all empty (buildIssueRef returns null)", async () => {
    const fetchMock = makeFetch(tasks, agents);
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    const callsBefore = fetchMock.mock.calls.length;
    // Default provider is GITHUB; org/repo/number are all "" → buildIssueRef returns null
    await act(async () => {
      await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>);
    });
    expect(result.current.submitting).toBe(false);
    expect(fetchMock.mock.calls.length).toBe(callsBefore); // no POST or refetch triggered
  });

  it("handleEnqueue does nothing when repo is empty with other fields filled (buildIssueRef OR guard)", async () => {
    const fetchMock = makeFetch(tasks, agents);
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setOrg("myorg"); result.current.setNumber("42"); });
    // repo is still "" — one empty field in the OR guard → buildIssueRef returns null
    const callsBefore = fetchMock.mock.calls.length;
    await act(async () => {
      await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>);
    });
    expect(result.current.submitting).toBe(false);
    expect(fetchMock.mock.calls.length).toBe(callsBefore);
  });

  it("handleEnqueue does nothing when number is empty with other fields filled (buildIssueRef OR guard third arm)", async () => {
    // The OR guard is: !org.trim() || !repo.trim() || !number.trim()
    // "all empty" hits the first arm (short-circuits on org).
    // "repo empty" hits the second arm. This test hits the third arm specifically:
    // org and repo are filled, but number is still "" → !number.trim() is true → returns null.
    const fetchMock = makeFetch(tasks, agents);
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => { result.current.setOrg("myorg"); result.current.setRepo("myrepo"); });
    // number is still "" — third arm of the OR guard → buildIssueRef returns null
    const callsBefore = fetchMock.mock.calls.length;
    await act(async () => {
      await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>);
    });
    expect(result.current.submitting).toBe(false);
    expect(fetchMock.mock.calls.length).toBe(callsBefore);
  });

  it("handleEnqueue does nothing when LINK provider has empty url (buildIssueRef returns null)", async () => {
    const fetchMock = makeFetch(tasks, agents);
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    // Switch to LINK provider but leave url empty → buildIssueRef returns null
    act(() => { result.current.setProvider("LINK"); });
    const callsBefore = fetchMock.mock.calls.length;
    await act(async () => {
      await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>);
    });
    expect(result.current.submitting).toBe(false);
    expect(fetchMock.mock.calls.length).toBe(callsBefore); // no POST or refetch triggered
  });

  it("polling clears error when queue recovers after a failed poll", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let queueCallCount = 0;
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve(agents) });
      queueCallCount++;
      if (queueCallCount === 2) {
        // Second queue call (first poll) fails
        return Promise.resolve({ ok: false, text: () => Promise.resolve("queue down"), json: () => Promise.resolve(null) });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBeNull();
    // First poll — fails
    act(() => { vi.advanceTimersByTime(5000); });
    await waitFor(() => expect(result.current.error).toBe("queue down"));
    // Second poll — recovers; fetchTasks calls setError(null) on success
    act(() => { vi.advanceTimersByTime(5000); });
    await waitFor(() => expect(result.current.error).toBeNull());
    expect(result.current.tasks).toEqual(tasks);
  });

  it("fetchTasks sets error via String() when fetch rejects with a non-Error value", async () => {
    // exercises `e instanceof Error ? e.message : String(e)` in fetchTasks catch — the String(e) branch
    // (existing "sets error when queue fetch fails" only covers the ok:false → ApiError → e.message path)
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      // GET /api/queue rejects with a non-Error value
      // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
      return Promise.reject("queue connection refused");
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.tasks).toEqual([]);
    expect(result.current.error).toBe("queue connection refused");
  });

  it("handleDelete returns raw JSON when DELETE fails with JSON that has no error field (parseError branch 2)", async () => {
    // handleDelete !res.ok path: parseError branch 2 — JSON.parse succeeds but
    // parsed.error is not a string → raw text returned as error message.
    // Existing "surfaces error" test covers branch 1 (JSON with .error).
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "DELETE")
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      return Promise.resolve({ ok: false, text: () => Promise.resolve('{"status":"delete rejected"}') });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleDelete("t1"); });
    // No .error field → parseError returns raw JSON text unchanged
    expect(result.current.error).toBe('{"status":"delete rejected"}');
    expect(result.current.deletingId).toBeNull();
  });

  it("handleDelete returns plain text when DELETE fails with non-JSON body (parseError branch 3)", async () => {
    // handleDelete !res.ok path: parseError branch 3 — JSON.parse throws → raw text returned.
    // Complements the branch 1 test ("surfaces error") and the branch 2 test above.
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "DELETE")
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      return Promise.resolve({ ok: false, text: () => Promise.resolve("delete forbidden") });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleDelete("t1"); });
    // Plain text (not JSON) → parseError returns it unchanged
    expect(result.current.error).toBe("delete forbidden");
    expect(result.current.deletingId).toBeNull();
  });

  it("handleDelete sets error via String() when fetch rejects with a non-Error value", async () => {
    // exercises `e instanceof Error ? e.message : String(e)` in handleDelete catch — the String(e) branch
    // (existing "handleDelete catches thrown network errors" uses new Error → e.message branch)
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      if (url === "/api/queue" && init?.method !== "DELETE")
        return Promise.resolve({ ok: true, json: () => Promise.resolve(tasks), text: () => Promise.resolve("") });
      // DELETE rejects with a non-Error value
      // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
      return Promise.reject("delete connection refused");
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => { await result.current.handleDelete("t1"); });
    expect(result.current.error).toBe("delete connection refused");
    expect(result.current.deletingId).toBeNull();
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

  it("handleEnqueue includes agent and priority in POST body when both are set (truthy branch of || undefined)", async () => {
    // `agent.trim() || undefined` and `priority || undefined` each have two branches:
    // - falsy: agent="" → undefined (omitted from JSON); priority=0 → undefined (omitted)
    // - truthy: agent="review" → included; priority=5 → included
    // All other enqueue tests use the defaults (empty agent, zero priority) and hit only the
    // falsy arm. This test explicitly sets both and verifies they appear in the request body.
    let capturedBody = "";
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/agents") return Promise.resolve({ ok: true, json: () => Promise.resolve(agents) });
      if (url === "/api/queue" && init?.method !== "POST")
        return Promise.resolve({ ok: true, json: () => Promise.resolve([]), text: () => Promise.resolve("") });
      const b = init?.body; capturedBody = typeof b === "string" ? b : "";
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ id: "t7", status: 1, priority: 5 }) });
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useQueuePanel());
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => {
      result.current.setOrg("myorg");
      result.current.setRepo("myrepo");
      result.current.setNumber("42");
      result.current.setAgent("review");
      result.current.setPriority(5);
    });
    await act(async () => {
      await result.current.handleEnqueue({ preventDefault: vi.fn() } as unknown as SyntheticEvent<HTMLFormElement>);
    });
    const parsed = JSON.parse(capturedBody) as { agent?: string; priority?: number };
    expect(parsed.agent).toBe("review");
    expect(parsed.priority).toBe(5);
    // After a successful enqueue the form resets to defaults
    expect(result.current.agent).toBe("");
    expect(result.current.priority).toBe(0);
  });
});
