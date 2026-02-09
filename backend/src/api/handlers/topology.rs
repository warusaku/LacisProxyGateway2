//! CelestialGlobe topology API v2
//!
//! Reads user_object_detail (SSoT) and returns topology data.
//! Layout computation is done entirely on the frontend (DFS O(n)).
//! No position data is stored or returned from the backend.
//!
//! Architecture:
//! - Backend: user_object_detail → nodes + edges (no position)
//! - Frontend: DFS layout from (parent_id, sort_order) → positions

use axum::{
    extract::{Extension, Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::api::auth_middleware::require_permission;
use crate::db::mongo::topology::{LogicDeviceDoc, TopologyStateDoc};
use crate::db::mongo::user_object_detail::UserObjectDetail;
use crate::error::AppError;
use crate::models::{AuthUser, ConfirmQuery, ConfirmRequired};
use crate::proxy::ProxyState;
use crate::user_object_ingester::logic_device_pseudo_mac;

// ============================================================================
// Response types — v2 (no position)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct TopologyV2Response {
    pub nodes: Vec<TopologyNodeV2>,
    pub edges: Vec<TopologyEdge>,
    pub metadata: TopologyMetadata,
    pub view_config: ViewConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyNodeV2 {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub mac: Option<String>,
    pub ip: Option<String>,
    pub source: String,
    pub parent_id: Option<String>,
    pub order: u32,
    pub lacis_id: Option<String>,
    pub candidate_lacis_id: Option<String>,
    pub device_type: Option<String>,
    pub product_type: Option<String>,
    pub network_device_type: Option<String>,
    pub status: String,
    pub state_type: String,
    pub metadata: serde_json::Value,
    pub collapsed: bool,
    pub collapsed_child_count: usize,
    pub descendant_count: usize,
    pub connection_type: String,
    pub fid: Option<String>,
    pub facility_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TopologyEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TopologyMetadata {
    pub total_devices: usize,
    pub total_clients: usize,
    pub controllers: usize,
    pub routers: usize,
    pub logic_devices: usize,
    pub generated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ViewConfig {
    pub collapsed_node_ids: Vec<String>,
}

// ============================================================================
// Legacy v1 types (kept for backward compatibility)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct TopologyResponse {
    pub nodes: Vec<TopologyNodeV1>,
    pub edges: Vec<TopologyEdge>,
    pub metadata: TopologyMetadataV1,
}

#[derive(Debug, Serialize)]
pub struct TopologyNodeV1 {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub mac: Option<String>,
    pub ip: Option<String>,
    pub source: String,
    pub parent_id: Option<String>,
    pub lacis_id: Option<String>,
    pub candidate_lacis_id: Option<String>,
    pub product_type: Option<String>,
    pub network_device_type: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct TopologyMetadataV1 {
    pub total_devices: usize,
    pub total_clients: usize,
    pub controllers: usize,
    pub routers: usize,
    pub generated_at: String,
}

// ============================================================================
// Query parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TopologyV2Query {
    /// "full" (default) | "routes" | "site"
    #[serde(default = "default_view")]
    pub view: String,
    /// Facility ID (required when view=site)
    pub fid: Option<String>,
    /// Apply collapsed state (default true)
    #[serde(default = "default_true")]
    pub collapsed: bool,
}

fn default_view() -> String {
    "full".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateLabelRequest {
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateParentRequest {
    pub new_parent_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CollapseRequest {
    pub collapsed: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrderRequest {
    pub new_order: u32,
}

#[derive(Debug, Deserialize)]
pub struct CreateLogicDeviceRequest {
    pub label: String,
    pub device_type: String,
    pub parent_id: Option<String>,
    pub ip: Option<String>,
    pub location: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLogicDeviceRequest {
    pub label: Option<String>,
    pub device_type: Option<String>,
    pub parent_id: Option<String>,
    pub ip: Option<String>,
    pub location: Option<String>,
    pub note: Option<String>,
}

// ============================================================================
// Helper: Format MAC address with colons
// ============================================================================

fn format_mac(mac: &str) -> String {
    let clean: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() == 12 {
        clean
            .as_bytes()
            .chunks(2)
            .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
            .collect::<Vec<&str>>()
            .join(":")
    } else {
        mac.to_string()
    }
}

// ============================================================================
// Internal node builder
// ============================================================================

struct RawNode {
    id: String,
    label: String,
    node_type: String,
    mac: Option<String>,
    ip: Option<String>,
    source: String,
    parent_id: Option<String>,
    order: u32,
    lacis_id: Option<String>,
    candidate_lacis_id: Option<String>,
    device_type: Option<String>,
    product_type: Option<String>,
    network_device_type: Option<String>,
    status: String,
    state_type: String,
    metadata: serde_json::Value,
    connection_type: String,
    fid: Option<String>,
    facility_name: Option<String>,
}

#[derive(Clone)]
struct RawEdge {
    from: String,
    to: String,
    edge_type: String,
    label: Option<String>,
}

/// Build raw nodes and edges from user_object_detail SSoT.
///
/// Rules:
/// 1. user_object_detail = 唯一のSSoT。存在 = 描画対象
/// 2. 全ノードは完全に等価
/// 3. ネットワーク構造: INTERNET → Gateway → Children → ...
/// 4. Gateway不在 = ネットワーク障害。孤児ノードはGatewayにフォールバック
async fn build_raw_topology(state: &ProxyState) -> (Vec<RawNode>, Vec<RawEdge>, usize, usize) {
    let mongo = &state.app_state.mongo;
    let entries = mongo.get_all_user_object_details().await.unwrap_or_default();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut device_count = 0usize;
    let mut client_count = 0usize;

    // Build _id set for orphan detection
    let id_set: HashSet<String> = entries.iter().map(|e| e.id.clone()).collect();

    // Find gateway _id for orphan fallback (rule 4)
    let gateway_id = entries
        .iter()
        .find(|e| e.node_type == "gateway")
        .map(|e| e.id.clone());

    let internet_id = "__internet__".to_string();

    // --- Convert each UserObjectDetail to RawNode ---
    for entry in &entries {
        // Determine parent_id for this node
        let parent_id = if entry.parent_id == "INTERNET" {
            Some(internet_id.clone())
        } else if id_set.contains(&entry.parent_id) {
            Some(entry.parent_id.clone())
        } else {
            // Orphan: parent_id references an _id that doesn't exist
            // Rule 4: Fallback to gateway (INTERNET直結禁止)
            Some(gateway_id.clone().unwrap_or_else(|| internet_id.clone()))
        };

        // Count devices vs clients
        match entry.node_type.as_str() {
            "client" | "wg_peer" => client_count += 1,
            "internet" => {}
            _ => device_count += 1,
        }

        nodes.push(RawNode {
            id: entry.id.clone(),
            label: entry.label.clone(),
            node_type: entry.node_type.clone(),
            mac: Some(format_mac(&entry.mac)),
            ip: entry.ip.clone(),
            source: entry.source.clone(),
            parent_id: parent_id.clone(),
            order: entry.sort_order,
            lacis_id: entry.lacis_id.clone(),
            candidate_lacis_id: entry.candidate_lacis_id.clone(),
            device_type: Some(entry.device_type.clone()),
            product_type: entry.product_type.clone(),
            network_device_type: entry.network_device_type.clone(),
            status: entry.state_type.clone(),
            state_type: entry.state_type.clone(),
            metadata: entry.metadata.clone(),
            connection_type: entry.connection_type.clone(),
            fid: entry.fid.clone(),
            facility_name: entry.facility_name.clone(),
        });

        // Create edge from parent to this node
        if let Some(ref pid) = parent_id {
            let edge_type = match entry.connection_type.as_str() {
                "wireless" => "wireless",
                "vpn" => "vpn",
                _ => "wired",
            };
            edges.push(RawEdge {
                from: pid.clone(),
                to: entry.id.clone(),
                edge_type: edge_type.to_string(),
                label: entry.ssid.clone(),
            });
        }
    }

    // --- Virtual Internet Node (root) ---
    nodes.push(RawNode {
        id: internet_id,
        label: "Internet".to_string(),
        node_type: "internet".to_string(),
        mac: None,
        ip: None,
        source: "lpg".to_string(),
        parent_id: None,
        order: 0,
        lacis_id: None,
        candidate_lacis_id: None,
        device_type: None,
        product_type: None,
        network_device_type: None,
        status: "online".to_string(),
        state_type: "online".to_string(),
        metadata: serde_json::json!({}),
        connection_type: "wired".to_string(),
        fid: None,
        facility_name: None,
    });

    (nodes, edges, device_count, client_count)
}

/// Count descendants recursively
fn count_descendants(node_id: &str, children_map: &HashMap<String, Vec<String>>) -> usize {
    let Some(kids) = children_map.get(node_id) else {
        return 0;
    };
    let mut count = kids.len();
    for kid in kids {
        count += count_descendants(kid, children_map);
    }
    count
}

// ============================================================================
// Route view mode helpers
// ============================================================================

fn filter_for_route_view(
    nodes: &[RawNode],
    edges: &[RawEdge],
    proxy_routes: &[(String, String)],
    node_ip_map: &HashMap<String, String>,
) -> (HashSet<String>, Vec<RawEdge>) {
    let mut target_node_ids: HashSet<String> = HashSet::new();
    for (_path, target_ip) in proxy_routes {
        for (node_id, ip) in node_ip_map {
            if ip == target_ip || ip.starts_with(&format!("{}:", target_ip)) {
                target_node_ids.insert(node_id.clone());
            }
        }
    }

    let parent_map: HashMap<String, String> = nodes
        .iter()
        .filter_map(|n| n.parent_id.as_ref().map(|p| (n.id.clone(), p.clone())))
        .collect();

    let mut visible_ids: HashSet<String> = HashSet::new();
    for tid in &target_node_ids {
        let mut current = tid.clone();
        visible_ids.insert(current.clone());
        while let Some(parent) = parent_map.get(&current) {
            visible_ids.insert(parent.clone());
            current = parent.clone();
        }
    }

    let route_edges: Vec<RawEdge> = edges
        .iter()
        .filter(|e| visible_ids.contains(&e.from) && visible_ids.contains(&e.to))
        .map(|e| RawEdge {
            from: e.from.clone(),
            to: e.to.clone(),
            edge_type: "route".to_string(),
            label: e.label.clone(),
        })
        .collect();

    (visible_ids, route_edges)
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/topology — v1 backward compatible
pub async fn get_topology(State(state): State<ProxyState>) -> Result<impl IntoResponse, AppError> {
    let (raw_nodes, raw_edges, device_count, client_count) = build_raw_topology(&state).await;
    let controllers = state
        .app_state
        .mongo
        .list_omada_controllers()
        .await
        .unwrap_or_default()
        .len();
    let routers = raw_nodes.iter().filter(|n| n.node_type == "router").count();

    let filtered_node_ids: HashSet<String> = raw_nodes.iter().map(|n| n.id.clone()).collect();

    let nodes: Vec<TopologyNodeV1> = raw_nodes
        .iter()
        .map(|n| TopologyNodeV1 {
            id: n.id.clone(),
            label: n.label.clone(),
            node_type: n.node_type.clone(),
            mac: n.mac.clone(),
            ip: n.ip.clone(),
            source: n.source.clone(),
            parent_id: n.parent_id.clone(),
            lacis_id: n.lacis_id.clone(),
            candidate_lacis_id: n.candidate_lacis_id.clone(),
            product_type: n.product_type.clone(),
            network_device_type: n.network_device_type.clone(),
            status: n.status.clone(),
            metadata: n.metadata.clone(),
        })
        .collect();

    let edges: Vec<TopologyEdge> = raw_edges
        .iter()
        .filter(|e| filtered_node_ids.contains(&e.from) && filtered_node_ids.contains(&e.to))
        .map(|e| TopologyEdge {
            from: e.from.clone(),
            to: e.to.clone(),
            edge_type: e.edge_type.clone(),
            label: e.label.clone(),
        })
        .collect();

    Ok(Json(TopologyResponse {
        nodes,
        edges,
        metadata: TopologyMetadataV1 {
            total_devices: device_count,
            total_clients: client_count,
            controllers,
            routers,
            generated_at: chrono::Utc::now().to_rfc3339(),
        },
    }))
}

/// GET /api/topology/v2 — full v2 response (no position, frontend computes layout)
pub async fn get_topology_v2(
    State(state): State<ProxyState>,
    Query(query): Query<TopologyV2Query>,
) -> Result<impl IntoResponse, AppError> {
    let mongo = &state.app_state.mongo;
    let (raw_nodes, raw_edges, device_count, client_count) = build_raw_topology(&state).await;

    // Load collapsed state
    let topo_state = mongo
        .get_topology_state()
        .await
        .unwrap_or(TopologyStateDoc {
            key: "global".to_string(),
            collapsed_node_ids: Vec::new(),
            last_layout_at: String::new(),
        });
    let collapsed_set: HashSet<String> = topo_state.collapsed_node_ids.iter().cloned().collect();

    // Build children map for descendant counting
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for n in &raw_nodes {
        if let Some(ref pid) = n.parent_id {
            children_map
                .entry(pid.clone())
                .or_default()
                .push(n.id.clone());
        }
    }

    // Handle view filter
    let (visible_ids, filtered_edges) = match query.view.as_str() {
        "routes" => {
            let routes = state
                .app_state
                .mysql
                .list_routes()
                .await
                .unwrap_or_default();
            let proxy_routes: Vec<(String, String)> = routes
                .iter()
                .filter(|r| r.active)
                .map(|r| {
                    let target_ip = extract_ip_from_url(&r.target);
                    (r.path.clone(), target_ip)
                })
                .collect();
            let node_ip_map: HashMap<String, String> = raw_nodes
                .iter()
                .filter_map(|n| n.ip.as_ref().map(|ip| (n.id.clone(), ip.clone())))
                .collect();
            filter_for_route_view(&raw_nodes, &raw_edges, &proxy_routes, &node_ip_map)
        }
        "site" => {
            if let Some(ref fid) = query.fid {
                let site_ids: HashSet<String> = raw_nodes
                    .iter()
                    .filter(|n| n.fid.as_ref() == Some(fid))
                    .map(|n| n.id.clone())
                    .collect();
                let mut expanded = site_ids.clone();
                for n in &raw_nodes {
                    if let Some(ref pid) = n.parent_id {
                        if site_ids.contains(pid) || expanded.contains(pid) {
                            expanded.insert(n.id.clone());
                        }
                    }
                }
                let edges: Vec<RawEdge> = raw_edges
                    .iter()
                    .filter(|e| expanded.contains(&e.from) && expanded.contains(&e.to))
                    .cloned()
                    .collect();
                (expanded, edges)
            } else {
                let all_ids: HashSet<String> = raw_nodes.iter().map(|n| n.id.clone()).collect();
                (all_ids, raw_edges.clone())
            }
        }
        _ => {
            let all_ids: HashSet<String> = raw_nodes.iter().map(|n| n.id.clone()).collect();
            (all_ids, raw_edges.clone())
        }
    };

    // Apply collapse filter
    let hidden_by_collapse: HashSet<String> = if query.collapsed {
        let mut hidden = HashSet::new();
        for collapsed_id in &collapsed_set {
            collect_descendants(collapsed_id, &children_map, &mut hidden);
        }
        hidden
    } else {
        HashSet::new()
    };

    // Build final nodes
    let controllers = mongo
        .list_omada_controllers()
        .await
        .unwrap_or_default()
        .len();
    let routers = raw_nodes.iter().filter(|n| n.node_type == "router").count();
    let logic_device_count = raw_nodes.iter().filter(|n| n.source == "manual").count();

    let nodes: Vec<TopologyNodeV2> = raw_nodes
        .iter()
        .filter(|n| visible_ids.contains(&n.id) && !hidden_by_collapse.contains(&n.id))
        .map(|n| {
            let is_collapsed = collapsed_set.contains(&n.id);
            let collapsed_child_count = if is_collapsed {
                children_map.get(&n.id).map(|c| c.len()).unwrap_or(0)
            } else {
                0
            };
            let desc_count = count_descendants(&n.id, &children_map);

            TopologyNodeV2 {
                id: n.id.clone(),
                label: n.label.clone(),
                node_type: n.node_type.clone(),
                mac: n.mac.clone(),
                ip: n.ip.clone(),
                source: n.source.clone(),
                parent_id: n.parent_id.clone(),
                order: n.order,
                lacis_id: n.lacis_id.clone(),
                candidate_lacis_id: n.candidate_lacis_id.clone(),
                device_type: n.device_type.clone(),
                product_type: n.product_type.clone(),
                network_device_type: n.network_device_type.clone(),
                status: n.status.clone(),
                state_type: n.state_type.clone(),
                metadata: n.metadata.clone(),
                collapsed: is_collapsed,
                collapsed_child_count,
                descendant_count: desc_count,
                connection_type: n.connection_type.clone(),
                fid: n.fid.clone(),
                facility_name: n.facility_name.clone(),
            }
        })
        .collect();

    let node_ids: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edges: Vec<TopologyEdge> = filtered_edges
        .iter()
        .filter(|e| node_ids.contains(&e.from) && node_ids.contains(&e.to))
        .map(|e| TopologyEdge {
            from: e.from.clone(),
            to: e.to.clone(),
            edge_type: e.edge_type.clone(),
            label: e.label.clone(),
        })
        .collect();

    Ok(Json(TopologyV2Response {
        nodes,
        edges,
        metadata: TopologyMetadata {
            total_devices: device_count,
            total_clients: client_count,
            controllers,
            routers,
            logic_devices: logic_device_count,
            generated_at: chrono::Utc::now().to_rfc3339(),
        },
        view_config: ViewConfig {
            collapsed_node_ids: topo_state.collapsed_node_ids,
        },
    }))
}

/// PUT /api/topology/nodes/:id/label — update node label via user_object_detail SSoT
pub async fn update_node_label(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(node_id): Path<String>,
    Json(req): Json<UpdateLabelRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let label = req.label.trim();
    if label.is_empty() || label.len() > 50 {
        return Err(AppError::BadRequest(
            "Label must be 1-50 characters".to_string(),
        ));
    }

    let mongo = &state.app_state.mongo;

    // Verify node exists in user_object_detail
    let _node = mongo
        .get_user_object_detail_by_id(&node_id)
        .await
        .map_err(|e| AppError::InternalError(e))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Node '{}' not found", node_id))
        })?;

    // Update label with customized=true
    mongo
        .update_user_object_detail_label(&node_id, label, true)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "label": label,
    })))
}

/// DELETE /api/topology/nodes/:id/label — revert to auto-generated label
pub async fn delete_node_label(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(node_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let mongo = &state.app_state.mongo;

    let node = mongo
        .get_user_object_detail_by_id(&node_id)
        .await
        .map_err(|e| AppError::InternalError(e))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Node '{}' not found", node_id))
        })?;

    mongo
        .update_user_object_detail_label(&node_id, &node.label, false)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "reverted": true,
    })))
}

