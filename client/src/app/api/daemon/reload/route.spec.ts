import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => ({
  daemonReloadConfig: vi.fn(),
}));

import { daemonReloadConfig } from "@/lib/grpc/client";
import { POST } from "./route";

const mockReload = vi.mocked(daemonReloadConfig);

beforeEach(() => { vi.clearAllMocks(); });

describe("POST /api/daemon/reload", () => {
  it("returns ok on success", async () => {
    mockReload.mockResolvedValueOnce(undefined);
    const res = await POST();
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ ok: true });
  });

  it("returns 502 on error", async () => {
    mockReload.mockRejectedValueOnce(new Error("reload failed"));
    const res = await POST();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("reload failed");
  });
});
