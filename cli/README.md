# cli

The `team` CLI — a gRPC client for interacting with the team daemon.

## Build

```bash
go build -o team .
```

Or from the repo root:

```bash
make build
```

## Usage

```bash
# Daemon management
team daemon-service get-info
team daemon-service get-config
team daemon-service update-config --workers-count 4
team daemon-service reload-config
team daemon-service shutdown

# Task queue
team queue-service enqueue --issue-ref github:owner/repo#42
team queue-service list-queue
team queue-service update-task --task-id <id> --agent gemini --priority 1
team queue-service remove-task --task-id <id>

# Worker status
team worker-service get-worker-status
```

The daemon address defaults to `[::1]:50051`. Override with the `--addr` flag or `DAEMON_ADDR` environment variable.

## Code generation

The CLI commands and MCP bindings are generated from the proto definitions via `protoc-gen-cobra` and `protoc-gen-go-mcp`. Regenerate with:

```bash
cd .. && buf generate
```
