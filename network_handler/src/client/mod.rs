//! Public client API shared by terminal and GUI applications.

mod connection;

pub use connection::{connect, disconnect};

pub async fn main() {
    let stat = connect("127.0.0.1:7878").await;
    match stat {
        Ok(stream) => {
            println!("Connected to server");
            disconnect(stream);
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
        }
    }

}