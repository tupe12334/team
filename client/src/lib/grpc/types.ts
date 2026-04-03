export interface DaemonInfo {
  version: string;
  uptimeSeconds: string;
  configPath: string;
  workersCount: number;
}

export type IssueProvider = "GITHUB" | "JIRA" | "CENTY";

export interface RepoIssueRef {
  organization: string;
  repository: string;
  number: string;
}

export type IssueRef =
  | { provider: IssueProvider; ref: "repoIssue"; repoIssue: RepoIssueRef }
  | { provider: IssueProvider; ref: "id"; id: string };

export interface Task {
  id: string;
  issueRef: IssueRef;
  agent: string;
  status: "QUEUED" | "RUNNING" | "COMPLETED" | "FAILED";
  priority: number;
  createdAt: string | null;
  updatedAt: string | null;
}

export interface WorkerInfo {
  workerId: string;
  status: "IDLE" | "BUSY";
  currentTaskId: string;
  currentAgent: string;
  taskStartedAt: string | null;
}

export interface WorkerStatusData {
  total: number;
  busy: number;
  idle: number;
  workers: WorkerInfo[];
}

export interface DaemonConfig {
  workersCount: number;
  logLevel: string;
}

export type OneofResponse<T> =
  | { result: "ok"; ok: T }
  | { result: "error"; error: string }
  | { result: "task"; task: T };

export function unwrap<T>(response: OneofResponse<T>): T {
  if (response.result === "error") throw new Error(response.error);
  if (response.result === "task") return response.task;
  return response.ok;
}
