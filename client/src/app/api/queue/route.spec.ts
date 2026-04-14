import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => {
  class ApiError extends Error {}
  return {
    ApiError,
    queueList: vi.fn(),
    queueEnqueue: vi.fn(),
  };
});

import { ApiError, queueList, queueEnqueue } from "@/lib/grpc/client";
import { GET, POST } from "./route";

const mockList = vi.mocked(queueList);
const mockEnqueue = vi.mocked(queueEnqueue);

beforeEach(() => { vi.clearAllMocks(); });

describe("GET /api/queue", () => {
  it("returns task list on success", async () => {
    const tasks = [{ id: "t1", status: 1 }, { id: "t2", status: 3 }];
    mockList.mockResolvedValueOnce(tasks as never);
    const res = await GET();
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(tasks);
  });

  it("returns 502 on error", async () => {
    mockList.mockRejectedValueOnce(new Error("list failed"));
    const res = await GET();
    expect(res.status).toBe(502);
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockList.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const res = await GET();
    expect(res.status).toBe(503);
  });
});

describe("POST /api/queue", () => {
  it("enqueues task and returns 201", async () => {
    const task = { id: "t3", status: 1, priority: 5 };
    mockEnqueue.mockResolvedValueOnce(task as never);
    const req = new Request("http://localhost/api/queue", {
      method: "POST",
      body: JSON.stringify({ issueRef: { github: { organization: "acme", repository: "app", number: "7" } }, agent: "review", priority: 5 }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await POST(req as never);
    expect(res.status).toBe(201);
    expect(await res.json()).toEqual(task);
    expect(mockEnqueue).toHaveBeenCalledWith(
      { github: { organization: "acme", repository: "app", number: "7" } },
      "review",
      5
    );
  });

  it("returns 400 when issueRef is missing", async () => {
    const req = new Request("http://localhost/api/queue", {
      method: "POST",
      body: JSON.stringify({ agent: "review" }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await POST(req as never);
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("issueRef is required");
  });

  it("returns 400 when daemon returns a validation ApiError", async () => {
    mockEnqueue.mockRejectedValueOnce(new ApiError("unknown agent 'bad'"));
    const req = new Request("http://localhost/api/queue", {
      method: "POST",
      body: JSON.stringify({ issueRef: { github: { organization: "acme", repository: "app", number: "1" } }, agent: "bad" }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await POST(req as never);
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toContain("unknown agent");
  });

  it("returns 502 on gRPC error", async () => {
    mockEnqueue.mockRejectedValueOnce(new Error("enqueue failed"));
    const req = new Request("http://localhost/api/queue", {
      method: "POST",
      body: JSON.stringify({ issueRef: { github: { organization: "acme", repository: "app", number: "1" } } }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await POST(req as never);
    expect(res.status).toBe(502);
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockEnqueue.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const req = new Request("http://localhost/api/queue", {
      method: "POST",
      body: JSON.stringify({ issueRef: { github: { organization: "acme", repository: "app", number: "1" } } }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await POST(req as never);
    expect(res.status).toBe(503);
  });
});
