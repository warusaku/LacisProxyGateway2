//! araneaSDK API handlers - proxy to mobes2.0 Cloud Functions

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

use crate::aranea::client::AraneaDeviceRegistration;
use crate::error::AppError;
use crate::proxy::ProxyState;

/// POST /api/aranea/register - Register a device via araneaDeviceGate
pub async fn aranea_register_device(
    State(state): State<ProxyState>,
    Json(payload): Json<AraneaDeviceRegistration>,
) -> Result<impl IntoResponse, AppError> {
    let result = state
        .aranea_client
        .register_device(&payload)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(result))
}

/// GET /api/aranea/devices - List devices via deviceStateReport (list mode)
pub async fn aranea_list_devices(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let result = state
        .aranea_client
        .get_device_states(None)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(result))
}

/// GET /api/aranea/devices/:lacis_id/state - Get device state history
pub async fn aranea_get_device_state(
    State(state): State<ProxyState>,
    Path(lacis_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let result = state
        .aranea_client
        .get_device_states(Some(&lacis_id))
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(result))
}

/// GET /api/aranea/summary - Summary of aranea config and status
pub async fn aranea_summary(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let summary = state.aranea_client.get_config_summary();
    Ok(Json(summary))
}
