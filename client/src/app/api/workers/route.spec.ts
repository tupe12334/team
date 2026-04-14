import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => {
  class ApiError extends Error {}
  return { ApiError, workerGetStatus: vi.fn() };
});
vi.mock("@/gen/worker", () => ({
  WorkerInfo: {},
  workerStatusToJSON: vi.fn((s: number) => (s === 1 ? "BUSY" : "IDLE")),
}));

import { workerGetStatus } from "@/lib/grpc/client";
import { GET } from "./route";

const mockGetStatus = vi.mocked(workerGetStatus);

beforeEach(() => { vi.clearAllMocks(); });

describe("GET /api/workers", () => {
  it("returns serialized worker status", async () => {
    mockGetStatus.mockResolvedValueOnce({
      total: 2,
      busy: 1,
      idle: 1,
      workers: [
        {
          workerId: "w1",
          status: 1,
          currentTaskId: "t1",
          currentAgent: "review",
          taskStartedAt: new Date("2026-01-01T00:00:00Z"),
        },
        {
          workerId: "w2",
          status: 0,
          currentTaskId: "",
          currentAgent: "",
          taskStartedAt: undefined,
        },
      ],
    });
    const res = await GET();
    expect(res.status).toBe(200);
    const body = await res.json() as {
      total: number;
      busy: number;
      idle: number;
      workers: { workerId: string; status: string; taskStartedAt: string | null }[];
    };
    expect(body.total).toBe(2);
    expect(body.busy).toBe(1);
    expect(body.workers[0].status).toBe("BUSY");
    expect(body.workers[0].taskStartedAt).toBe("2026-01-01T00:00:00.000Z");
    expect(body.workers[1].status).toBe("IDLE");
    expect(body.workers[1].taskStartedAt).toBeNull();
  });

  it("returns 400 when gRPC call returns an application error (ApiError)", async () => {
    const { ApiError } = await import("@/lib/grpc/client");
    mockGetStatus.mockRejectedValueOnce(new ApiError("worker service error"));
    const res = await GET();
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("worker service error");
  });

  it("returns 502 on error", async () => {
    mockGetStatus.mockRejectedValueOnce(new Error("worker service down"));
    const res = await GET();
    expect(res.status).toBe(502);
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockGetStatus.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const res = await GET();
    expect(res.status).toBe(503);
  });

  it("returns 502 with String() error when a non-Error value is thrown", async () => {
    // exercises `err instanceof Error ? err.message : String(err)` — the String(err) arm
    mockGetStatus.mockRejectedValueOnce("workers service crashed");
    const res = await GET();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("workers service crashed");
  });
});
