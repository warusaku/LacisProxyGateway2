//! CelestialGlobe topology API v2
//!
//! Integrates all data sources (Omada, OpenWrt, External, LogicDevice) into a unified
//! network topology graph with server-side layout computation.
//!
//! Architecture:
//! - Layout computation happens server-side (Rust)
//! - Frontend receives pre-positioned nodes and renders them
//! - Frontend sends position/collapse/reparent updates back

use axum::{
    extract::{Extension, Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::api::auth_middleware::require_permission;
use crate::db::mongo::topology::{LogicDeviceDoc, NodePosition, TopologyStateDoc};
use crate::error::AppError;
use crate::models::{AuthUser, ConfirmQuery, ConfirmRequired};
use crate::node_order::logic_device_pseudo_mac;
use crate::proxy::ProxyState;

// ============================================================================
// Response types — v2
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
    pub lacis_id: Option<String>,
    pub candidate_lacis_id: Option<String>,
    pub product_type: Option<String>,
    pub network_device_type: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
    // v2 fields
    pub position: PositionV2,
    pub collapsed: bool,
    pub collapsed_child_count: usize,
    pub descendant_count: usize,
    pub connection_type: String,
    pub fid: Option<String>,
    pub facility_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionV2 {
    pub x: f64,
    pub y: f64,
    pub pinned: bool,
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
    pub last_layout_at: String,
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
pub struct UpdatePositionRequest {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub struct BatchPositionEntry {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub struct BatchUpdatePositionsRequest {
    pub positions: Vec<BatchPositionEntry>,
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
// Helper: Format MAC address with colons (e.g., "AABBCCDDEEFF" → "AA:BB:CC:DD:EE:FF")
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
        mac.to_string() // Already formatted or non-standard
    }
}

// looks_like_mac() and client_label() moved to node_order.rs (SSoT)

// ============================================================================
// Internal node builder (shared between v1 and v2)
// ============================================================================

struct RawNode {
    id: String,
    label: String,
    node_type: String,
    mac: Option<String>,
    ip: Option<String>,
    source: String,
    parent_id: Option<String>,
    lacis_id: Option<String>,
    candidate_lacis_id: Option<String>,
    product_type: Option<String>,
    network_device_type: Option<String>,
    status: String,
    metadata: serde_json::Value,
    connection_type: String,
    fid: Option<String>,
    facility_name: Option<String>,
}

struct RawEdge {
    from: String,
    to: String,
    edge_type: String,
    label: Option<String>,
}

/// Build raw nodes and edges from cg_node_order SSoT.
///
/// nodeOrder absolute rules:
/// 1. nodeOrder = 唯一のSSoT。nodeOrderに存在 = 描画対象
/// 2. 全ノードは完全に等価
/// 3. ネットワーク構造: INTERNET → Gateway → Children → ...
/// 4. Gateway不在 = ネットワーク障害。孤児ノードはGatewayにフォールバック (INTERNET直結禁止)
/// 5. Controllerは物理トポロジーに含めない
async fn build_raw_topology(state: &ProxyState) -> (Vec<RawNode>, Vec<RawEdge>, usize, usize) {
    let mongo = &state.app_state.mongo;
    let entries = mongo.get_all_node_order().await.unwrap_or_default();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut device_count = 0usize;
    let mut client_count = 0usize;

    // Build MAC set for orphan detection
    let mac_set: HashSet<String> = entries.iter().map(|e| e.mac.clone()).collect();

    // Find gateway MAC for orphan fallback (rule 4)
    let gateway_mac = entries
        .iter()
        .find(|e| e.node_type == "gateway")
        .map(|e| e.mac.clone());

    let internet_id = "__internet__".to_string();

    // --- Convert each NodeOrderEntry to RawNode ---
    for entry in &entries {
        // Determine parent_id for this node
        let parent_id = if entry.parent_mac == "INTERNET" {
            Some(internet_id.clone())
        } else if mac_set.contains(&entry.parent_mac) {
            Some(entry.parent_mac.clone())
        } else {
            // Orphan: parent_mac references a MAC that doesn't exist in nodeOrder
            // Rule 4: Fallback to gateway (INTERNET直結禁止)
            Some(gateway_mac.clone().unwrap_or_else(|| internet_id.clone()))
        };

        // Count devices vs clients
        match entry.node_type.as_str() {
            "client" | "wg_peer" => client_count += 1,
            "internet" => {}
            _ => device_count += 1,
        }

        nodes.push(RawNode {
            id: entry.mac.clone(),
            label: entry.label.clone(),
            node_type: entry.node_type.clone(),
            mac: Some(format_mac(&entry.mac)),
            ip: entry.ip.clone(),
            source: entry.source.clone(),
            parent_id: parent_id.clone(),
            lacis_id: entry.lacis_id.clone(),
            candidate_lacis_id: entry.candidate_lacis_id.clone(),
            product_type: entry.product_type.clone(),
            network_device_type: entry.network_device_type.clone(),
            status: entry.status.clone(),
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
                to: entry.mac.clone(),
                edge_type: edge_type.to_string(),
                label: entry.ssid.clone(),
            });
        }
    }

    // --- Virtual Internet Node (root) ---
    nodes.push(RawNode {
        id: internet_id.clone(),
        label: "Internet".to_string(),
        node_type: "internet".to_string(),
        mac: None,
        ip: None,
        source: "lpg".to_string(),
        parent_id: None,
        lacis_id: None,
        candidate_lacis_id: None,
        product_type: None,
        network_device_type: None,
        status: "online".to_string(),
        metadata: serde_json::json!({}),
        connection_type: "wired".to_string(),
        fid: None,
        facility_name: None,
    });

    (nodes, edges, device_count, client_count)
}

// ============================================================================
// Layout algorithm (mindmap-style, subtree-height-aware)
// ============================================================================

/// Vertical spacing between leaf nodes (minimum gap)
const LEAF_V_GAP: f64 = 60.0;
/// Extra vertical padding between subtrees of infrastructure nodes (switch/ap/router/gateway)
const SUBTREE_V_PAD: f64 = 24.0;
/// Horizontal spacing per depth level
const H_SPACING_INFRA: f64 = 300.0;
const H_SPACING_CLIENT: f64 = 240.0;

/// Node height estimate for layout purposes (matches frontend NODE_SIZES roughly)
fn node_height(node_type: &str) -> f64 {
    match node_type {
        "internet" => 72.0,
        "controller" | "lpg_server" => 100.0,
        "gateway" | "router" => 80.0,
        "switch" | "ap" | "external" => 64.0,
        "client" | "wg_peer" | "logic_device" => 52.0,
        _ => 52.0,
    }
}

fn h_spacing_for(node_type: &str) -> f64 {
    match node_type {
        "client" | "wg_peer" => H_SPACING_CLIENT,
        _ => H_SPACING_INFRA,
    }
}

fn is_infra(node_type: &str) -> bool {
    matches!(
        node_type,
        "internet"
            | "controller"
            | "gateway"
            | "router"
            | "switch"
            | "ap"
            | "lpg_server"
            | "external"
            | "logic_device"
    )
}

/// Compute the total vertical extent of a subtree rooted at `node_id`.
/// Returns the total height that this subtree occupies.
fn subtree_height(
    node_id: &str,
    node_type_map: &HashMap<String, String>,
    children_map: &HashMap<String, Vec<String>>,
    pinned_positions: &HashMap<String, NodePosition>,
) -> f64 {
    let self_h = node_type_map
        .get(node_id)
        .map(|t| node_height(t))
        .unwrap_or(52.0);

    // Pinned nodes: their subtree doesn't contribute to automatic layout sizing
    if let Some(pos) = pinned_positions.get(node_id) {
        if pos.pinned {
            return self_h;
        }
    }

    let Some(kids) = children_map.get(node_id) else {
        return self_h; // leaf node
    };
    if kids.is_empty() {
        return self_h;
    }

    // Sum of children subtree heights + gaps between them
    let mut total: f64 = 0.0;
    for (i, kid_id) in kids.iter().enumerate() {
        let kid_h = subtree_height(kid_id, node_type_map, children_map, pinned_positions);
        total += kid_h;
        if i > 0 {
            // Add gap between siblings
            let kid_is_infra = node_type_map
                .get(kid_id)
                .map(|t| is_infra(t))
                .unwrap_or(false);
            total += if kid_is_infra {
                LEAF_V_GAP + SUBTREE_V_PAD
            } else {
                LEAF_V_GAP
            };
        }
    }

    // The subtree height is at least the node's own height
    total.max(self_h)
}

fn compute_layout(
    nodes: &[RawNode],
    positions: &HashMap<String, NodePosition>,
) -> Vec<NodePosition> {
    // Build tree structures
    let mut children_ids: HashMap<String, Vec<String>> = HashMap::new();
    let mut children_nodes: HashMap<String, Vec<&RawNode>> = HashMap::new();
    let mut node_type_map: HashMap<String, String> = HashMap::new();
    let mut root_nodes: Vec<&RawNode> = Vec::new();

    for node in nodes {
        node_type_map.insert(node.id.clone(), node.node_type.clone());
        if let Some(ref pid) = node.parent_id {
            children_ids
                .entry(pid.clone())
                .or_default()
                .push(node.id.clone());
            children_nodes.entry(pid.clone()).or_default().push(node);
        } else {
            root_nodes.push(node);
        }
    }

    // Sort children: infrastructure nodes first (switch/ap/router), then clients
    // This groups network devices together visually
    for kids in children_nodes.values_mut() {
        kids.sort_by(|a, b| {
            let a_infra = is_infra(&a.node_type);
            let b_infra = is_infra(&b.node_type);
            b_infra.cmp(&a_infra).then_with(|| a.label.cmp(&b.label))
        });
    }
    // Also sort children_ids to match
    for (pid, kid_ids) in children_ids.iter_mut() {
        if let Some(sorted_nodes) = children_nodes.get(pid.as_str()) {
            *kid_ids = sorted_nodes.iter().map(|n| n.id.clone()).collect();
        }
    }

    let mut result: Vec<NodePosition> = Vec::new();

    // Compute total root subtree heights
    let root_heights: Vec<f64> = root_nodes
        .iter()
        .map(|r| subtree_height(&r.id, &node_type_map, &children_ids, positions))
        .collect();
    let total_root_h: f64 = root_heights.iter().sum::<f64>()
        + (root_nodes.len().saturating_sub(1) as f64) * (LEAF_V_GAP + SUBTREE_V_PAD);
    let mut cursor_y = -total_root_h / 2.0;

    for (i, root) in root_nodes.iter().enumerate() {
        let st_h = root_heights[i];

        // Pinned root: use stored position
        if let Some(stored) = positions.get(&root.id) {
            if stored.pinned {
                result.push(stored.clone());
                layout_subtree_v2(
                    &root.id,
                    stored.x,
                    stored.y,
                    &children_nodes,
                    &children_ids,
                    &node_type_map,
                    positions,
                    &mut result,
                );
                cursor_y += st_h + LEAF_V_GAP + SUBTREE_V_PAD;
                continue;
            }
        }

        let x = 0.0;
        let y = cursor_y + st_h / 2.0; // center of this subtree extent
        result.push(NodePosition {
            node_id: root.id.clone(),
            x,
            y,
            pinned: false,
        });

        layout_subtree_v2(
            &root.id,
            x,
            y,
            &children_nodes,
            &children_ids,
            &node_type_map,
            positions,
            &mut result,
        );

        cursor_y += st_h + LEAF_V_GAP + SUBTREE_V_PAD;
    }

    result
}

/// Layout children of `parent_id` using subtree-height-aware cumulative offset.
/// Each child is positioned at parent_x + h_spacing, with Y determined by
/// the cumulative sum of preceding sibling subtree heights.
fn layout_subtree_v2(
    parent_id: &str,
    parent_x: f64,
    parent_y: f64,
    children_nodes: &HashMap<String, Vec<&RawNode>>,
    children_ids: &HashMap<String, Vec<String>>,
    node_type_map: &HashMap<String, String>,
    pinned_positions: &HashMap<String, NodePosition>,
    result: &mut Vec<NodePosition>,
) {
    let Some(kids) = children_nodes.get(parent_id) else {
        return;
    };
    if kids.is_empty() {
        return;
    }

    // Compute subtree heights for each child
    let kid_heights: Vec<f64> = kids
        .iter()
        .map(|k| subtree_height(&k.id, node_type_map, children_ids, pinned_positions))
        .collect();

    // Total height = sum of subtree heights + gaps
    let mut total_h: f64 = 0.0;
    for (i, h) in kid_heights.iter().enumerate() {
        total_h += h;
        if i > 0 {
            let kid_infra = is_infra(&kids[i].node_type);
            total_h += if kid_infra {
                LEAF_V_GAP + SUBTREE_V_PAD
            } else {
                LEAF_V_GAP
            };
        }
    }

    // Start Y: center children block around parent_y
    let mut cursor_y = parent_y - total_h / 2.0;

    for (i, kid) in kids.iter().enumerate() {
        let st_h = kid_heights[i];
        let child_x = parent_x + h_spacing_for(&kid.node_type);

        // Add gap before non-first siblings
        if i > 0 {
            let kid_infra = is_infra(&kid.node_type);
            cursor_y += if kid_infra {
                LEAF_V_GAP + SUBTREE_V_PAD
            } else {
                LEAF_V_GAP
            };
        }

        // Pinned: use stored position
        if let Some(stored) = pinned_positions.get(&kid.id) {
            if stored.pinned {
                result.push(stored.clone());
                layout_subtree_v2(
                    &kid.id,
                    stored.x,
                    stored.y,
                    children_nodes,
                    children_ids,
                    node_type_map,
                    pinned_positions,
                    result,
                );
                cursor_y += st_h;
                continue;
            }
        }

        // Center this child within its subtree extent
        let y = cursor_y + st_h / 2.0;
        result.push(NodePosition {
            node_id: kid.id.clone(),
            x: child_x,
            y,
            pinned: false,
        });

        layout_subtree_v2(
            &kid.id,
            child_x,
            y,
            children_nodes,
            children_ids,
            node_type_map,
            pinned_positions,
            result,
        );

        cursor_y += st_h;
    }
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
    proxy_routes: &[(String, String)], // (path, target_ip)
    node_ip_map: &HashMap<String, String>,
) -> (HashSet<String>, Vec<RawEdge>) {
    // Find node IDs whose IP matches any proxy route target
    let mut target_node_ids: HashSet<String> = HashSet::new();
    for (_path, target_ip) in proxy_routes {
        for (node_id, ip) in node_ip_map {
            if ip == target_ip || ip.starts_with(&format!("{}:", target_ip)) {
                target_node_ids.insert(node_id.clone());
            }
        }
    }

    // Trace paths from target nodes up to root
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

    // Filter edges to only those between visible nodes
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

/// GET /api/topology/v2 — full v2 response with positions
pub async fn get_topology_v2(
    State(state): State<ProxyState>,
    Query(query): Query<TopologyV2Query>,
) -> Result<impl IntoResponse, AppError> {
    let mongo = &state.app_state.mongo;
    let (raw_nodes, raw_edges, device_count, client_count) = build_raw_topology(&state).await;

    // Load persistent state
    let saved_positions = mongo.get_all_node_positions().await.unwrap_or_default();
    // custom_labels no longer needed: nodeOrder.label + label_customized is the SSoT
    let topo_state = mongo
        .get_topology_state()
        .await
        .unwrap_or(TopologyStateDoc {
            key: "global".to_string(),
            collapsed_node_ids: Vec::new(),
            last_layout_at: String::new(),
        });

    let pos_map: HashMap<String, NodePosition> = saved_positions
        .into_iter()
        .map(|p| (p.node_id.clone(), p))
        .collect();
    let collapsed_set: HashSet<String> = topo_state.collapsed_node_ids.iter().cloned().collect();

    // Compute layout for nodes without stored positions
    let computed_positions = compute_layout(&raw_nodes, &pos_map);
    let mut final_pos_map: HashMap<String, NodePosition> = computed_positions
        .into_iter()
        .map(|p| (p.node_id.clone(), p))
        .collect();
    // Override with stored pinned positions
    for (id, pos) in &pos_map {
        if pos.pinned {
            final_pos_map.insert(id.clone(), pos.clone());
        }
    }

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
            // Get proxy routes
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
                // Include children of site nodes
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
    // Controller count from omada_controllers (not in nodeOrder per rule 6)
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
            let pos = final_pos_map.get(&n.id).cloned().unwrap_or(NodePosition {
                node_id: n.id.clone(),
                x: 0.0,
                y: 0.0,
                pinned: false,
            });
            let is_collapsed = collapsed_set.contains(&n.id);
            let collapsed_child_count = if is_collapsed {
                children_map.get(&n.id).map(|c| c.len()).unwrap_or(0)
            } else {
                0
            };
            let desc_count = count_descendants(&n.id, &children_map);

            // Label comes directly from nodeOrder SSoT
            // (label_customized entries are preserved during ingestion upsert)

            TopologyNodeV2 {
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
                position: PositionV2 {
                    x: pos.x,
                    y: pos.y,
                    pinned: pos.pinned,
                },
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
            last_layout_at: topo_state.last_layout_at,
        },
    }))
}

/// POST /api/topology/layout — recalculate layout
pub async fn recalc_topology_layout(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let mongo = &state.app_state.mongo;
    let (raw_nodes, _raw_edges, _, _) = build_raw_topology(&state).await;

    // Only keep pinned positions
    let saved = mongo.get_all_node_positions().await.unwrap_or_default();
    let pinned_map: HashMap<String, NodePosition> = saved
        .into_iter()
        .filter(|p| p.pinned)
        .map(|p| (p.node_id.clone(), p))
        .collect();

    let new_positions = compute_layout(&raw_nodes, &pinned_map);
    mongo
        .batch_upsert_node_positions(&new_positions)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    let now = chrono::Utc::now().to_rfc3339();
    mongo
        .set_last_layout_at(&now)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "Layout recalculated",
        "nodes_positioned": new_positions.len(),
        "last_layout_at": now,
    })))
}

