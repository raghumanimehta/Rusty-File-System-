//! Server-side connection handling.

pub mod session;

pub async fn run() {
    // Start the server and accept connections
    if let Err(e) = session::accept_new_connections_in_tasks().await {
        eprintln!("Server error: {}", e);
    }
}