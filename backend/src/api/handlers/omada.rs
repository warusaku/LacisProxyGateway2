//! Omada API handlers

use axum::{extract::State, Json};

use crate::omada::OmadaClient;
use crate::proxy::ProxyState;

/// Get Omada network status
pub async fn get_network_status(
    State(state): State<ProxyState>,
) -> Json<serde_json::Value> {
    let client = OmadaClient::new(state.app_state.mysql.clone());
    let status = client.get_network_status().await;
    Json(serde_json::to_value(status).unwrap_or_default())
}

/// Test Omada connection
pub async fn test_connection(
    State(state): State<ProxyState>,
) -> Json<serde_json::Value> {
    let client = OmadaClient::new(state.app_state.mysql.clone());

    // Try to load config
    match client.load_config().await {
        Ok(true) => {
            // Try to get devices
            match client.get_devices().await {
                Ok(devices) => {
                    Json(serde_json::json!({
                        "success": true,
                        "message": format!("Connected! Found {} devices", devices.len()),
                        "devices": devices.len()
                    }))
                }
                Err(e) => {
                    Json(serde_json::json!({
                        "success": false,
                        "message": format!("Connection failed: {}", e)
                    }))
                }
            }
        }
        Ok(false) => {
            Json(serde_json::json!({
                "success": false,
                "message": "Omada not configured"
            }))
        }
        Err(e) => {
            Json(serde_json::json!({
                "success": false,
                "message": format!("Config load failed: {}", e)
            }))
        }
    }
}
