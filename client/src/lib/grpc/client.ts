import { getDaemonClient, getQueueClient, getWorkerClient, grpcCall } from "./grpc";
import {
  DaemonInfo,
  DaemonConfig,
  IssueRef,
  OneofResponse,
  Task,
  WorkerStatusData,
  unwrap,
} from "./types";

export type {
  DaemonInfo,
  DaemonConfig,
  IssueRef,
  IssueProvider,
  RepoIssueRef,
  Task,
  WorkerInfo,
  WorkerStatusData,
} from "./types";

export async function daemonGetInfo(): Promise<DaemonInfo> {
  const res = await grpcCall<OneofResponse<DaemonInfo>>(
    getDaemonClient(),
    "getInfo",
    {}
  );
  return unwrap(res);
}

export async function daemonShutdown(): Promise<void> {
  await grpcCall(getDaemonClient(), "shutdown", {});
}

export async function daemonReloadConfig(): Promise<void> {
  await grpcCall(getDaemonClient(), "reloadConfig", {});
}

export async function queueList(): Promise<Task[]> {
  const res = await grpcCall<OneofResponse<{ tasks: Task[] }>>(
    getQueueClient(),
    "listQueue",
    {}
  );
  return unwrap(res).tasks;
}

export async function queueEnqueue(
  issueRef: IssueRef,
  agent?: string
): Promise<Task> {
  const res = await grpcCall<OneofResponse<Task>>(
    getQueueClient(),
    "enqueue",
    { issueRef, agent }
  );
  return unwrap(res);
}

export async function queueUpdateTask(
  taskId: string,
  update: { agent?: string; priority?: number }
): Promise<Task> {
  const res = await grpcCall<OneofResponse<Task>>(
    getQueueClient(),
    "updateTask",
    { taskId, ...update }
  );
  return unwrap(res);
}

export async function queueRemoveTask(taskId: string): Promise<void> {
  await grpcCall(getQueueClient(), "removeTask", { taskId });
}

export async function daemonGetConfig(): Promise<DaemonConfig> {
  const res = await grpcCall<OneofResponse<DaemonConfig>>(
    getDaemonClient(),
    "getConfig",
    {}
  );
  return unwrap(res);
}

export async function daemonUpdateConfig(
  config: DaemonConfig
): Promise<DaemonConfig> {
  const res = await grpcCall<OneofResponse<DaemonConfig>>(
    getDaemonClient(),
    "updateConfig",
    { config }
  );
  return unwrap(res);
}

export async function workerGetStatus(): Promise<WorkerStatusData> {
  const res = await grpcCall<OneofResponse<WorkerStatusData>>(
    getWorkerClient(),
    "getWorkerStatus",
    {}
  );
  return unwrap(res);
}
