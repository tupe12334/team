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
# Agent list
team agent-service get-available-agents

# Daemon management
team daemon-service get-info
team daemon-service get-config
team daemon-service update-config --workers-count 4
team daemon-service reload-config
team daemon-service shutdown

# Task queue (GitHub)
team queue-service enqueue --issue-ref-github --issue-ref-github-organization <org> --issue-ref-github-repository <repo> --issue-ref-github-number <num>
# Task queue (Centy)
team queue-service enqueue --issue-ref-centy --issue-ref-centy-organization <org> --issue-ref-centy-repository <repo> --issue-ref-centy-number <id>
# Task queue (Jira)
team queue-service enqueue --issue-ref-jira --issue-ref-jira-id <PROJ-123>
# Task queue (Link URL)
team queue-service enqueue --issue-ref-link --issue-ref-link-url <url>
team queue-service list-queue
team queue-service update-task --task-id <id> --agent gemini --priority 1
team queue-service remove-task --task-id <id>

# Worker status
team worker-service get-worker-status
```

The daemon address defaults to `[::1]:50051`. Override with the `--server-addr` flag or `DAEMON_ADDR` environment variable.

## Code generation

The CLI commands and MCP bindings are generated from the proto definitions via `protoc-gen-cobra` and `protoc-gen-go-mcp`. Regenerate with:

```bash
cd .. && buf generate
```
