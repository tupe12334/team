# team

> A system-wide daemon that manages a queue of tasks and assigns them to [worktree-io](https://github.com/worktree-io/runner) for execution.

`team` connects to your issue trackers (GitHub, Centy.io, Jira), lets you build a task queue, and automatically routes each task to a worker. Execution is fully delegated to `worktree-io` — `team` only manages the queue and worker assignment.

## Features

- **Issue tracker integration** — enqueue tasks from GitHub, Centy.io, or any URL (Jira URLs are detected but not yet executable by worktree-io)
- **Centy auto-polling** — the daemon watches Centy for issues with status "in queue" and enqueues them automatically every 30 s; once a task finishes it stays in the queue for 7 days to prevent re-dispatch
- **35 gstack agents** — each task can be assigned a named agent skill (review, qa, ship, plan-eng-review, …) passed as `TEAM_AGENT` to `worktree-io` hooks
- **Configurable worker pool** — set how many agents run in parallel; optional `enabled_agents` list restricts the dropdown to a subset
- **worktree-io delegation** — each task is executed via `worktree open <issue-ref>`; all process, worktree, and hook logic lives in `worktree-io`
- **gRPC API** — all queue and worker management is done via gRPC
- **Multiple interfaces** — CLI, web UI, TUI, and MCP server
- **Queue pruning** — completed and failed tasks older than 7 days are automatically removed on each queue save

## Requirements

- Rust 1.85+ (`cargo`)
- Go 1.26+
- Node.js 20+ with `pnpm`
- [`worktree-io`](https://github.com/worktree-io/runner) (`worktree` binary in PATH)

## Build

```bash
pnpm install   # installs proto code-gen tooling (ts-proto etc.)
make build     # generates proto bindings, then compiles all components
```

This produces:
- `bin/team` — CLI client
- `bin/mcp-server` — MCP server
- `daemon/target/debug/daemon` — daemon binary
- `tui/target/debug/tui` — TUI binary

## Configuration

Copy `.env.example` to `.env` and set:

```
DAEMON_PORT=50051       # gRPC port the daemon listens on
```

The daemon config file (`~/.config/team/config.toml` by default, override with `CONFIG_PATH` env var):

```toml
workers_count = 4       # number of parallel workers
log_level = "info"      # error | warn | info | debug | trace
enabled_agents = []     # empty = all agents enabled
```

Agent behavior (hooks, editor, TTL) is configured in `worktree-io` — see its [configuration docs](https://github.com/worktree-io/runner).

## Usage

Start the daemon and web UI together:

```bash
make run
```

Or start the daemon manually:

```bash
DAEMON_PORT=50051 ./daemon/target/debug/daemon
```

Interact via the CLI:

```bash
# List available agents
team agent-service get-available-agents

# Enqueue a task (GitHub)
team queue-service enqueue \
  --issue-ref-github \
  --issue-ref-github-organization <org> \
  --issue-ref-github-repository <repo> \
  --issue-ref-github-number <number>

# Enqueue a task (Centy)
team queue-service enqueue \
  --issue-ref-centy \
  --issue-ref-centy-organization <org> \
  --issue-ref-centy-repository <repo> \
  --issue-ref-centy-number <id>

# Enqueue a task (URL — resolved automatically; GitHub and Centy URLs are supported)
team queue-service enqueue --issue-ref-link --issue-ref-link-url https://github.com/org/repo/issues/42

# List the queue
team queue-service list-queue

# Remove a task
team queue-service remove-task --task-id <id>

# Check worker status
team worker-service get-worker-status

# Daemon management
team daemon-service get-info
team daemon-service get-config
team daemon-service update-config --workers-count 4
team daemon-service reload-config
team daemon-service shutdown
```

The daemon address defaults to `[::1]:DAEMON_PORT`. Override with `DAEMON_ADDR` or `--server-addr`.

## Interfaces

| Interface | How to run |
|-----------|-----------|
| **CLI** | `./bin/team <command>` |
| **Web UI** | `make run` → http://localhost:3000 |
| **TUI** | `make tui` |
| **MCP** | Configure `./bin/mcp-server` in your MCP client |

## How it works

```
Sources
  Centy auto-poll ──► (issues with status "in queue", every 30 s)
  Manual / CLI / Web / MCP ──────────────────────────────────────►  Task Queue
                                                                         │
                                                                         ▼
                                                                   Queue Manager
                                                               (discovery + assignment)
                                                                         │
                                                                         ▼
                                                               worktree open <ref>
                                                         (TEAM_AGENT env → gstack skill)
```

`team` is responsible only for queue state and worker concurrency. When a worker slot opens it calls `worktree open <issue-ref>`, optionally setting `TEAM_AGENT` so the `post:open` hook runs the right gstack skill (e.g. `claude --dangerously-skip-permissions /review`). All worktree creation, branch management, and agent invocation are handled by `worktree-io`.

## License

MIT
