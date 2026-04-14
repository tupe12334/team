import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => {
  class ApiError extends Error {}
  return { ApiError, daemonShutdown: vi.fn() };
});

import { daemonShutdown } from "@/lib/grpc/client";
import { POST } from "./route";

const mockShutdown = vi.mocked(daemonShutdown);

beforeEach(() => { vi.clearAllMocks(); });

describe("POST /api/daemon/shutdown", () => {
  it("returns ok on success", async () => {
    mockShutdown.mockResolvedValueOnce(undefined);
    const res = await POST();
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ ok: true });
  });

  it("returns 502 on error", async () => {
    mockShutdown.mockRejectedValueOnce(new Error("process gone"));
    const res = await POST();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("process gone");
  });

  it("returns 400 when gRPC call returns an application error (ApiError)", async () => {
    const { ApiError } = await import("@/lib/grpc/client");
    mockShutdown.mockRejectedValueOnce(new ApiError("shutdown error"));
    const res = await POST();
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("shutdown error");
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockShutdown.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const res = await POST();
    expect(res.status).toBe(503);
  });

  it("returns 502 with String() error when a non-Error value is thrown", async () => {
    // exercises `err instanceof Error ? err.message : String(err)` — the String(err) arm
    mockShutdown.mockRejectedValueOnce("shutdown service unavailable");
    const res = await POST();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("shutdown service unavailable");
  });
});
