# proto

Protocol Buffer definitions for the team daemon gRPC API.

## Files

| File | Service | Description |
|---|---|---|
| `daemon.proto` | `DaemonService` | Daemon info, config management, shutdown |
| `queue.proto` | `QueueService` | Task queue — enqueue, list, update, remove |
| `worker.proto` | `WorkerService` | Worker pool status |

## Generate

From the repo root (requires `buf`):

```bash
buf generate
```

This generates:
- **Go** bindings into `cli/gen/` (used by both `cli` and `mcp`)
- Rust code is generated at build time by `daemon/build.rs` via `tonic-build`

## buf.yaml

Managed with [Buf](https://buf.build). See `../buf.gen.yaml` for plugin configuration.
