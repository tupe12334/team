import { NextRequest, NextResponse } from "next/server";
import { queueList, queueEnqueue, IssueRef } from "@/lib/grpc/client";

export async function GET() {
  try {
    const tasks = await queueList();
    return NextResponse.json(tasks);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}

export async function POST(req: NextRequest) {
  try {
    const body = await req.json();
    const { issueRef, agent } = body as { issueRef: IssueRef; agent?: string };
    if (!issueRef) {
      return NextResponse.json({ error: "issueRef is required" }, { status: 400 });
    }
    const task = await queueEnqueue(issueRef, agent);
    return NextResponse.json(task, { status: 201 });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
