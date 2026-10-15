//! State and request handling for one connected client.
use std::print;

use tokio::net::TcpStream;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // let task_a = tokio::spawn(async {
    //     println!("Task A is running");
    // });

    // let task_b = tokio::spawn(async {
    //     println!("Task B is running");
    // });

    // task_a.await.expect("Task A failed");
    // task_b.await.expect("Task B failed");
}


pub async fn accept_new_connections_in_tasks() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878").await?;

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            handle_connection(stream).unwrap();
        });
    }
    Ok(())
}

pub fn handle_connection(stream: TcpStream) -> std::io::Result<()> {
    // Handle the connection here
    print!("Handling connection from: {:?}", stream.peer_addr()?);
    Ok(())
}

