import { NextResponse } from "next/server";
import { daemonGetInfo } from "@/lib/grpc/client";

export async function GET() {
  try {
    const info = await daemonGetInfo();
    return NextResponse.json(info);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
