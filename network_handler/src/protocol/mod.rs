//! Request and response definitions shared by the client and server.

pub mod request;
pub mod response;

pub struct Session {
    pub client_addr: std::net::SocketAddr,
    pub server_addr: std::net::SocketAddr,
}

pub fn new_session(client_addr: std::net::SocketAddr, server_addr: std::net::SocketAddr) -> Session {
    Session {
        client_addr,
        server_addr,
    }
}