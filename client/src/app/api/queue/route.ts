/* eslint-disable single-export/single-export */
import { NextRequest, NextResponse } from "next/server";
import { queueList, queueEnqueue, ApiError } from "@/lib/grpc/client";
import { grpcHttpStatus } from "@/lib/grpc/grpc";
import type { IssueRefInput } from "@/lib/grpc/client";

export async function GET() {
  try {
    const tasks = await queueList();
    return NextResponse.json(tasks);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: grpcHttpStatus(err) });
  }
}

export async function POST(req: NextRequest) {
  try {
    // eslint-disable-next-line no-restricted-syntax
    const { issueRef, agent, priority } = (await req.json()) as { issueRef?: IssueRefInput; agent?: string; priority?: number };
    if (!issueRef) {
      return NextResponse.json({ error: "issueRef is required" }, { status: 400 });
    }
    const task = await queueEnqueue(issueRef, agent, priority);
    return NextResponse.json(task, { status: 201 });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const s = err instanceof ApiError ? 400 : grpcHttpStatus(err);
    return NextResponse.json({ error: message }, { status: s });
  }
}
