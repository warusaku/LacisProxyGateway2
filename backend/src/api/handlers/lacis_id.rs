//! LacisID candidate calculation and assignment API handlers

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::lacis_id::{
    compute_network_device_lacis_id, default_product_code, normalize_mac_for_lacis_id,
};
use crate::models::AuthUser;
use crate::proxy::ProxyState;

#[derive(Debug, Serialize)]
pub struct LacisIdCandidate {
    pub device_id: String,
    pub source: String,
    pub mac: String,
    pub display_name: String,
    pub product_type: String,
    pub network_device_type: String,
    pub candidate_lacis_id: String,
    pub assigned_lacis_id: Option<String>,
    pub status: String,
}

/// GET /api/lacis-id/candidates — all devices with candidate lacisIDs
pub async fn lacis_id_candidates(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let mongo = &state.app_state.mongo;

    let omada_devices = mongo
        .get_omada_devices(None, None)
        .await
        .unwrap_or_default();
    let openwrt_routers = mongo.list_openwrt_routers().await.unwrap_or_default();
    let external_devices = mongo.list_external_devices().await.unwrap_or_default();

    let mut candidates = Vec::new();

    // Omada devices
    for dev in &omada_devices {
        let candidate = compute_network_device_lacis_id(
            &dev.product_type,
            &dev.mac,
            default_product_code(&dev.network_device_type),
        );
        let status = if dev.lacis_id.is_some() {
            "assigned"
        } else {
            "unassigned"
        };
        candidates.push(LacisIdCandidate {
            device_id: dev.mac.clone(),
            source: "omada".to_string(),
            mac: dev.mac.clone(),
            display_name: dev.name.clone(),
            product_type: dev.product_type.clone(),
            network_device_type: dev.network_device_type.clone(),
            candidate_lacis_id: candidate,
            assigned_lacis_id: dev.lacis_id.clone(),
            status: status.to_string(),
        });
    }

    // OpenWrt routers
    for router in &openwrt_routers {
        let candidate = compute_network_device_lacis_id(
            &router.product_type,
            &router.mac,
            default_product_code(&router.network_device_type),
        );
        let status = if router.lacis_id.is_some() {
            "assigned"
        } else {
            "unassigned"
        };
        candidates.push(LacisIdCandidate {
            device_id: router.router_id.clone(),
            source: "openwrt".to_string(),
            mac: router.mac.clone(),
            display_name: router.display_name.clone(),
            product_type: router.product_type.clone(),
            network_device_type: router.network_device_type.clone(),
            candidate_lacis_id: candidate,
            assigned_lacis_id: router.lacis_id.clone(),
            status: status.to_string(),
        });
    }

    // External devices
    for dev in &external_devices {
        if dev.mac.is_empty() {
            continue; // Skip MAC-less logical devices
        }
        let candidate = compute_network_device_lacis_id(
            &dev.product_type,
            &dev.mac,
            default_product_code(&dev.network_device_type),
        );
        let status = if dev.lacis_id.is_some() {
            "assigned"
        } else {
            "unassigned"
        };
        candidates.push(LacisIdCandidate {
            device_id: dev.device_id.clone(),
            source: "external".to_string(),
            mac: dev.mac.clone(),
            display_name: dev.display_name.clone(),
            product_type: dev.product_type.clone(),
            network_device_type: dev.network_device_type.clone(),
            candidate_lacis_id: candidate,
            assigned_lacis_id: dev.lacis_id.clone(),
            status: status.to_string(),
        });
    }

    Ok(Json(candidates))
}

#[derive(Debug, Deserialize)]
pub struct ComputeLacisIdRequest {
    pub mac: String,
    pub product_type: String,
    pub product_code: Option<String>,
}

/// POST /api/lacis-id/compute — compute a lacisID for given MAC + product_type
pub async fn lacis_id_compute(
    Json(payload): Json<ComputeLacisIdRequest>,
) -> Result<impl IntoResponse, AppError> {
    let product_code = payload.product_code.as_deref().unwrap_or("0000");
    let normalized = normalize_mac_for_lacis_id(&payload.mac);

    if normalized.len() != 12 {
        return Err(AppError::BadRequest(
            "Invalid MAC address: must be 12 hex characters after normalization".to_string(),
        ));
    }
    if payload.product_type.len() != 3 {
        return Err(AppError::BadRequest(
            "Product type must be exactly 3 digits".to_string(),
        ));
    }

    let lacis_id =
        compute_network_device_lacis_id(&payload.product_type, &payload.mac, product_code);

    Ok(Json(serde_json::json!({
        "lacis_id": lacis_id,
        "mac": normalized,
        "product_type": payload.product_type,
        "product_code": product_code,
        "length": lacis_id.len(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct AssignLacisIdRequest {
    pub source: String,
    pub lacis_id: String,
}

/// POST /api/lacis-id/assign/:device_id — assign a candidate lacisID to a device in DB (admin: permission >= 80)
pub async fn lacis_id_assign(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(device_id): Path<String>,
    Json(payload): Json<AssignLacisIdRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    if payload.lacis_id.len() != 20 {
        return Err(AppError::BadRequest(
            "LacisID must be exactly 20 characters".to_string(),
        ));
    }

    let updated = state
        .app_state
        .mongo
        .assign_lacis_id(&payload.source, &device_id, &payload.lacis_id)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    if !updated {
        return Err(AppError::BadRequest(format!(
            "Device not found: source={}, id={}",
            payload.source, device_id
        )));
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "device_id": device_id,
        "source": payload.source,
        "lacis_id": payload.lacis_id,
    })))
}