/// PUT /api/topology/nodes/:id/parent — reparent any node via user_object_detail SSoT
pub async fn update_node_parent(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(node_id): Path<String>,
    Json(req): Json<UpdateParentRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let mongo = &state.app_state.mongo;

    // Validate: node must exist
    let _node = mongo
        .get_user_object_detail_by_id(&node_id)
        .await
        .map_err(|e| AppError::InternalError(e))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Node '{}' not found", node_id))
        })?;

    let new_parent_id = &req.new_parent_id;

    // Validate: new parent must be "INTERNET" or exist and be eligible
    if new_parent_id != "INTERNET" {
        let parent = mongo
            .get_user_object_detail_by_id(new_parent_id)
            .await
            .map_err(|e| AppError::InternalError(e))?
            .ok_or_else(|| {
                AppError::NotFound(format!("New parent '{}' not found", new_parent_id))
            })?;

        // Check parent eligibility
        if !UserObjectDetail::can_be_parent(&parent.id) {
            return Err(AppError::BadRequest(format!(
                "Node '{}' cannot be a parent (only LacisID or Logic Device nodes can be parents)",
                new_parent_id
            )));
        }

        // Circular reference check
        let entries = mongo.get_all_user_object_details().await.unwrap_or_default();
        let id_to_parent: HashMap<String, String> = entries
            .iter()
            .map(|e| (e.id.clone(), e.parent_id.clone()))
            .collect();

        let mut current = new_parent_id.clone();
        let mut visited = HashSet::new();
        while current != "INTERNET" {
            if current == node_id {
                return Err(AppError::BadRequest(
                    "Circular reference detected: new parent is a descendant of this node"
                        .to_string(),
                ));
            }
            if !visited.insert(current.clone()) {
                break;
            }
            current = id_to_parent
                .get(&current)
                .cloned()
                .unwrap_or_else(|| "INTERNET".to_string());
        }
    }

    // Update user_object_detail
    mongo
        .update_user_object_detail_parent(&node_id, new_parent_id)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "new_parent_id": new_parent_id,
    })))
}

