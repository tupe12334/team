import { NextResponse } from "next/server";
import { daemonGetAvailableAgents, ApiError } from "@/lib/grpc/client";
import { grpcHttpStatus } from "@/lib/grpc/grpc";

export async function GET() {
  try {
    const agents = await daemonGetAvailableAgents();
    return NextResponse.json(agents);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const s = err instanceof ApiError ? 400 : grpcHttpStatus(err);
    return NextResponse.json({ error: message }, { status: s });
  }
}
