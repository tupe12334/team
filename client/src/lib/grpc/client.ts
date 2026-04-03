/* eslint-disable single-export/single-export */
import { getDaemonClient, getQueueClient, getWorkerClient, grpcCall } from "./grpc";
import type {
  DaemonInfo,
  DaemonConfig,
  GetInfoResponse,
  GetConfigResponse,
  UpdateConfigResponse,
} from "@/gen/daemon";
import type {
  IssueRef,
  IssueRefInput,
  Task,
  EnqueueResponse,
  ListQueueResponse,
  UpdateTaskResponse,
} from "@/gen/queue";
import type { WorkerStatusData, WorkerStatusResponse } from "@/gen/worker";

export type { DaemonInfo, DaemonConfig } from "@/gen/daemon";
export type { IssueRef, IssueRefInput, Task } from "@/gen/queue";
export type { WorkerInfo, WorkerStatusData } from "@/gen/worker";

export class ApiError extends Error {}
export class EmptyResponseError extends Error {
  constructor() { super("Empty response"); }
}

function unwrap<T>(response: { ok?: T; task?: T; error?: string }): T {
  if (response.error !== undefined) throw new ApiError(response.error);
  // eslint-disable-next-line no-restricted-syntax
  const value = response.ok ?? response.task;
  if (value === undefined) throw new EmptyResponseError();
  return value;
}

export async function daemonGetInfo(): Promise<DaemonInfo> {
  const c = getDaemonClient();
  const res = await grpcCall<GetInfoResponse>((cb) => c.getInfo({}, cb));
  return unwrap(res);
}

export async function daemonShutdown(): Promise<void> {
  const c = getDaemonClient();
  await grpcCall((cb) => c.shutdown({}, cb));
}

export async function daemonReloadConfig(): Promise<void> {
  const c = getDaemonClient();
  await grpcCall((cb) => c.reloadConfig({}, cb));
}

export async function queueList(): Promise<Task[]> {
  const c = getQueueClient();
  const res = await grpcCall<ListQueueResponse>((cb) => c.listQueue({}, cb));
  if (res.error !== undefined) throw new ApiError(res.error);
  // eslint-disable-next-line no-restricted-syntax
  return res.ok?.tasks ?? [];
}

export async function queueEnqueue(issueRef: IssueRefInput, agent?: string, priority?: number): Promise<Task> {
  const c = getQueueClient();
  const res = await grpcCall<EnqueueResponse>((cb) => c.enqueue({ issueRef, agent, priority }, cb));
  return unwrap(res);
}

export async function queueUpdateTask(
  taskId: string,
  update: { agent?: string; priority?: number }
): Promise<Task> {
  const c = getQueueClient();
  const res = await grpcCall<UpdateTaskResponse>((cb) => c.updateTask({ taskId, ...update }, cb));
  return unwrap(res);
}

export async function queueRemoveTask(taskId: string): Promise<void> {
  const c = getQueueClient();
  await grpcCall((cb) => c.removeTask({ taskId }, cb));
}

export async function daemonGetConfig(): Promise<DaemonConfig> {
  const c = getDaemonClient();
  const res = await grpcCall<GetConfigResponse>((cb) => c.getConfig({}, cb));
  return unwrap(res);
}

export async function daemonUpdateConfig(config: DaemonConfig): Promise<DaemonConfig> {
  const c = getDaemonClient();
  const res = await grpcCall<UpdateConfigResponse>((cb) => c.updateConfig({ config }, cb));
  return unwrap(res);
}

export async function workerGetStatus(): Promise<WorkerStatusData> {
  const c = getWorkerClient();
  const res = await grpcCall<WorkerStatusResponse>((cb) => c.getWorkerStatus({}, cb));
  return unwrap(res);
}
