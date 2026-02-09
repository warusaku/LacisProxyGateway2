//! WireGuard API handlers
//!
//! Key generation, peer CRUD (via Omada OpenAPI), config file generation.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::models::{AuthUser, ConfirmRequired};
use crate::omada::client::{CreateWgPeerRequest, UpdateWgPeerRequest};
use crate::proxy::ProxyState;
use crate::wireguard::{config as wg_config, keygen};

// ============================================================================
// Request types
// ============================================================================

#[derive(Deserialize)]
pub struct CreatePeerApiRequest {
    pub controller_id: String,
    pub site_id: String,
    pub name: String,
    pub interface_id: String,
    pub public_key: String,
    pub allow_address: Vec<String>,
    pub keep_alive: Option<i32>,
    pub comment: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdatePeerApiRequest {
    pub controller_id: String,
    pub site_id: String,
    pub name: Option<String>,
    pub allow_address: Option<Vec<String>>,
    pub keep_alive: Option<i32>,
    pub comment: Option<String>,
}

#[derive(Deserialize)]
pub struct DeletePeerQuery {
    pub controller_id: String,
    pub site_id: String,
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Deserialize, Default)]
pub struct WgInterfaceQuery {
    pub controller_id: Option<String>,
    pub site_id: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/wireguard/keypair - Generate a new key pair
pub async fn generate_keypair() -> Json<serde_json::Value> {
    let keypair = keygen::generate_keypair();
    Json(serde_json::json!({
        "ok": true,
        "private_key": keypair.private_key,
        "public_key": keypair.public_key,
    }))
}

/// POST /api/wireguard/peers - Create a peer via Omada OpenAPI (admin: permission >= 80)
pub async fn create_peer(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<CreatePeerApiRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let client = match state.omada_manager.get_client(&req.controller_id).await {
        Some(c) => c,
        None => {
            return Ok(Json(serde_json::json!({
                "ok": false,
                "error": format!("Controller {} not found", req.controller_id),
            })));
        }
    };

    let omada_req = CreateWgPeerRequest {
        name: req.name,
        interface_id: req.interface_id,
        public_key: req.public_key,
        allow_address: req.allow_address,
        keep_alive: req.keep_alive,
        comment: req.comment,
    };

    match client.create_wireguard_peer(&req.site_id, &omada_req).await {
        Ok(result) => {
            let syncer = crate::omada::OmadaSyncer::new(
                state.omada_manager.clone(),
                state.app_state.mongo.clone(),
            );
            let _ = syncer.sync_one(&req.controller_id).await;

            Ok(Json(serde_json::json!({
                "ok": true,
                "peer": result,
            })))
        }
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// PUT /api/wireguard/peers/:id - Update a peer via Omada OpenAPI (admin: permission >= 80)
pub async fn update_peer(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(peer_id): Path<String>,
    Json(req): Json<UpdatePeerApiRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let client = match state.omada_manager.get_client(&req.controller_id).await {
        Some(c) => c,
        None => {
            return Ok(Json(serde_json::json!({
                "ok": false,
                "error": format!("Controller {} not found", req.controller_id),
            })));
        }
    };

    let omada_req = UpdateWgPeerRequest {
        name: req.name,
        allow_address: req.allow_address,
        keep_alive: req.keep_alive,
        comment: req.comment,
    };

    match client
        .update_wireguard_peer(&req.site_id, &peer_id, &omada_req)
        .await
    {
        Ok(()) => {
            let syncer = crate::omada::OmadaSyncer::new(
                state.omada_manager.clone(),
                state.app_state.mongo.clone(),
            );
            let _ = syncer.sync_one(&req.controller_id).await;

            Ok(Json(serde_json::json!({
                "ok": true,
                "message": format!("Peer {} updated", peer_id),
            })))
        }
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// DELETE /api/wireguard/peers/:id - Delete a peer via Omada OpenAPI (dangerous: permission == 100, confirm required)
pub async fn delete_peer(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(peer_id): Path<String>,
    Query(q): Query<DeletePeerQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 100)?;

    // Confirm guard
    if !q.confirm {
        return Ok(Json(serde_json::json!(ConfirmRequired {
            action: "delete_wireguard_peer".to_string(),
            target: format!("WireGuard peer {}", peer_id),
            warning: "This will delete the WireGuard peer from the Omada controller. VPN connectivity will be lost.".to_string(),
            confirm_required: true,
        })));
    }

    let client = match state.omada_manager.get_client(&q.controller_id).await {
        Some(c) => c,
        None => {
            return Ok(Json(serde_json::json!({
                "ok": false,
                "error": format!("Controller {} not found", q.controller_id),
            })));
        }
    };

    match client.delete_wireguard_peer(&q.site_id, &peer_id).await {
        Ok(()) => {
            let syncer = crate::omada::OmadaSyncer::new(
                state.omada_manager.clone(),
                state.app_state.mongo.clone(),
            );
            let _ = syncer.sync_one(&q.controller_id).await;

            Ok(Json(serde_json::json!({
                "ok": true,
                "message": format!("Peer {} deleted", peer_id),
            })))
        }
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// POST /api/wireguard/config - Generate a WireGuard client config file
pub async fn generate_config(
    Json(params): Json<wg_config::WgClientConfigParams>,
) -> Json<serde_json::Value> {
    let config_str = wg_config::generate_config(&params);
    Json(serde_json::json!({
        "ok": true,
        "config": config_str,
    }))
}

/// GET /api/wireguard/interfaces - WG interfaces (aggregated from peers)
pub async fn get_interfaces(
    State(state): State<ProxyState>,
    Query(q): Query<WgInterfaceQuery>,
) -> Json<serde_json::Value> {
    match state
        .app_state
        .mongo
        .get_omada_wg_peers(q.controller_id.as_deref(), q.site_id.as_deref())
        .await
    {
        Ok(peers) => {
            // Group peers by interface_id → interface summary
            let mut interfaces: std::collections::HashMap<String, serde_json::Value> =
                std::collections::HashMap::new();

            for peer in &peers {
                let entry = interfaces
                    .entry(peer.interface_id.clone())
                    .or_insert_with(|| {
                        serde_json::json!({
                            "interface_id": peer.interface_id,
                            "interface_name": peer.interface_name,
                            "controller_id": peer.controller_id,
                            "site_id": peer.site_id,
                            "peer_count": 0,
                            "active_peers": 0,
                        })
                    });

                if let Some(obj) = entry.as_object_mut() {
                    let count = obj.get("peer_count").and_then(|v| v.as_u64()).unwrap_or(0);
                    obj.insert("peer_count".to_string(), serde_json::json!(count + 1));

                    if peer.status {
                        let active = obj
                            .get("active_peers")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        obj.insert("active_peers".to_string(), serde_json::json!(active + 1));
                    }
                }
            }

            let list: Vec<serde_json::Value> = interfaces.into_values().collect();
            Json(serde_json::json!({
                "ok": true,
                "interfaces": list,
                "total": list.len(),
            }))
        }
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// GET /api/wireguard/peers - List all peers (reuses omada_wg_peers)
pub async fn get_peers(
    State(state): State<ProxyState>,
    Query(q): Query<WgInterfaceQuery>,
) -> Json<serde_json::Value> {
    match state
        .app_state
        .mongo
        .get_omada_wg_peers(q.controller_id.as_deref(), q.site_id.as_deref())
        .await
    {
        Ok(peers) => Json(serde_json::json!({
            "ok": true,
            "peers": peers,
            "total": peers.len(),
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}
