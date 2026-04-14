import { NextResponse } from "next/server";
import { workerGetStatus, ApiError } from "@/lib/grpc/client";
import { grpcHttpStatus } from "@/lib/grpc/grpc";
import { WorkerInfo, workerStatusToJSON } from "@/gen/worker";

function serializeWorker(w: WorkerInfo) {
  return {
    workerId: w.workerId,
    status: workerStatusToJSON(w.status),
    currentTaskId: w.currentTaskId,
    currentAgent: w.currentAgent,
    taskStartedAt: w.taskStartedAt instanceof Date ? w.taskStartedAt.toISOString() : null,
  };
}

export async function GET() {
  try {
    const status = await workerGetStatus();
    return NextResponse.json({
      total: status.total,
      busy: status.busy,
      idle: status.idle,
      workers: status.workers.map(serializeWorker),
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const s = err instanceof ApiError ? 400 : grpcHttpStatus(err);
    return NextResponse.json({ error: message }, { status: s });
  }
}
