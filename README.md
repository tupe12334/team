# team

> A system-wide daemon that manages a queue of tasks and assigns them to [worktree-io](https://github.com/worktree-io/runner) for execution.

`team` connects to your issue trackers (GitHub, Centy.io, Jira), lets you build a task queue, and automatically routes each task to a worker. Execution is fully delegated to `worktree-io` — `team` only manages the queue and worker assignment.

## Features

- **Issue tracker integration** — enqueue tasks from GitHub, Centy.io, Jira, or any URL
- **Configurable worker pool** — set how many agents run in parallel
- **worktree-io delegation** — each task is executed via `worktree open <issue-ref>`; all process, worktree, and hook logic lives in `worktree-io`
- **gRPC API** — all queue and worker management is done via gRPC
- **Multiple interfaces** — CLI, web UI, TUI, and MCP server

## Requirements

- Rust 1.85+ (`cargo`)
- Go 1.22+
- Node.js 20+ with `pnpm`
- [`worktree-io`](https://github.com/worktree-io/runner) (`worktree` binary in PATH)

## Build

```bash
make build
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

# Enqueue a task (Jira)
team queue-service enqueue --issue-ref-jira --issue-ref-jira-id PROJ-123

# Enqueue a task (URL — resolved automatically)
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
Sources (GitHub / Centy.io / CLI / Web / MCP)
        │
        ▼
   Task Queue  ◄──── user manages (add / remove / reprioritize)
        │
        ▼
  Queue Manager  ──── discovery + assignment
        │
        ▼
  worktree-io  ──── `worktree open <issue-ref>`
```

`team` is responsible only for queue state and worker concurrency. When a worker slot opens, it runs `worktree open <issue-ref>` and waits for the exit code. All worktree creation, branch management, hook execution, and agent invocation are handled by `worktree-io`.

## License

MIT
