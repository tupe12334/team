import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => {
  class ApiError extends Error {}
  return { ApiError, daemonGetInfo: vi.fn() };
});

import { daemonGetInfo } from "@/lib/grpc/client";
import { GET } from "./route";

const mockGetInfo = vi.mocked(daemonGetInfo);

beforeEach(() => { vi.clearAllMocks(); });

describe("GET /api/daemon/info", () => {
  it("returns daemon info on success", async () => {
    const info = { version: "0.1.0", uptimeSeconds: 120, configPath: "/etc/d.toml", workersCount: 2 };
    mockGetInfo.mockResolvedValueOnce(info);
    const res = await GET();
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(info);
  });

  it("returns 502 when gRPC throws", async () => {
    mockGetInfo.mockRejectedValueOnce(new Error("timeout"));
    const res = await GET();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("timeout");
  });

  it("returns 400 when gRPC call returns an application error (ApiError)", async () => {
    const { ApiError } = await import("@/lib/grpc/client");
    mockGetInfo.mockRejectedValueOnce(new ApiError("daemon error"));
    const res = await GET();
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("daemon error");
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockGetInfo.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const res = await GET();
    expect(res.status).toBe(503);
  });
});
