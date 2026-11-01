use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{extract::connect_info::Connected, serve::IncomingStream};
use tokio::net::TcpListener;

use crate::ui::Ui;

#[derive(Debug, Clone)]
pub struct AppState {
    pub ui: Ui,
    pub data: HashMap<String, String>,
    pub refresh_time_s: u32,
    pub image: Vec<u8>,
    pub welcome_image: Vec<u8>,
    pub image_name: String,
    pub welcome_image_name: String,
}
impl AppState {
    pub fn new() -> Self {
        Self {
            welcome_image: Vec::new(),
            welcome_image_name: String::new(),
            image_name: String::new(),
            image: Vec::new(),
            refresh_time_s: 60,
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
