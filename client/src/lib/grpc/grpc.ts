/* eslint-disable single-export/single-export */
import { credentials, ServiceError, status as GrpcStatus } from "@grpc/grpc-js";
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
  if (process.env.DAEMON_ADDR) return process.env.DAEMON_ADDR;
  // eslint-disable-next-line no-restricted-syntax
  const daemonPort = process.env.DAEMON_PORT;
  if (!daemonPort) throw new DaemonPortMissingError();
  // eslint-disable-next-line no-restricted-syntax
  return `${process.env.DAEMON_HOST ?? "[::1]"}:${daemonPort}`;
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

/**
 * Maps a gRPC ServiceError status code to the most appropriate HTTP status.
 * Falls back to 502 for transport errors or unknown status codes.
 */
export function grpcHttpStatus(err: unknown): number {
  if (typeof err === "object" && err !== null && "code" in err) {
    const code = (err as { code: number }).code;
    if (code === GrpcStatus.NOT_FOUND) return 404;
    if (code === GrpcStatus.FAILED_PRECONDITION) return 409;
    if (code === GrpcStatus.INVALID_ARGUMENT) return 400;
    if (code === GrpcStatus.UNAVAILABLE) return 503;
  }
  return 502;
}

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
