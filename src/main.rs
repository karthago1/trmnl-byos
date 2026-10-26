//! Basic BYOS server example
//!
//! Run with: cargo run --example basic_byos --features axum
//!
//! Then test with:
//!   curl -H "ID: test-device" http://localhost:3000/api/display

mod api;
mod app_state;
mod ui;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use axum::{Router, routing::get};
use clap::Parser;
use tower_http::services::ServeDir;

use crate::{
    api::{display, log, setup},
    app_state::{AddrInfo, AppState},
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = "/tmp/trmnl-images".to_string())]
    img_dir: String,

    #[arg(short, long, default_value_t = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED , 3000)))]
    addr: SocketAddr,

    /// path to the ui directory, the dir should contain [MAC_ADDRESS].sh and [MAC_ADDRESS].json
    ui_dir: String,
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
    std::fs::create_dir_all(&args.img_dir).expect("failed to create image folder");

    let app = Router::new()
        .route("/api/setup", get(setup))
        .route("/api/display", get(display))
        .route("/api/log", axum::routing::post(log))
        .with_state(AppState {
            img_dir: args.img_dir.clone(),
            ui_dir: args.ui_dir,
        })
        .nest_service("/img", ServeDir::new(args.img_dir));

    let listener = tokio::net::TcpListener::bind(args.addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<AddrInfo>(),
    )
    .await
    .unwrap();
}
