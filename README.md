# team

> A system-wide daemon that manages a queue of tasks and assigns them to [worktree-io](https://github.com/worktree-io/runner) for execution.

`team` connects to your issue providers (GitHub, Centy.io), lets you build a task queue, and automatically routes each task to a worker. Execution is fully delegated to `worktree-io` — `team` only manages the queue and worker assignment.

## Features

- **Issue provider integration** — pull tasks from GitHub and Centy.io via a unified interface
- **Configurable worker pool** — set how many agents run in parallel
- **worktree-io delegation** — each task is executed via `worktree <issue-ref> --headless`; all process, worktree, and hook logic lives in `worktree-io`
- **gRPC API** — all queue and worker management is done via gRPC

## Installation

```bash
cargo install --path .
```

Requires Rust 1.78+, `protoc` (Protocol Buffer compiler), and [`worktree-io`](https://github.com/worktree-io/runner) (`worktree` binary in PATH).

## Configuration

Create a `team.toml` configuration file:

```toml
[daemon]
workers = 4  # number of parallel workers

[[providers]]
type = "github"
token = "ghp_..."
repos = ["owner/repo"]

[[providers]]
type = "centy"
token = "..."
```

Agent behavior (hooks, editor, TTL) is configured in `worktree-io` — see its [configuration docs](https://github.com/worktree-io/runner).

## Usage

Start the daemon:

```bash
teamd --config team.toml
```

Interact via the gRPC client:

```bash
# Enqueue a task from an issue
team enqueue --issue github:owner/repo#42

# List the queue
team queue list

# Remove a task from the queue
team queue remove <task-id>

# Check worker status
team workers status
```

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
  worktree-io  ──── `worktree <issue-ref> --headless`
```

`team` is responsible only for queue state and worker concurrency. When a worker slot opens, it runs `worktree <issue-ref> --headless` and waits for the exit code. All worktree creation, branch management, hook execution, and agent invocation are handled by `worktree-io`.

## License

MIT
