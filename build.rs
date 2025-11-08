fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only build RPC code if the feature is enabled
    // Note: cfg! doesn't work in build.rs, use env var instead
    if std::env::var("CARGO_FEATURE_RPC").is_ok() {
        tonic_build::configure()
            .build_server(true)
            .build_client(false) // Server-only for now
            .compile(
                &[
                    "proto/mnemosyne/v1/types.proto",
                    "proto/mnemosyne/v1/memory.proto",
                    "proto/mnemosyne/v1/health.proto",
                ],
                &["proto"], // Include path
            )?;
    }

    Ok(())
}
