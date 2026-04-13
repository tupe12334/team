import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => ({
  daemonGetAvailableAgents: vi.fn(),
}));

import { daemonGetAvailableAgents } from "@/lib/grpc/client";
import { GET } from "./route";

const mockGetAgents = vi.mocked(daemonGetAvailableAgents);

beforeEach(() => { vi.clearAllMocks(); });

describe("GET /api/agents", () => {
  it("returns agent list on success", async () => {
    const agents = [{ name: "review", description: "Review" }, { name: "qa", description: "QA" }];
    mockGetAgents.mockResolvedValueOnce(agents);
    const res = await GET();
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(agents);
  });

  it("returns 502 when gRPC call throws", async () => {
    mockGetAgents.mockRejectedValueOnce(new Error("connection refused"));
    const res = await GET();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("connection refused");
  });

  it("returns 502 with string error for non-Error throws", async () => {
    mockGetAgents.mockRejectedValueOnce("unknown failure");
    const res = await GET();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("unknown failure");
  });
});
