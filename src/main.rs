//! Basic BYOS server example
//!
//! Run with: cargo run --example basic_byos --features axum
//!
//! Then test with:
//!   curl -H "ID: test-device" http://localhost:3000/api/display

mod api;
mod app_state;
mod ui;

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, Mutex},
};

use axum::{Router, routing::get};
use clap::Parser;

use crate::{
    api::{display, download, log, setup, update_image},
    app_state::{AddrInfo, AppState},
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED , 3000)))]
    addr: SocketAddr,
    // /// path to the ui directory, the dir should contain [MAC_ADDRESS].sh and [MAC_ADDRESS].json
    // ui_dir: String,
}

#[tokio::main]
async fn main() {
    println!("Starting TRMNL BYOS server on http://localhost:3000");
    println!();
    println!("Endpoints:");
    println!("  GET  /api/setup   - Device registration");
    println!("  GET  /api/display - Get display image");
    println!("  POST /api/log     - Device telemetry");
    println!();
    println!("Test with:");
    println!("  curl -H 'ID: test-device' http://localhost:3000/api/display");

    let args = Args::parse();

    let app = Router::new()
        .route("/api/setup", get(setup))
        .route("/api/display", get(display))
        .route("/api/log", axum::routing::post(log))
        .route("/api/update_image", axum::routing::post(update_image))
        .route("/api/img/{id}", get(download))
        .with_state(Arc::new(Mutex::new(AppState::new())));
    // .nest_service("/img", ServeDir::new(args.img_dir));

    let listener = tokio::net::TcpListener::bind(args.addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<AddrInfo>(),
    )
    .await
    .unwrap();
}