/// PUT /api/topology/nodes/:id/position — update node position
pub async fn update_node_position(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(node_id): Path<String>,
    Json(req): Json<UpdatePositionRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let pos = NodePosition {
        node_id: node_id.clone(),
        x: req.x,
        y: req.y,
        pinned: true,
    };
    state
        .app_state
        .mongo
        .upsert_node_position(&pos)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
    })))
}

/// PUT /api/topology/nodes/batch-positions — batch update node positions
pub async fn batch_update_positions(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<BatchUpdatePositionsRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let positions: Vec<NodePosition> = req
        .positions
        .iter()
        .map(|e| NodePosition {
            node_id: e.node_id.clone(),
            x: e.x,
            y: e.y,
            pinned: true,
        })
        .collect();

    state
        .app_state
        .mongo
        .batch_upsert_node_positions(&positions)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "updated_count": positions.len(),
    })))
}

/// PUT /api/topology/nodes/:id/label — update node label via nodeOrder SSoT
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

    // Verify node exists in nodeOrder
    let _node = mongo
        .get_node_order_by_mac(&node_id)
        .await
        .map_err(|e| AppError::InternalError(e))?
        .ok_or_else(|| AppError::NotFound(format!("Node '{}' not found in nodeOrder", node_id)))?;

    // Update label with customized=true (prevents ingestion from overwriting)
    mongo
        .update_node_order_label(&node_id, label, true)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "label": label,
    })))
}

