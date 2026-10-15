//! Server executable entry point.

// src/bin/server.rs — starts the executable
#[tokio::main]
async fn main() {
    network_handler::server::run().await;
}