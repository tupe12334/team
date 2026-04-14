import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => {
  class ApiError extends Error {}
  return {
    ApiError,
    daemonGetConfig: vi.fn(),
    daemonUpdateConfig: vi.fn(),
  };
});

import { ApiError, daemonGetConfig, daemonUpdateConfig } from "@/lib/grpc/client";
import { GET, PATCH } from "./route";

const mockGetConfig = vi.mocked(daemonGetConfig);
const mockUpdateConfig = vi.mocked(daemonUpdateConfig);

beforeEach(() => { vi.clearAllMocks(); });

describe("GET /api/daemon/config", () => {
  it("returns config on success", async () => {
    const config = { workersCount: 4, logLevel: "info", enabledAgents: [] };
    mockGetConfig.mockResolvedValueOnce(config);
    const res = await GET();
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(config);
  });

  it("returns 502 on error", async () => {
    mockGetConfig.mockRejectedValueOnce(new Error("no daemon"));
    const res = await GET();
    expect(res.status).toBe(502);
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockGetConfig.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const res = await GET();
    expect(res.status).toBe(503);
  });

  it("returns 502 with String() error when a non-Error value is thrown", async () => {
    // exercises `err instanceof Error ? err.message : String(err)` — the String(err) arm
    mockGetConfig.mockRejectedValueOnce("config service unavailable");
    const res = await GET();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("config service unavailable");
  });
});

describe("PATCH /api/daemon/config", () => {
  it("updates config and returns updated value", async () => {
    const updated = { workersCount: 8, logLevel: "debug", enabledAgents: ["review"] };
    mockUpdateConfig.mockResolvedValueOnce(updated);
    const req = new Request("http://localhost/api/daemon/config", {
      method: "PATCH",
      body: JSON.stringify(updated),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req);
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(updated);
    expect(mockUpdateConfig).toHaveBeenCalledWith(updated);
  });

  it("returns 502 on gRPC transport error", async () => {
    mockUpdateConfig.mockRejectedValueOnce(new Error("save failed"));
    const req = new Request("http://localhost/api/daemon/config", {
      method: "PATCH",
      body: JSON.stringify({ workersCount: 1, logLevel: "warn", enabledAgents: [] }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req);
    expect(res.status).toBe(502);
  });

  it("returns 400 when daemon rejects validation (ApiError)", async () => {
    mockUpdateConfig.mockRejectedValueOnce(new ApiError("workers_count must be at least 1"));
    const req = new Request("http://localhost/api/daemon/config", {
      method: "PATCH",
      body: JSON.stringify({ workersCount: 0, logLevel: "info", enabledAgents: [] }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req);
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toContain("workers_count");
  });

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockUpdateConfig.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const req = new Request("http://localhost/api/daemon/config", {
      method: "PATCH",
      body: JSON.stringify({ workersCount: 2, logLevel: "info", enabledAgents: [] }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req);
    expect(res.status).toBe(503);
  });

  it("returns 502 with String() error when a non-Error value is thrown", async () => {
    // exercises `err instanceof Error ? err.message : String(err)` — the String(err) arm
    mockUpdateConfig.mockRejectedValueOnce("update service unavailable");
    const req = new Request("http://localhost/api/daemon/config", {
      method: "PATCH",
      body: JSON.stringify({ workersCount: 4, logLevel: "info", enabledAgents: [] }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req);
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("update service unavailable");
  });
});
