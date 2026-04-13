import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => ({
  queueUpdateTask: vi.fn(),
  queueRemoveTask: vi.fn(),
}));

import { queueUpdateTask, queueRemoveTask } from "@/lib/grpc/client";
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

  it("returns 502 on error", async () => {
    mockUpdate.mockRejectedValueOnce(new Error("not found"));
    const req = new Request("http://localhost/api/queue/t1", {
      method: "PATCH",
      body: JSON.stringify({}),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req as never, makeParams("t1"));
    expect(res.status).toBe(502);
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

  it("returns 502 on error", async () => {
    mockRemove.mockRejectedValueOnce(new Error("delete failed"));
    const req = new Request("http://localhost/api/queue/t1", { method: "DELETE" });
    const res = await DELETE(req as never, makeParams("t1"));
    expect(res.status).toBe(502);
  });
});
