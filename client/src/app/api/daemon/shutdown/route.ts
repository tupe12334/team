import { NextResponse } from "next/server";
import { daemonShutdown, ApiError } from "@/lib/grpc/client";
import { grpcHttpStatus } from "@/lib/grpc/grpc";

export async function POST() {
  try {
    await daemonShutdown();
    return NextResponse.json({ ok: true });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const s = err instanceof ApiError ? 400 : grpcHttpStatus(err);
    return NextResponse.json({ error: message }, { status: s });
  }
}