/// PUT /api/topology/nodes/:id/order — update sibling order
pub async fn update_node_order(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(node_id): Path<String>,
    Json(req): Json<UpdateOrderRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let mongo = &state.app_state.mongo;

    // Verify node exists
    let _node = mongo
        .get_user_object_detail_by_id(&node_id)
        .await
        .map_err(|e| AppError::InternalError(e))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Node '{}' not found", node_id))
        })?;

    mongo
        .update_user_object_detail_sort_order(&node_id, req.new_order)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "new_order": req.new_order,
    })))
}

/// PUT /api/topology/nodes/:id/collapse — toggle collapse
pub async fn toggle_node_collapse(
    State(state): State<ProxyState>,
    Path(node_id): Path<String>,
    Json(req): Json<CollapseRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .app_state
        .mongo
        .set_node_collapsed(&node_id, req.collapsed)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "collapsed": req.collapsed,
    })))
}

/// POST /api/topology/logic-devices — create logic device
/// Also adds to user_object_detail SSoT and cg_logic_devices (metadata)
pub async fn create_logic_device(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<CreateLogicDeviceRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let uuid = uuid::Uuid::new_v4();
    let id = format!("logic:{}", uuid);
    let pseudo_mac = logic_device_pseudo_mac(&uuid.to_string());
    let now = chrono::Utc::now().to_rfc3339();

    let mongo = &state.app_state.mongo;

    // Resolve parent_id
    let parent_id = if let Some(ref pid) = req.parent_id {
        pid.clone()
    } else {
        // Default: find gateway or fallback to INTERNET
        let entries = mongo.get_all_user_object_details().await.unwrap_or_default();
        entries
            .iter()
            .find(|e| e.node_type == "gateway")
            .map(|e| e.id.clone())
            .unwrap_or_else(|| "INTERNET".to_string())
    };

    // Create in cg_logic_devices (metadata store)
    let doc = LogicDeviceDoc {
        id: id.clone(),
        label: req.label.clone(),
        device_type: req.device_type.clone(),
        parent_id: Some(parent_id.clone()),
        ip: req.ip.clone(),
        location: req.location.clone(),
        note: req.note.clone(),
        lacis_id: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    mongo
        .create_logic_device(&doc)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    // Create in user_object_detail SSoT
    let detail = UserObjectDetail {
        id: pseudo_mac.clone(),
        mac: pseudo_mac.clone(),
        lacis_id: None,
        device_type: "NetworkDevice".to_string(),
        parent_id,
        sort_order: 0,
        node_type: "logic_device".to_string(),
        state_type: "StaticOnline".to_string(),
        label: req.label,
        label_customized: false,
        ip: req.ip,
        hostname: None,
        source: "manual".to_string(),
        source_ref_id: Some(id.clone()),
        connection_type: "wired".to_string(),
        product_type: None,
        product_code: None,
        network_device_type: Some(req.device_type),
        candidate_lacis_id: None,
        fid: None,
        facility_name: None,
        ssid: None,
        metadata: serde_json::json!({
            "location": &req.location,
            "note": &req.note,
            "logic_device_id": &id,
        }),
        aranea_lacis_id: None,
        created_at: now.clone(),
        updated_at: now,
    };
    mongo
        .upsert_user_object_detail(&detail)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
        "mac": pseudo_mac,
        "message": "Logic device created",
    })))
}

