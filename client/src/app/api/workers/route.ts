import { NextResponse } from "next/server";
import { workerGetStatus } from "@/lib/grpc/client";

export async function GET() {
  try {
    const status = await workerGetStatus();
    return NextResponse.json(status);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
