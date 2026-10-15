//! Client executable entry point.
#[tokio::main]
async fn main() {
    network_handler::client::main().await;
}
