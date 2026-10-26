use std::net::SocketAddr;
use sha2::{Digest, Sha256};

pub fn generate_peer_token(addr: &SocketAddr, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(addr.ip().to_string().as_bytes());
    hasher.update(salt.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn is_loopback(addr: &SocketAddr) -> bool {
    addr.ip().is_loopback()
}