/// DELETE /api/topology/nodes/:id/label — revert to auto-generated label via nodeOrder SSoT
pub async fn delete_node_label(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(node_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let mongo = &state.app_state.mongo;

    // Verify node exists
    let node = mongo
        .get_node_order_by_mac(&node_id)
        .await
        .map_err(|e| AppError::InternalError(e))?
        .ok_or_else(|| AppError::NotFound(format!("Node '{}' not found in nodeOrder", node_id)))?;

    // Set label_customized=false so next ingestion cycle will overwrite with auto-generated label
    mongo
        .update_node_order_label(&node_id, &node.label, false)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "reverted": true,
    })))
}

/// PUT /api/topology/nodes/:id/parent — reparent any node via nodeOrder SSoT
pub async fn update_node_parent(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(node_id): Path<String>,
    Json(req): Json<UpdateParentRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let mongo = &state.app_state.mongo;

    // Validate: node must exist in nodeOrder
    let node = mongo
        .get_node_order_by_mac(&node_id)
        .await
        .map_err(|e| AppError::InternalError(e))?
        .ok_or_else(|| AppError::NotFound(format!("Node '{}' not found in nodeOrder", node_id)))?;

    // Validate: new parent must be "INTERNET" or exist in nodeOrder
    let new_parent_mac = &req.new_parent_id;
    let new_depth = if new_parent_mac == "INTERNET" {
        1
    } else {
        let parent = mongo
            .get_node_order_by_mac(new_parent_mac)
            .await
            .map_err(|e| AppError::InternalError(e))?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "New parent '{}' not found in nodeOrder",
                    new_parent_mac
                ))
            })?;

        // Circular reference check: walk up from new_parent to ensure node_id is not an ancestor
        let entries = mongo.get_all_node_order().await.unwrap_or_default();
        let mac_to_parent: HashMap<String, String> = entries
            .iter()
            .map(|e| (e.mac.clone(), e.parent_mac.clone()))
            .collect();

        let mut current = new_parent_mac.clone();
        let mut visited = HashSet::new();
        while current != "INTERNET" {
            if current == node_id {
                return Err(AppError::BadRequest(
                    "Circular reference detected: new parent is a descendant of this node"
                        .to_string(),
                ));
            }
            if !visited.insert(current.clone()) {
                break; // Safety: prevent infinite loop on broken data
            }
            current = mac_to_parent
                .get(&current)
                .cloned()
                .unwrap_or_else(|| "INTERNET".to_string());
        }

        parent.depth + 1
    };

    // Update nodeOrder
    mongo
        .update_node_order_parent(&node_id, new_parent_mac, new_depth)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    // Also update logic device if this is one (keep cg_logic_devices in sync)
    if node.source == "manual" {
        if let Some(ref source_ref) = node.source_ref_id {
            let update = mongodb::bson::doc! { "parent_id": new_parent_mac };
            let _ = mongo.update_logic_device(source_ref, update).await;
        }
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "new_parent_id": new_parent_mac,
        "new_depth": new_depth,
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

/// POST /api/topology/logic-devices — create logic device (also adds to nodeOrder SSoT)
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

    // Resolve parent_mac: req.parent_id is now a MAC (or None → gateway fallback)
    let mongo = &state.app_state.mongo;
    let parent_mac = if let Some(ref pid) = req.parent_id {
        pid.clone()
    } else {
        // Default: find gateway or fallback to INTERNET
        let entries = mongo.get_all_node_order().await.unwrap_or_default();
        entries
            .iter()
            .find(|e| e.node_type == "gateway")
            .map(|e| e.mac.clone())
            .unwrap_or_else(|| "INTERNET".to_string())
    };

    let parent_depth = if parent_mac == "INTERNET" {
        0
    } else {
        mongo
            .get_node_order_by_mac(&parent_mac)
            .await
            .ok()
            .flatten()
            .map(|e| e.depth)
            .unwrap_or(1)
    };

    // Create in cg_logic_devices (metadata store)
    let doc = LogicDeviceDoc {
        id: id.clone(),
        label: req.label.clone(),
        device_type: req.device_type.clone(),
        parent_id: Some(parent_mac.clone()),
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

    // Create in nodeOrder SSoT
    use crate::db::mongo::topology::NodeOrderEntry;
    let entry = NodeOrderEntry {
        mac: pseudo_mac.clone(),
        parent_mac,
        depth: parent_depth + 1,
        order: 0,
        label: req.label,
        node_type: "logic_device".to_string(),
        ip: req.ip,
        hostname: None,
        source: "manual".to_string(),
        source_ref_id: Some(id.clone()),
        status: "manual".to_string(),
        state_type: "manual".to_string(),
        connection_type: "wired".to_string(),
        lacis_id: None,
        candidate_lacis_id: None,
        product_type: None,
        network_device_type: Some(req.device_type),
        fid: None,
        facility_name: None,
        metadata: serde_json::json!({
            "location": &req.location,
            "note": &req.note,
            "logic_device_id": &id,
        }),
        label_customized: false,
        ssid: None,
        created_at: now.clone(),
        updated_at: now,
    };
    mongo
        .upsert_node_order(&entry)
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
/// Also removes from nodeOrder SSoT
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

    // Find the pseudo-MAC for this logic device in nodeOrder
    let entries = mongo.get_all_node_order().await.unwrap_or_default();
    let node_entry = entries
        .iter()
        .find(|e| e.source_ref_id.as_deref() == Some(&id));

    // Delete from nodeOrder if found
    if let Some(entry) = node_entry {
        let _ = mongo.delete_node_order(&entry.mac).await;
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
    // Extract host:port or just host from URL like "http://192.168.3.242:3000"
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }
    // Fallback: try to extract IP directly
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

// Make RawEdge cloneable for site filter
impl Clone for RawEdge {
    fn clone(&self) -> Self {
        RawEdge {
            from: self.from.clone(),
            to: self.to.clone(),
            edge_type: self.edge_type.clone(),
            label: self.label.clone(),
        }
    }
}
