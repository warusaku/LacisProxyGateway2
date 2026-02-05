//! Settings handlers

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::error::AppError;
use crate::proxy::ProxyState;

use super::SuccessResponse;

/// GET /api/settings - List all settings
pub async fn list_settings(State(state): State<ProxyState>) -> Result<impl IntoResponse, AppError> {
    let settings = state.app_state.mysql.list_settings().await?;

    // Mask the Discord webhook URL
    let masked: Vec<_> = settings
        .into_iter()
        .map(|mut s| {
            if s.setting_key == "discord_webhook_url" && s.setting_value.is_some() {
                s.setting_value = Some("********".to_string());
            }
            s
        })
        .collect();

    Ok(Json(masked))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub value: Option<String>,
}

/// PUT /api/settings/:key - Update a setting
pub async fn update_setting(
    State(state): State<ProxyState>,
    Path(key): Path<String>,
    Json(payload): Json<UpdateSettingRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate setting key exists
    let existing = state.app_state.mysql.get_setting(&key).await;
    if existing.is_err() {
        return Err(AppError::NotFound(format!("Setting {} not found", key)));
    }

    let updated = state
        .app_state
        .mysql
        .set_setting(&key, payload.value.as_deref())
        .await?;

    if updated {
        tracing::info!("Updated setting: {}", key);
        Ok(Json(SuccessResponse::new("Setting updated")))
    } else {
        Err(AppError::NotFound(format!("Setting {} not found", key)))
    }
}

/// POST /api/settings/test-discord - Test Discord webhook
pub async fn test_discord_notification(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let webhook_url = state
        .app_state
        .mysql
        .get_discord_webhook_url()
        .await?
        .ok_or_else(|| AppError::BadRequest("Discord webhook URL not configured".to_string()))?;

    // Send test notification
    let client = reqwest::Client::new();
    let response = client
        .post(&webhook_url)
        .json(&serde_json::json!({
            "embeds": [{
                "title": "LacisProxyGateway2 Test",
                "description": "Discord notification is working!",
                "color": 3066993,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }]
        }))
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to send notification: {}", e)))?;

    if response.status().is_success() {
        tracing::info!("Discord test notification sent successfully");
        Ok(Json(SuccessResponse::new("Test notification sent")))
    } else {
        Err(AppError::InternalError(format!(
            "Discord returned status: {}",
            response.status()
        )))
    }
}
