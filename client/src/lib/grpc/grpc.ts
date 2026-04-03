import { credentials, ServiceError } from "@grpc/grpc-js";
import { DaemonServiceClient } from "@/gen/daemon";
import { QueueServiceClient } from "@/gen/queue";
import { WorkerServiceClient } from "@/gen/worker";

export class DaemonPortMissingError extends Error {
  constructor() {
    super("DAEMON_PORT must be set in .env");
  }
}

function getDaemonAddr(): string {
  const daemonPort = process.env.DAEMON_PORT;
  if (!daemonPort) throw new DaemonPortMissingError();
  return `${process.env.DAEMON_HOST ?? "localhost"}:${daemonPort}`;
}

let _daemon: DaemonServiceClient | null = null;
let _queue: QueueServiceClient | null = null;
let _worker: WorkerServiceClient | null = null;

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
