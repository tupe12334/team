/* eslint-disable single-export/single-export */
import { credentials, ServiceError } from "@grpc/grpc-js";
import { AgentServiceClient } from "@/gen/agents";
import { DaemonServiceClient } from "@/gen/daemon";
import { QueueServiceClient } from "@/gen/queue";
import { WorkerServiceClient } from "@/gen/worker";

export class DaemonPortMissingError extends Error {
  constructor() {
    super("DAEMON_PORT must be set in .env");
  }
}

function getDaemonAddr(): string {
  // eslint-disable-next-line no-restricted-syntax
  const daemonPort = process.env.DAEMON_PORT;
  if (!daemonPort) throw new DaemonPortMissingError();
  // eslint-disable-next-line no-restricted-syntax, default/no-localhost
  return `${process.env.DAEMON_HOST ?? "localhost"}:${daemonPort}`;
}

let _agent: AgentServiceClient | null = null;
let _daemon: DaemonServiceClient | null = null;
let _queue: QueueServiceClient | null = null;
let _worker: WorkerServiceClient | null = null;

export const getAgentClient = () =>
  (_agent ??= new AgentServiceClient(getDaemonAddr(), credentials.createInsecure()));
export const getDaemonClient = () =>
  (_daemon ??= new DaemonServiceClient(getDaemonAddr(), credentials.createInsecure()));
export const getQueueClient = () =>
  (_queue ??= new QueueServiceClient(getDaemonAddr(), credentials.createInsecure()));
export const getWorkerClient = () =>
  (_worker ??= new WorkerServiceClient(getDaemonAddr(), credentials.createInsecure()));

/** Promisified gRPC unary call. */
export function grpcCall<TRes>(
  fn: (cb: (err: ServiceError | null, res: TRes) => void) => void
): Promise<TRes> {
  return new Promise((resolve, reject) => {
    fn((err, response) => {
      if (err) reject(err);
      else resolve(response);
    });
  });
}
