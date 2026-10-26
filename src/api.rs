use std::collections::HashMap;

use axum::{
    Json,
    extract::{ConnectInfo, State},
};

use trmnl::{DeviceInfo, DisplayResponse, LogEntry, LogResponse, SetupResponse};

use crate::{
    app_state::{AddrInfo, AppState},
    ui,
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
    State(state): State<AppState>,
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

    let mut cmd = std::process::Command::new(format!("{}/{}.sh", state.ui_dir, device.mac_address));
    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let img_name = format!("{}.png", device.mac_address);
                let img_path = format!("{}/{img_name}", state.img_dir);
                let ui_path = format!("{}/{}.json", state.ui_dir, device.mac_address);

                let result = serde_json::from_slice(&output.stdout).map(
                    |mut data: HashMap<String, String>| {
                        let _ = data.insert(
                            "battery".to_string(),
                            device.battery_percentage().unwrap_or_default().to_string(),
                        );
                        ui::render_ui(&img_path, &ui_path, data)
                    },
                );
                println!("result {result:?}");
                let img_url = format!("http://{}/img/{img_name}", addr.local);
                return Json(DisplayResponse::new(img_url, img_name).with_refresh_rate(60));
            } else {
                println!("display: status {}", output.status);
                // TODO show error
            }
        }
        Err(error) => {
            println!("display: {error}");
            // TODO show error
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
