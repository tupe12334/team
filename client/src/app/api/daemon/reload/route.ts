import { NextResponse } from "next/server";
import { daemonReloadConfig } from "@/lib/grpc/client";

export async function POST() {
  try {
    await daemonReloadConfig();
    return NextResponse.json({ ok: true });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
