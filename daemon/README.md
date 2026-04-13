# daemon

The `team` daemon — a gRPC server written in Rust that manages the task queue, worker pool, and Centy auto-polling.

## Build

```bash
cargo build --release
```

Requires `protoc` (Protocol Buffer compiler) for the build step.

## Run

```bash
DAEMON_PORT=50051 cargo run
```

Or with the release binary:

```bash
DAEMON_PORT=50051 ./target/release/daemon
```

Override the config file location:

```bash
CONFIG_PATH=~/.config/team/config.toml DAEMON_PORT=50051 ./target/release/daemon
```

## Services

Implements four gRPC services defined in `../proto/`:

| Service | Description |
|---|---|
| `AgentService` | List available gstack agents (filtered by `enabled_agents` config) |
| `DaemonService` | Daemon info, config management, reload, shutdown |
| `QueueService` | Enqueue tasks, list/update/remove from queue |
| `WorkerService` | Worker pool status |

## Background tasks

Two background loops start automatically on boot:

- **Worker pool** — picks the highest-priority QUEUED task, marks it RUNNING, calls `worktree open <issue-ref>` (with `TEAM_AGENT` set if the task has an agent), waits for exit, marks COMPLETED or FAILED.
- **Centy poller** — every 30 s, runs `centy list issues --status "in queue" --global --json` and auto-enqueues any issue not already in the queue. Issues already present (in any status) are skipped to prevent re-dispatch within the 7-day retention window.

## Queue persistence

The queue is stored as JSON alongside the config file (`queue.json`). On each save, tasks that are COMPLETED or FAILED and older than 7 days are pruned. Tasks that were RUNNING when the daemon last exited are reset to QUEUED on restart.

## Proto generation

Proto files are compiled at build time via `build.rs` using `tonic-build`. After editing `.proto` files, just `cargo build`.
