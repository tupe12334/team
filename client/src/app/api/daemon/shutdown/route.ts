import { NextResponse } from "next/server";
import { daemonShutdown } from "@/lib/grpc/client";

export async function POST() {
  try {
    await daemonShutdown();
    return NextResponse.json({ ok: true });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
