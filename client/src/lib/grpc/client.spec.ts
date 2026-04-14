import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("./grpc", () => ({
  getAgentClient: vi.fn(),
  getDaemonClient: vi.fn(),
  getQueueClient: vi.fn(),
  getWorkerClient: vi.fn(),
  grpcCall: vi.fn(),
}));

import { grpcCall } from "./grpc";
import {
  ApiError,
  EmptyResponseError,
  daemonGetInfo,
  daemonShutdown,
  daemonReloadConfig,
  daemonGetConfig,
  daemonUpdateConfig,
  daemonGetAvailableAgents,
  queueList,
  queueEnqueue,
  queueUpdateTask,
  queueRemoveTask,
  workerGetStatus,
} from "./client";

const mockGrpcCall = vi.mocked(grpcCall);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("daemonGetInfo", () => {
  it("returns DaemonInfo on success", async () => {
    const info = { version: "1.0.0", uptimeSeconds: 60, configPath: "/etc/daemon.toml", workersCount: 4 };
    mockGrpcCall.mockResolvedValueOnce({ ok: info });
    const result = await daemonGetInfo();
    expect(result).toEqual(info);
  });

  it("throws ApiError when response contains error field", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "daemon unavailable" });
    await expect(daemonGetInfo()).rejects.toThrow(ApiError);
  });

  it("throws EmptyResponseError when response has neither ok nor error", async () => {
    mockGrpcCall.mockResolvedValueOnce({});
    await expect(daemonGetInfo()).rejects.toThrow(EmptyResponseError);
  });
});

describe("daemonGetAvailableAgents", () => {
  it("returns agents array on success", async () => {
    const agents = [
      { name: "review", description: "Code review agent" },
      { name: "qa", description: "QA agent" },
    ];
    mockGrpcCall.mockResolvedValueOnce({ ok: { agents } });
    const result = await daemonGetAvailableAgents();
    expect(result).toEqual(agents);
  });

  it("returns empty array when ok.agents is empty", async () => {
    mockGrpcCall.mockResolvedValueOnce({ ok: { agents: [] } });
    const result = await daemonGetAvailableAgents();
    expect(result).toEqual([]);
  });
});

describe("queueList", () => {
  it("returns task list on success", async () => {
    const tasks = [{ id: "t1", status: 1 }];
    mockGrpcCall.mockResolvedValueOnce({ ok: { tasks } });
    const result = await queueList();
    expect(result).toEqual(tasks);
  });

  it("returns empty array when tasks is undefined", async () => {
    mockGrpcCall.mockResolvedValueOnce({ ok: {} });
    const result = await queueList();
    expect(result).toEqual([]);
  });

  it("throws ApiError on error response", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "queue error" });
    await expect(queueList()).rejects.toThrow(ApiError);
  });
});

describe("queueEnqueue", () => {
  it("returns task on success", async () => {
    const task = { id: "t2", status: 1, priority: 5 };
    mockGrpcCall.mockResolvedValueOnce({ task });
    const issueRef = { github: { organization: "acme", repository: "app", number: "7" } };
    const result = await queueEnqueue(issueRef);
    expect(result).toEqual(task);
  });

  it("throws ApiError on error response", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "issue_ref is required" });
    const issueRef = { github: { organization: "acme", repository: "app", number: "1" } };
    await expect(queueEnqueue(issueRef)).rejects.toThrow(ApiError);
  });
});

describe("queueUpdateTask", () => {
  it("returns updated task on success", async () => {
    const task = { id: "t1", status: 1, priority: 3, agent: "qa" };
    mockGrpcCall.mockResolvedValueOnce({ task });
    const result = await queueUpdateTask("t1", { agent: "qa", priority: 3 });
    expect(result).toEqual(task);
  });

  it("throws ApiError on error response", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "task not found" });
    await expect(queueUpdateTask("missing-id", { priority: 1 })).rejects.toThrow(ApiError);
  });
});

describe("queueRemoveTask", () => {
  it("resolves on success", async () => {
    mockGrpcCall.mockResolvedValueOnce(undefined);
    await expect(queueRemoveTask("t1")).resolves.toBeUndefined();
  });
});

describe("workerGetStatus", () => {
  it("returns worker status on success", async () => {
    const status = { total: 4, busy: 1, idle: 3, workers: [] };
    mockGrpcCall.mockResolvedValueOnce({ ok: status });
    const result = await workerGetStatus();
    expect(result).toEqual(status);
  });

  it("throws ApiError on error response", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "worker service unavailable" });
    await expect(workerGetStatus()).rejects.toThrow(ApiError);
  });
});

describe("daemonShutdown", () => {
  it("resolves on success", async () => {
    mockGrpcCall.mockResolvedValueOnce({ ok: {} });
    await expect(daemonShutdown()).resolves.toBeUndefined();
  });

  it("throws ApiError on error response", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "shutdown failed" });
    await expect(daemonShutdown()).rejects.toThrow(ApiError);
  });
});

describe("daemonReloadConfig", () => {
  it("resolves on success", async () => {
    mockGrpcCall.mockResolvedValueOnce({ ok: {} });
    await expect(daemonReloadConfig()).resolves.toBeUndefined();
  });

  it("throws ApiError on error response", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "reload failed" });
    await expect(daemonReloadConfig()).rejects.toThrow(ApiError);
  });
});

describe("daemonGetConfig", () => {
  it("returns config on success", async () => {
    const config = { workersCount: 4, logLevel: "info", enabledAgents: [] };
    mockGrpcCall.mockResolvedValueOnce({ ok: config });
    const result = await daemonGetConfig();
    expect(result).toEqual(config);
  });

  it("throws ApiError on error response", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "config unavailable" });
    await expect(daemonGetConfig()).rejects.toThrow(ApiError);
  });

  it("throws EmptyResponseError when response has neither ok nor error", async () => {
    mockGrpcCall.mockResolvedValueOnce({});
    await expect(daemonGetConfig()).rejects.toThrow(EmptyResponseError);
  });
});

describe("daemonUpdateConfig", () => {
  it("returns updated config on success", async () => {
    const config = { workersCount: 2, logLevel: "debug", enabledAgents: ["review", "qa"] };
    mockGrpcCall.mockResolvedValueOnce({ ok: config });
    const result = await daemonUpdateConfig(config);
    expect(result).toEqual(config);
  });

  it("throws ApiError on error response", async () => {
    mockGrpcCall.mockResolvedValueOnce({ error: "update failed" });
    await expect(daemonUpdateConfig({ workersCount: 1, logLevel: "info", enabledAgents: [] })).rejects.toThrow(ApiError);
  });
});
