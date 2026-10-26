use std::net::SocketAddr;

use axum::{extract::connect_info::Connected, serve::IncomingStream};
use tokio::net::TcpListener;

#[derive(Clone, Debug)]
pub struct AppState {
    pub img_dir: String,
    pub ui_dir: String,
}

#[derive(Clone, Debug)]
pub struct AddrInfo {
    pub local: SocketAddr,
}

impl Connected<IncomingStream<'_, TcpListener>> for AddrInfo {
    fn connect_info(stream: IncomingStream<'_, TcpListener>) -> Self {
        Self {
            local: stream
                .io()
                .local_addr()
                .expect("failed to get local address"),
        }
    }
}
