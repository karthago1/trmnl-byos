use std::{collections::HashMap, net::SocketAddr};

use axum::{
    Json,
    body::Body,
    extract::{ConnectInfo, Path, State},
    http::{Response, StatusCode, header},
};

use serde::{Deserialize, Serialize};
use trmnl::{DeviceInfo, LogEntry, LogResponse};

use crate::{
    app_state::{AddrInfo, AppState, SharedState},
    ui::{self, Item, Position, Section, Ui, Value},
};

/// Response for GET /api/setup endpoint.
///
/// Sent when device first connects. The device stores this configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupResponse {
    /// API key (can be any string for BYOS)
    pub api_key: String,

    /// Friendly device name
    pub friendly_id: String,

    /// Initial image URL
    pub image_url: String,

    /// Welcome message
    pub message: String,

    /// Status code (200 = success)
    pub status: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayResponse {
    /// Status code (0 = success)
    pub status: u32,

    /// Full URL to the display image
    pub image_url: String,

    /// Filename for change detection.
    ///
    /// **CRITICAL**: The firmware compares this to the previous filename.
    /// If they match, it skips the display refresh!
    /// Use timestamps in filenames to ensure updates are detected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Whether to trigger firmware update
    pub update_firmware: bool,

    /// URL to firmware binary (if update_firmware is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_url: Option<String>,

    /// Refresh rate in seconds.
    ///
    /// **Note**: Must be a string (firmware expects string).
    pub refresh_rate: String,

    /// Whether to reset the device
    pub reset_firmware: bool,

    pub special_function: String,
    pub maximum_compatibility: bool,

    pub temperature_profile: String,
}

impl DisplayResponse {
    /// Create a new display response.
    ///
    /// # Arguments
    ///
    /// * `image_url` - Full URL to the PNG image
    /// * `filename` - Filename for change detection (use timestamps!)
    pub fn new(image_url: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            status: 0,
            image_url: image_url.into(),
            filename: Some(filename.into()),
            update_firmware: false,
            firmware_url: None,
            refresh_rate: "60".to_string(),
            reset_firmware: false,
            maximum_compatibility: false,
            temperature_profile: "a".to_string(),
            special_function: "none".to_string(),
        }
    }

    /// Set custom refresh rate (in seconds).
    #[must_use]
    pub fn with_refresh_rate(mut self, seconds: u32) -> Self {
        self.refresh_rate = seconds.to_string();
        self
    }
}

/// GET /api/setup - Device registration
pub async fn setup(
    device: DeviceInfo,
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<AddrInfo>,
) -> Json<SetupResponse> {
    println!("Device {} requesting setup", device.mac_address);

    let mut state = state.lock().expect("setup. fauled to lock");

    if state.welcome_image_name.is_empty() {
        let ui = Ui {
            width: 800,
            height: 480,
            rectangles: Vec::new(),
            sections: vec![Section {
                pos: Position {
                    x: Value::Absolute(0),
                    y: Value::Absolute(0),
                },
                height: Value::Relative(1.),
                width: Value::Relative(1.),
                items: vec![Item::Text(ui::Text {
                    text: "Welcome to my home".to_string(),
                    pos: Position {
                        x: Value::Center,
                        y: Value::Center,
                    },
                    size: 60.,
                })],
            }],
        };
        state.welcome_image = ui::render_ui(&ui, Default::default()).expect("failed to ŕender");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        state.welcome_image_name = format!("wel-id-{timestamp}.png");
    }

    let url = format!("http://{}/api/img/{}", addr.local, state.welcome_image_name);

    Json(SetupResponse {
        status: 200,
        api_key: "mybyos-12".to_string(),
        friendly_id: format!("trmnl-{}", device.short_id()),
        image_url: url,
        message: "Welcome to BYOS!".to_string(),
    })
}

pub fn create_img(
    state: &mut AppState,
    addr: SocketAddr,
    battery: Option<u8>,
) -> anyhow::Result<(String, String)> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let img_name = format!("m-id-{timestamp}.png");

    if let Some(battery) = battery {
        let _ = state
            .data
            .insert("battery".to_string(), battery.to_string());
    }
    state.image = ui::render_ui(&state.ui, state.data.clone())?;

    state.image_name = img_name.clone();

    let url = format!("http://{addr}/api/img/{img_name}");
    Ok((url, img_name))
}

/// GET /api/display - Main display endpoint
pub async fn display(
    device: DeviceInfo,
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<AddrInfo>,
) -> Json<DisplayResponse> {
    println!(
        "Device {} requesting display (battery: {:?}%) ver {:?}",
        device.mac_address,
        device.battery_percentage(),
        device.firmware_version
    );

    match state.lock() {
        Ok(mut state) => {
            let bat = device.battery_percentage().unwrap_or_default();
            match create_img(&mut state, addr.local, Some(bat)) {
                Ok((url, img_name)) => {
                    return Json(
                        DisplayResponse::new(url, img_name).with_refresh_rate(state.refresh_time_s),
                    );
                }
                Err(err) => {
                    println!("display: {err}");
                }
            }
        }
        Err(err) => {
            eprintln!("display: {err}");
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

pub async fn download(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<axum::response::Response, StatusCode> {
    let state = state.lock().map_err(|err| {
        eprintln!("downlaod: {err}");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    if id != state.image_name {
        return Err(StatusCode::NOT_FOUND);
    }

    let image = if id.starts_with("wel-") {
        state.welcome_image.clone()
    } else {
        state.image.clone()
    };

    Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{id}\""),
        )
        .body(Body::from(image))
        .map_err(|err| {
            eprintln!("downlaod: {err}");
            StatusCode::SERVICE_UNAVAILABLE
        })
}
