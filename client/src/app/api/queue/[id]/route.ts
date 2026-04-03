/* eslint-disable single-export/single-export */
import { NextRequest, NextResponse } from "next/server";
import { queueUpdateTask, queueRemoveTask } from "@/lib/grpc/client";

export async function PATCH(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> }
) {
  try {
    const { id } = await params;
    // eslint-disable-next-line no-restricted-syntax
    const { agent, priority } = (await req.json()) as { agent?: string; priority?: number };
    const task = await queueUpdateTask(id, { agent, priority });
    return NextResponse.json(task);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}

export async function DELETE(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> }
) {
  try {
    const { id } = await params;
    await queueRemoveTask(id);
    return new NextResponse(null, { status: 204 });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
