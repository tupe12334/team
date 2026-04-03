# Proto Design

**Date:** 2026-04-03

## Overview

Three `.proto` files defining the gRPC surface for the `team` daemon. All three services are registered on the same daemon process, which listens on a Unix socket (local CLI) and a TCP port (web client). No auth layer in the proto.

---

## `proto/queue.proto` — `QueueService`

### RPCs

| RPC | Request | Response |
|-----|---------|----------|
| `Enqueue` | `EnqueueRequest` | `Task` |
| `ListQueue` | `ListQueueRequest` | `ListQueueResponse` |
| `UpdateTask` | `UpdateTaskRequest` | `Task` |
| `RemoveTask` | `RemoveTaskRequest` | `Empty` |

### Messages

**`Task`**
- `id` — string
- `issue_ref` — string (e.g. `github:owner/repo#42`, `centy:project/slug`)
- `agent` — string (name of assigned agent)
- `status` — enum: `QUEUED` / `RUNNING` / `COMPLETED` / `FAILED`
- `priority` — int32
- `created_at` — google.protobuf.Timestamp
- `updated_at` — google.protobuf.Timestamp

**`EnqueueRequest`**
- `issue_ref` — string
- `agent` — string (optional override; daemon routes automatically if omitted)

**`ListQueueRequest`**
- (empty for now; reserved for future filtering)

**`ListQueueResponse`**
- `tasks` — repeated `Task`

**`UpdateTaskRequest`**
- `task_id` — string
- `agent` — string (optional)
- `priority` — int32 (optional)

**`RemoveTaskRequest`**
- `task_id` — string

---

## `proto/worker.proto` — `WorkerService`

### RPCs

| RPC | Request | Response |
|-----|---------|----------|
| `GetWorkerStatus` | `Empty` | `WorkerStatusResponse` |

### Messages

**`WorkerStatusResponse`**
- `total` — int32 (configured max workers)
- `busy` — int32
- `idle` — int32
- `workers` — repeated `WorkerInfo`

**`WorkerInfo`**
- `worker_id` — string
- `status` — enum: `IDLE` / `BUSY`
- `current_task_id` — string (empty if idle)
- `current_agent` — string (empty if idle)
- `task_started_at` — google.protobuf.Timestamp (unset if idle)

---

## `proto/daemon.proto` — `DaemonService`

### RPCs

| RPC | Request | Response |
|-----|---------|----------|
| `GetInfo` | `Empty` | `DaemonInfo` |
| `Shutdown` | `Empty` | `Empty` |
| `ReloadConfig` | `Empty` | `ReloadConfigResponse` |

### Messages

**`DaemonInfo`**
- `version` — string
- `uptime_seconds` — int64
- `config_path` — string
- `workers_count` — int32 (configured max)

**`ReloadConfigResponse`**
- `success` — bool
- `error` — string

---

## Out of scope (tracked separately)

- Task output / log streaming — see project issue #1
