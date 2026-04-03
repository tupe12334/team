import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";
import path from "path";

const PROTO_DIR =
  process.env.PROTO_DIR ?? path.resolve(process.cwd(), "..", "proto");
if (!process.env.DAEMON_ADDR) throw new Error("DAEMON_ADDR must be set");
const DAEMON_ADDR: string = process.env.DAEMON_ADDR;

const LOAD_OPTIONS: protoLoader.Options = {
  keepCase: false,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true,
};

function makeClient(protoFile: string, serviceName: string): grpc.Client {
  const packageDef = protoLoader.loadSync(
    path.join(PROTO_DIR, protoFile),
    LOAD_OPTIONS
  );
  const pkg = grpc.loadPackageDefinition(packageDef) as Record<
    string,
    Record<string, grpc.ServiceClientConstructor>
  >;
  const ServiceCtor = pkg["team"][serviceName];
  return new ServiceCtor(DAEMON_ADDR, grpc.credentials.createInsecure());
}

// Singletons — reuse the same channel across requests
let _daemon: grpc.Client | null = null;
let _queue: grpc.Client | null = null;
let _worker: grpc.Client | null = null;

export const getDaemonClient = () =>
  (_daemon ??= makeClient("daemon.proto", "DaemonService"));
export const getQueueClient = () =>
  (_queue ??= makeClient("queue.proto", "QueueService"));
export const getWorkerClient = () =>
  (_worker ??= makeClient("worker.proto", "WorkerService"));

/** Promisified gRPC unary call. */
export function grpcCall<TReq extends object, TRes>(
  client: grpc.Client,
  method: string,
  request: TReq
): Promise<TRes> {
  return new Promise((resolve, reject) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (client as any)[method](
      request,
      (err: grpc.ServiceError | null, response: TRes) => {
        if (err) reject(err);
        else resolve(response);
      }
    );
  });
}

// ── Response types (mirrors proto oneofs) ────────────────────────────────────

export interface DaemonInfo {
  version: string;
  uptimeSeconds: string;
  configPath: string;
  workersCount: number;
}

export interface Task {
  id: string;
  issueRef: string;
  agent: string;
  status: "QUEUED" | "RUNNING" | "COMPLETED" | "FAILED";
  priority: number;
  createdAt: string | null;
  updatedAt: string | null;
}

export interface WorkerInfo {
  workerId: string;
  status: "IDLE" | "BUSY";
  currentTaskId: string;
  currentAgent: string;
  taskStartedAt: string | null;
}

export interface WorkerStatusData {
  total: number;
  busy: number;
  idle: number;
  workers: WorkerInfo[];
}

// ── Typed helpers that unwrap the oneof result ────────────────────────────────

type OneofResponse<T> =
  | { result: "ok"; ok: T }
  | { result: "error"; error: string }
  | { result: "task"; task: T };

function unwrap<T>(response: OneofResponse<T>): T {
  if (response.result === "error") throw new Error(response.error as string);
  if (response.result === "task") return response.task;
  return response.ok;
}

export async function daemonGetInfo(): Promise<DaemonInfo> {
  const res = await grpcCall<object, OneofResponse<DaemonInfo>>(
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
  const res = await grpcCall<object, OneofResponse<{ tasks: Task[] }>>(
    getQueueClient(),
    "listQueue",
    {}
  );
  return unwrap(res).tasks;
}

export async function queueEnqueue(
  issueRef: string,
  agent?: string
): Promise<Task> {
  const res = await grpcCall<
    { issueRef: string; agent?: string },
    OneofResponse<Task>
  >(getQueueClient(), "enqueue", { issueRef, agent });
  return unwrap(res);
}

export async function queueUpdateTask(
  taskId: string,
  update: { agent?: string; priority?: number }
): Promise<Task> {
  const res = await grpcCall<object, OneofResponse<Task>>(
    getQueueClient(),
    "updateTask",
    { taskId, ...update }
  );
  return unwrap(res);
}

export async function queueRemoveTask(taskId: string): Promise<void> {
  await grpcCall(getQueueClient(), "removeTask", { taskId });
}

export interface DaemonConfig {
  workersCount: number;
  logLevel: string;
}

export async function daemonGetConfig(): Promise<DaemonConfig> {
  const res = await grpcCall<object, OneofResponse<DaemonConfig>>(
    getDaemonClient(),
    "getConfig",
    {}
  );
  return unwrap(res);
}

export async function daemonUpdateConfig(config: DaemonConfig): Promise<DaemonConfig> {
  const res = await grpcCall<{ config: DaemonConfig }, OneofResponse<DaemonConfig>>(
    getDaemonClient(),
    "updateConfig",
    { config }
  );
  return unwrap(res);
}

export async function workerGetStatus(): Promise<WorkerStatusData> {
  const res = await grpcCall<object, OneofResponse<WorkerStatusData>>(
    getWorkerClient(),
    "getWorkerStatus",
    {}
  );
  return unwrap(res);
}