/// PUT /api/topology/logic-devices/:id — update logic device
pub async fn update_logic_device(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateLogicDeviceRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let mut update = mongodb::bson::Document::new();
    if let Some(label) = req.label {
        update.insert("label", label);
    }
    if let Some(device_type) = req.device_type {
        update.insert("device_type", device_type);
    }
    if let Some(parent_id) = req.parent_id {
        update.insert("parent_id", parent_id);
    }
    if let Some(ip) = req.ip {
        update.insert("ip", ip);
    }
    if let Some(location) = req.location {
        update.insert("location", location);
    }
    if let Some(note) = req.note {
        update.insert("note", note);
    }
    update.insert("updated_at", chrono::Utc::now().to_rfc3339());

    let modified = state
        .app_state
        .mongo
        .update_logic_device(&id, update)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    if !modified {
        return Err(AppError::NotFound(format!(
            "Logic device '{}' not found",
            id
        )));
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
        "message": "Logic device updated",
    })))
}

/// DELETE /api/topology/logic-devices/:id — delete logic device (dangerous)
pub async fn delete_logic_device(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
    Query(confirm): Query<ConfirmQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 100)?;

    if !confirm.confirm {
        return Ok(Json(serde_json::json!(ConfirmRequired {
            action: "delete_logic_device".to_string(),
            target: id.clone(),
            warning: format!("This will permanently delete logic device '{}'", id),
            confirm_required: true,
        })));
    }

    let mongo = &state.app_state.mongo;

    // Find the pseudo-MAC for this logic device in user_object_detail
    let entries = mongo.get_all_user_object_details().await.unwrap_or_default();
    let node_entry = entries
        .iter()
        .find(|e| e.source_ref_id.as_deref() == Some(&*id));

    // Delete from user_object_detail if found
    if let Some(entry) = node_entry {
        let _ = mongo.delete_user_object_detail(&entry.id).await;
    }

    // Delete from cg_logic_devices
    let deleted = mongo
        .delete_logic_device(&id)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    if !deleted {
        return Err(AppError::NotFound(format!(
            "Logic device '{}' not found",
            id
        )));
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "message": format!("Logic device '{}' deleted", id),
    })))
}

// ============================================================================
// Helpers
// ============================================================================

fn collect_descendants(
    node_id: &str,
    children_map: &HashMap<String, Vec<String>>,
    result: &mut HashSet<String>,
) {
    if let Some(kids) = children_map.get(node_id) {
        for kid in kids {
            result.insert(kid.clone());
            collect_descendants(kid, children_map, result);
        }
    }
}

fn extract_ip_from_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }
    url.replace("http://", "")
        .replace("https://", "")
        .split('/')
        .next()
        .unwrap_or(url)
        .split(':')
        .next()
        .unwrap_or(url)
        .to_string()
}
