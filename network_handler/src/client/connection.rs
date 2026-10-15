//! Client-side TCP connection handling.

use std::io;
use tokio::net::TcpStream;

/// Connect to a server using the caller's Tokio runtime.
pub async fn connect(address: &str) -> io::Result<TcpStream> {
    TcpStream::connect(address).await
}

/// Close the local connection by taking ownership and dropping the stream.
/// This does not wait for the server to finish processing earlier requests.
pub fn disconnect(stream: TcpStream) {
    drop(stream);
}
