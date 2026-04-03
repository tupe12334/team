# team

> A system-wide daemon that manages a queue of tasks dispatched to AI agent workers.

`team` connects to your issue providers (GitHub, Centy.io), lets you build a task queue, and automatically routes each task to the right AI agent — Claude Code, Gemini CLI, Codex, or any other CLI-based agent you configure. You control how many agents run in parallel; `team` handles the rest.

## Features

- **Issue provider integration** — pull tasks from GitHub and Centy.io via a unified interface
- **Configurable worker pool** — set how many agents run in parallel
- **AI-powered routing** — tasks are assigned to an agent at enqueue time based on scope and codebase context; you can override any assignment
- **Platform-agnostic agents** — works with Claude Code, Gemini CLI, Codex, or any CLI-based agent
- **gRPC API** — all queue and worker management is done via gRPC

## Installation

```bash
cargo install --path .
```

Requires Rust 1.78+ and `protoc` (Protocol Buffer compiler).

## Configuration

Create a `team.toml` configuration file:

```toml
[daemon]
workers = 4  # number of parallel agent workers

[[agents]]
name = "claude"
command = "claude"  # path or name of the CLI binary
args = []

[[agents]]
name = "gemini"
command = "gemini"
args = []

[[providers]]
type = "github"
token = "ghp_..."
repos = ["owner/repo"]

[[providers]]
type = "centy"
token = "..."
```

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

# Change agent assignment for a queued task
team queue update <task-id> --agent gemini

# Remove a task from the queue
team queue remove <task-id>

# Check worker status
team workers status
```

## How it works

```
Issue Providers (GitHub / Centy.io)
        │
        ▼
   Task Queue  ◄──── user manages (add / remove / reprioritize)
        │
        ▼
   AI Router  ──── assigns agent at enqueue time (overridable)
        │
        ▼
  Worker Pool  ──── N parallel workers (user-configured)
        │
        ▼
Agent CLI Process (Claude Code / Gemini CLI / Codex / ...)
```

When a task enters the queue, the AI router analyzes its description and the associated codebase to select the best-suited agent. When a worker slot opens, the daemon spawns the assigned agent CLI as a subprocess and monitors it to completion.

## License

MIT
