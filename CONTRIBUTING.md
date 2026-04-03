# Contributing to team

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                          teamd                              │
│                                                             │
│  ┌──────────────┐    ┌──────────┐    ┌──────────────────┐  │
│  │Issue Providers│───▶│  Queue   │───▶│   AI Router      │  │
│  │  GitHub       │    │          │    │  (agent assign)  │  │
│  │  Centy.io     │    │          │    └────────┬─────────┘  │
│  └──────────────┘    └──────────┘             │             │
│                                               ▼             │
│                                      ┌─────────────────┐   │
│                                      │  Worker Pool    │   │
│                                      │  [w1][w2][w3]   │   │
│                                      └────────┬────────┘   │
└───────────────────────────────────────────────┼────────────┘
                                                │
                              ┌─────────────────┼──────────────┐
                              ▼                 ▼              ▼
                        claude-code CLI    gemini CLI      codex CLI
```

### Components

**Daemon core (`src/daemon`)**
The gRPC server and main lifecycle manager. Owns the worker pool and coordinates all subsystems. Entry point for all client interactions.

**Issue providers (`src/providers`)**
Adapters for external issue trackers. Each provider implements the `IssueProvider` trait, which exposes a uniform interface for listing and fetching issues regardless of source. Adding a new provider means implementing this trait.

**Queue (`src/queue`)**
An ordered, in-memory (persisted to disk) task queue. Supports enqueue, remove, reprioritize, and agent reassignment. Tasks carry metadata: source issue reference, assigned agent, status, and timestamps.

**AI Router (`src/router`)**
Runs at enqueue time. Given a task description and codebase context, selects the most appropriate agent. The user can override the assignment at any time before a worker picks it up.

**Worker pool (`src/workers`)**
Manages N concurrent worker slots (user-configured). When a slot opens, it picks the next task from the queue, spawns the assigned agent CLI as a subprocess, streams its output, and marks the task complete or failed on exit.

**Agent adapters (`src/agents`)**
Thin wrappers around each CLI binary (Claude Code, Gemini CLI, Codex). Each adapter implements the `Agent` trait, which handles how the task prompt is formatted and passed to the CLI, and how output is captured.

### Data flow

```
1. User enqueues task (gRPC call or issue provider sync)
2. AI Router assigns an agent based on task + codebase context
3. Task sits in queue; user can inspect or modify assignment
4. Worker slot opens → picks next task
5. Worker spawns agent CLI subprocess with task as prompt
6. Output is streamed and stored; task marked complete/failed
```

## Development setup

```bash
# Install Rust (1.78+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install protoc (Protocol Buffer compiler)
# macOS:
brew install protobuf
# Ubuntu:
apt install -y protobuf-compiler

# Build
cargo build

# Run tests
cargo test

# Run the daemon locally
cargo run --bin teamd -- --config team.toml
```

## Adding an issue provider

1. Create `src/providers/<name>.rs`
2. Implement the `IssueProvider` trait:

```rust
pub trait IssueProvider: Send + Sync {
    async fn list_issues(&self) -> Result<Vec<Issue>>;
    async fn get_issue(&self, id: &str) -> Result<Issue>;
}
```

3. Register the provider in `src/providers/mod.rs`
4. Add the configuration schema in `src/config.rs`

## Adding an agent

1. Create `src/agents/<name>.rs`
2. Implement the `Agent` trait:

```rust
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self, task: &Task) -> Result<AgentOutput>;
}
```

3. Register the agent in `src/agents/mod.rs`
4. Add the configuration schema in `src/config.rs`

## gRPC / proto changes

Proto definitions live in `proto/team.proto`. After editing:

```bash
# Regenerate Rust bindings (handled automatically by build.rs)
cargo build
```

Keep the proto file as the source of truth. Do not hand-edit the generated files in `src/proto/`.

## Code style & conventions

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Each module should have a single clear responsibility
- Async runtime: Tokio
- Error handling: `anyhow` for application errors, `thiserror` for library errors
- Tests live alongside the code in `#[cfg(test)]` modules; integration tests in `tests/`
