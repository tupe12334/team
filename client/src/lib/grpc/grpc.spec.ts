import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import type { ServiceError } from "@grpc/grpc-js";

describe("getDaemonAddr", () => {
  const OLD_ENV = { ...process.env };

  beforeEach(() => {
    vi.resetModules();
    process.env = { ...OLD_ENV };
    delete process.env.DAEMON_ADDR;
    delete process.env.DAEMON_PORT;
    delete process.env.DAEMON_HOST;
  });

  afterEach(() => {
    process.env = OLD_ENV;
  });

  it("returns DAEMON_ADDR directly when set", async () => {
    process.env.DAEMON_ADDR = "192.168.1.1:9999";
    const { getAgentClient } = await import("./grpc");
    // Calling getAgentClient exercises getDaemonAddr — we just verify it doesn't throw
    expect(() => getAgentClient()).not.toThrow();
  });

  it("throws DaemonPortMissingError when neither DAEMON_ADDR nor DAEMON_PORT is set", async () => {
    const { getDaemonClient, DaemonPortMissingError } = await import("./grpc");
    // Reset cached client so getDaemonAddr is called again
    expect(() => getDaemonClient()).toThrow(DaemonPortMissingError);
  });

  it("builds address from DAEMON_PORT", async () => {
    process.env.DAEMON_PORT = "12345";
    const { getQueueClient } = await import("./grpc");
    expect(() => getQueueClient()).not.toThrow();
  });
});

describe("grpcCall", () => {
  it("resolves when the callback receives a response", async () => {
    vi.resetModules();
    process.env.DAEMON_ADDR = "localhost:50051";
    const { grpcCall } = await import("./grpc");
    const result = await grpcCall<{ value: number }>((cb) => {
      cb(null, { value: 42 });
    });
    expect(result).toEqual({ value: 42 });
  });

  it("rejects when the callback receives an error", async () => {
    vi.resetModules();
    process.env.DAEMON_ADDR = "localhost:50051";
    const { grpcCall } = await import("./grpc");
    const fakeErr = new Error("rpc failed") as ServiceError;
    await expect(
      grpcCall((cb) => { cb(fakeErr, null as never); })
    ).rejects.toThrow("rpc failed");
  });
});
