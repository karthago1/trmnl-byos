use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{extract::connect_info::Connected, serve::IncomingStream};
use serde::Deserialize;
use tokio::net::TcpListener;

use crate::ui::Ui;

#[derive(Debug, Clone, Deserialize)]
pub struct AppState {
    pub ui: Ui,
    pub data: HashMap<String, String>,
    pub img_dir: String,
    pub refresh_time_s: u32,
}
impl AppState {
    pub fn new(img_dir: String) -> Self {
        Self {
            refresh_time_s: 60,
            img_dir,
            ui: Ui {
                width: 800,
                height: 480,
                sections: Vec::new(),
                rectangles: Vec::new(),
            },
            data: Default::default(),
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

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
