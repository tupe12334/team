import path from "path";
import {
  credentials,
  Client,
  ServiceError,
  loadPackageDefinition,
  ServiceClientConstructor,
} from "@grpc/grpc-js";
import { loadSync, Options } from "@grpc/proto-loader";

const PROTO_DIR =
  process.env.PROTO_DIR ?? path.resolve(process.cwd(), "..", "proto");

const LOAD_OPTIONS: Options = {
  keepCase: false,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true,
};

export class DaemonPortMissingError extends Error {
  constructor() {
    super("DAEMON_PORT must be set in .env");
  }
}

function makeClient(protoFile: string, serviceName: string): Client {
  const daemonPort = process.env.DAEMON_PORT;
  if (!daemonPort) throw new DaemonPortMissingError();
  const daemonHost = process.env.DAEMON_HOST ?? "localhost";
  const daemonAddr = `${daemonHost}:${daemonPort}`;
  const packageDef = loadSync(path.join(PROTO_DIR, protoFile), LOAD_OPTIONS);
  const pkg = loadPackageDefinition(packageDef) as Record<
    string,
    Record<string, ServiceClientConstructor>
  >;
  // eslint-disable-next-line security/detect-object-injection
  const ServiceCtor = pkg.team[serviceName];
  return new ServiceCtor(daemonAddr, credentials.createInsecure());
}

let _daemon: Client | null = null;
let _queue: Client | null = null;
let _worker: Client | null = null;

export const getDaemonClient = () =>
  (_daemon ??= makeClient("daemon.proto", "DaemonService"));
export const getQueueClient = () =>
  (_queue ??= makeClient("queue.proto", "QueueService"));
export const getWorkerClient = () =>
  (_worker ??= makeClient("worker.proto", "WorkerService"));

type GrpcMethod = (
  req: object,
  cb: (err: ServiceError | null, res: unknown) => void
) => void;

/** Promisified gRPC unary call. */
export function grpcCall<TRes>(
  client: Client,
  method: string,
  request: object
): Promise<TRes> {
  return new Promise((resolve, reject) => {
    // eslint-disable-next-line security/detect-object-injection
    const fn = (client as Record<string, GrpcMethod>)[method];
    fn(request, (err, response) => {
      if (err) reject(err);
      else resolve(response as TRes);
    });
  });
}
