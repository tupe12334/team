# mcp

An MCP (Model Context Protocol) server that exposes the team daemon's gRPC API as MCP tools, so AI assistants can manage the task queue directly.

## Build

```bash
go build -o mcp-server .
```

Or from the repo root:

```bash
make build
```

## Run

```bash
./mcp-server
```

The server communicates over stdio (MCP standard). It connects to the daemon at `[::1]:50051` by default — override with the `DAEMON_ADDR` environment variable.

## MCP tools exposed

All four daemon gRPC services are forwarded as MCP tools:

- **AgentService** — `get-available-agents`
- **DaemonService** — `get-info`, `get-config`, `update-config`, `reload-config`, `shutdown`
- **QueueService** — `enqueue`, `list-queue`, `update-task`, `remove-task`
- **WorkerService** — `get-worker-status`

## Configure in Claude Code

Add to your `claude_desktop_config.json` (or MCP settings):

```json
{
  "mcpServers": {
    "team": {
      "command": "/path/to/mcp-server",
      "env": {
        "DAEMON_ADDR": "[::1]:50051"
      }
    }
  }
}
```
