/* eslint-disable single-export/single-export */
import { NextResponse } from "next/server";
import { daemonGetConfig, daemonUpdateConfig, DaemonConfig } from "@/lib/grpc/client";

export async function GET() {
  try {
    const config = await daemonGetConfig();
    return NextResponse.json(config);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}

export async function PATCH(req: Request) {
  try {
    // eslint-disable-next-line no-restricted-syntax
    const body = (await req.json()) as DaemonConfig;
    const config = await daemonUpdateConfig(body);
    return NextResponse.json(config);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
