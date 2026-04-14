import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => ({
  daemonGetConfig: vi.fn(),
  daemonUpdateConfig: vi.fn(),
}));

import { daemonGetConfig, daemonUpdateConfig } from "@/lib/grpc/client";
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

  it("returns 502 on gRPC error", async () => {
    mockUpdateConfig.mockRejectedValueOnce(new Error("save failed"));
    const req = new Request("http://localhost/api/daemon/config", {
      method: "PATCH",
      body: JSON.stringify({ workersCount: 1, logLevel: "warn", enabledAgents: [] }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req);
    expect(res.status).toBe(502);
  });

  it("returns 502 when daemon rejects validation (zero workers_count)", async () => {
    mockUpdateConfig.mockRejectedValueOnce(new Error("workers_count must be at least 1"));
    const req = new Request("http://localhost/api/daemon/config", {
      method: "PATCH",
      body: JSON.stringify({ workersCount: 0, logLevel: "info", enabledAgents: [] }),
      headers: { "Content-Type": "application/json" },
    });
    const res = await PATCH(req);
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toContain("workers_count");
  });
});
