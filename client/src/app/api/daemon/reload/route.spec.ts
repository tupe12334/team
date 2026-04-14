import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/grpc/client", () => {
  class ApiError extends Error {}
  return { ApiError, daemonReloadConfig: vi.fn() };
});

import { ApiError, daemonReloadConfig } from "@/lib/grpc/client";
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

  it("returns 503 when daemon is unavailable (gRPC UNAVAILABLE)", async () => {
    mockReload.mockRejectedValueOnce(Object.assign(new Error("daemon down"), { code: 14 }));
    const res = await POST();
    expect(res.status).toBe(503);
  });

  it("returns 400 when daemon returns an application error (malformed config)", async () => {
    mockReload.mockRejectedValueOnce(new ApiError("malformed config at '/path/config.toml': invalid TOML"));
    const res = await POST();
    expect(res.status).toBe(400);
    const body = await res.json() as { error: string };
    expect(body.error).toContain("malformed config");
  });

  it("returns 502 with String() error when a non-Error value is thrown", async () => {
    // exercises `err instanceof Error ? err.message : String(err)` — the String(err) arm
    mockReload.mockRejectedValueOnce("reload service unavailable");
    const res = await POST();
    expect(res.status).toBe(502);
    const body = await res.json() as { error: string };
    expect(body.error).toBe("reload service unavailable");
  });
});
