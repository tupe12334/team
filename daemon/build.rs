fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &[
                "../proto/agents.proto",
                "../proto/daemon.proto",
                "../proto/queue.proto",
                "../proto/worker.proto",
            ],
            &["../proto"],
        )?;
    Ok(())
}
