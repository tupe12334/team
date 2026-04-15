import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => {
  class ApiError extends Error {}
  return {
    ApiError,
    queueUpdateTask: vi.fn(),
    queueRemoveTask: vi.fn(),
  };
});

import { ApiError, queueUpdateTask, queueRemoveTask } from "@/lib/grpc/client";
import { PATCH, DELETE } from "./route";

const mockUpdate = vi.mocked(queueUpdateTask);
const mockRemove = vi.mocked(queueRemoveTask);

function makeParams(id: string) {
  return { params: Promise.resolve({ id }) };
}

beforeEach(() => { vi.clearAllMocks(); });

describe("PATCH /api/queue/[id]", () => {
  it("updates task and returns result", async () => {
    const task = { id: "t1", status: 2, agent: "qa", priority: 3 };
    mockUpdate.mockResolvedValueOnce(task as never);
    const req = new Request("http://localhost/api/queue/t1", {
      method: "PATCH",
      body: JSON.stringify({ agent: "qa", priority: 3 }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req as never, makeParams("t1"));
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(task);
    expect(mockUpdate).toHaveBeenCalledWith("t1", { agent: "qa", priority: 3 });
  });

  it("returns 400 when daemon returns a validation ApiError", async () => {
    mockUpdate.mockRejectedValueOnce(new ApiError("unknown agent 'bad'"));
    const req = new Request("http://localhost/api/queue/t1", {
      method: "PATCH",
      body: JSON.stringify({ agent: "bad" }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req as never, makeParams("t1"));
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toContain("unknown agent");
  });

  it("returns 502 on transport error", async () => {
    mockUpdate.mockRejectedValueOnce(new Error("not found"));
    const req = new Request("http://localhost/api/queue/t1", {
      method: "PATCH",
      body: JSON.stringify({}),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req as never, makeParams("t1"));
    expect(res.status).toBe(502);
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockUpdate.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const req = new Request("http://localhost/api/queue/t1", {
      method: "PATCH",
      body: JSON.stringify({}),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req as never, makeParams("t1"));
    expect(res.status).toBe(503);
  });

  it("returns 502 with String() error when a non-Error value is thrown", async () => {
    // exercises `err instanceof Error ? err.message : String(err)` — the String(err) arm
    mockUpdate.mockRejectedValueOnce("update service unavailable");
    const req = new Request("http://localhost/api/queue/t1", {
      method: "PATCH",
      body: JSON.stringify({}),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req as never, makeParams("t1"));
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("update service unavailable");
  });
});

describe("DELETE /api/queue/[id]", () => {
  it("removes task and returns 204", async () => {
    mockRemove.mockResolvedValueOnce(undefined);
    const req = new Request("http://localhost/api/queue/t1", { method: "DELETE" });
    const res = await DELETE(req as never, makeParams("t1"));
    expect(res.status).toBe(204);
    expect(mockRemove).toHaveBeenCalledWith("t1");
  });

  it("returns 409 when task is running (gRPC FAILED_PRECONDITION)", async () => {
    mockRemove.mockRejectedValueOnce(Object.assign(new Error("cannot remove running task"), { code: 9 }));
    const req = new Request("http://localhost/api/queue/t1", { method: "DELETE" });
    const res = await DELETE(req as never, makeParams("t1"));
    expect(res.status).toBe(409);
    const body = await res.json() as { error: string };
    expect(body.error).toContain("cannot remove running task");
  });

  it("returns 404 when task not found (gRPC NOT_FOUND)", async () => {
    mockRemove.mockRejectedValueOnce(Object.assign(new Error("task not found"), { code: 5 }));
    const req = new Request("http://localhost/api/queue/t1", { method: "DELETE" });
    const res = await DELETE(req as never, makeParams("t1"));
    expect(res.status).toBe(404);
  });

  it("returns 502 on transport error", async () => {
    mockRemove.mockRejectedValueOnce(new Error("delete failed"));
    const req = new Request("http://localhost/api/queue/t1", { method: "DELETE" });
    const res = await DELETE(req as never, makeParams("t1"));
    expect(res.status).toBe(502);
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockRemove.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const req = new Request("http://localhost/api/queue/t1", { method: "DELETE" });
    const res = await DELETE(req as never, makeParams("t1"));
    expect(res.status).toBe(503);
  });

  it("returns 502 with String() error when a non-Error value is thrown", async () => {
    // exercises `err instanceof Error ? err.message : String(err)` — the String(err) arm
    mockRemove.mockRejectedValueOnce("remove service unavailable");
    const req = new Request("http://localhost/api/queue/t1", { method: "DELETE" });
    const res = await DELETE(req as never, makeParams("t1"));
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("remove service unavailable");
  });

  it("returns 400 when daemon returns an application-level error (ApiError)", async () => {
    // Exercises the `err instanceof ApiError ? 400` branch in the DELETE catch block.
    // PATCH has this test but DELETE did not — the ApiError arm was dead code in tests.
    // In production this fires when queueRemoveTask rejects with a domain-level error
    // rather than a transport/gRPC status error (e.g. a future business-rule rejection).
    mockRemove.mockRejectedValueOnce(new ApiError("cannot remove a protected task"));
    const req = new Request("http://localhost/api/queue/t1", { method: "DELETE" });
    const res = await DELETE(req as never, makeParams("t1"));
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("cannot remove a protected task");
  });
});
