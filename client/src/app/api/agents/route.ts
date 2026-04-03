import { NextResponse } from "next/server";
import { daemonGetAvailableAgents } from "@/lib/grpc/client";

export async function GET() {
  try {
    const agents = await daemonGetAvailableAgents();
    return NextResponse.json(agents);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
