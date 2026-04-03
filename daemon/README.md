# daemon

The `teamd` daemon — a gRPC server written in Rust that manages the task queue and worker pool.

## Build

```bash
cargo build --release
```

Requires `protoc` (Protocol Buffer compiler) for the build step.

## Run

```bash
cargo run -- --config team.toml
```

Or with the release binary:

```bash
./target/release/daemon --config team.toml
```

The daemon listens on `[::1]:50051` by default. Set `DAEMON_PORT` to override the port, or configure it in a `.env` file.

## Services

Implements three gRPC services defined in `../proto/`:

| Service | Description |
|---|---|
| `DaemonService` | Daemon info, config, shutdown |
| `QueueService` | Enqueue tasks, list/update/remove from queue |
| `WorkerService` | Worker pool status |

## Proto generation

Proto files are compiled at build time via `build.rs` using `tonic-build`. After editing `.proto` files, just `cargo build`.
