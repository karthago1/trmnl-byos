use std::collections::HashMap;

use axum::{
    Json,
    extract::{ConnectInfo, State},
};

use serde::{Deserialize, Serialize};
use trmnl::{DeviceInfo, DisplayResponse, LogEntry, LogResponse, SetupResponse};

use crate::{
    app_state::{AddrInfo, SharedState},
    ui::{self, Ui},
};

/// GET /api/setup - Device registration
pub async fn setup(
    device: DeviceInfo,
    ConnectInfo(addr): ConnectInfo<AddrInfo>,
) -> Json<SetupResponse> {
    println!("Device {} requesting setup", device.mac_address);

    let img_url = format!("http://{}/img/welcome.png", addr.local);
    Json(SetupResponse::new(
        format!("trmnl-{}", device.short_id()),
        img_url,
        "Welcome to BYOS!",
    ))
}

/// GET /api/display - Main display endpoint
pub async fn display(
    device: DeviceInfo,
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<AddrInfo>,
) -> Json<DisplayResponse> {
    // println!(
    //     "Device {} requesting display (battery: {:?}%)",
    //     device.mac_address,
    //     device.battery_percentage()
    // );

    // In a real implementation, you would:
    // 1. Generate HTML content based on your data
    // 2. Render to PNG
    // 3. Save the PNG and return its URL

    match state.lock() {
        Ok(mut state) => {
            let img_name = format!("{}.png", device.mac_address);
            let img_path = format!("{}/{img_name}", state.img_dir);
            let _ = state.data.insert(
                "battery".to_string(),
                device.battery_percentage().unwrap_or_default().to_string(),
            );
            let res = ui::render_ui(&img_path, &state.ui, state.data.clone());

            println!("result {res:?}");
            let img_url = format!("http://{}/img/{img_name}", addr.local);
            return Json(DisplayResponse::new(img_url, img_name).with_refresh_rate(60));
        }
        Err(err) => {
            println!("display: {err}");
        }
    }

    Json(
        DisplayResponse::new(
            "http://192.168.188.52:3000/img/output.png".to_string(),
            // format!("{}.png", timestamp),
            "output.png",
        )
        .with_refresh_rate(60),
    )
}

/// POST /api/log - Device telemetry
pub async fn log(device: DeviceInfo, Json(entry): Json<LogEntry>) -> Json<LogResponse> {
    println!(
        "Log from {}: {:?} (battery: {:?}V)",
        device.mac_address,
        entry.log_message,
        entry
            .device_status_stamp
            .as_ref()
            .and_then(|s| s.battery_voltage)
    );

    Json(LogResponse::ok())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateImageResponse {
    /// Status code (0 = success)
    pub status: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateImageReq {
    pub ui: Ui,
    pub data: HashMap<String, String>,
    pub refresh_time_s: u32,
}

/// POST /api/update_image
pub async fn update_image(
    State(state): State<SharedState>,
    Json(req): Json<UpdateImageReq>,
) -> Json<UpdateImageResponse> {
    let res = match state.lock() {
        Ok(mut state) => {
            state.ui = req.ui;
            state.data = req.data;
            state.refresh_time_s = req.refresh_time_s;

            UpdateImageResponse {
                status: 1,
                error: None,
            }
        }
        Err(err) => UpdateImageResponse {
            status: 1,
            error: Some(err.to_string()),
        },
    };
    Json(res)
}
