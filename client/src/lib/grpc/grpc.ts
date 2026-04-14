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
 * gRPC codes: NOT_FOUND=5, FAILED_PRECONDITION=9, INVALID_ARGUMENT=3, UNAVAILABLE=14
 */
export function grpcHttpStatus(err: unknown): number {
  if (typeof err === "object" && err !== null && "code" in err) {
    // eslint-disable-next-line no-restricted-syntax
    const code = (err as Record<string, unknown>).code;
    if (typeof code === "number") {
      if (code === 5) return 404; // NOT_FOUND
      if (code === 9) return 409; // FAILED_PRECONDITION
      if (code === 3) return 400; // INVALID_ARGUMENT
      if (code === 14) return 503; // UNAVAILABLE
    }
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
