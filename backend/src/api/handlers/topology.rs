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
use crate::lacis_id::{compute_network_device_lacis_id, default_product_code};
use crate::models::{AuthUser, ConfirmQuery, ConfirmRequired};
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

/// Build raw nodes and edges from all data sources
async fn build_raw_topology(state: &ProxyState) -> (Vec<RawNode>, Vec<RawEdge>, usize, usize) {
    let mongo = &state.app_state.mongo;

    let omada_controllers = mongo.list_omada_controllers().await.unwrap_or_default();
    let omada_devices = mongo.get_omada_devices(None, None).await.unwrap_or_default();
    let omada_clients = mongo.get_omada_clients(None, None, None).await.unwrap_or_default();
    let omada_wg_peers = mongo.get_omada_wg_peers(None, None).await.unwrap_or_default();
    let openwrt_routers = mongo.list_openwrt_routers().await.unwrap_or_default();
    let openwrt_clients = mongo.get_openwrt_clients(None).await.unwrap_or_default();
    let external_devices = mongo.list_external_devices().await.unwrap_or_default();
    let external_clients = mongo.get_external_clients(None).await.unwrap_or_default();
    let logic_devices = mongo.list_logic_devices().await.unwrap_or_default();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut device_count = 0usize;
    let mut client_count = 0usize;

    // --- Omada Controllers ---
    for ctrl in &omada_controllers {
        let node_id = format!("omada:{}:ctrl", ctrl.controller_id);
        // Extract fid from first site if available
        let fid = ctrl.sites.first().and_then(|s| s.fid.clone());
        let facility_name = ctrl.sites.first().and_then(|s| s.fid_display_name.clone());
        nodes.push(RawNode {
            id: node_id,
            label: ctrl.display_name.clone(),
            node_type: "controller".to_string(),
            mac: None,
            ip: Some(ctrl.base_url.clone()),
            source: "omada".to_string(),
            parent_id: None,
            lacis_id: None,
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: Some("Controller".to_string()),
            status: ctrl.status.clone(),
            metadata: serde_json::json!({
                "controller_ver": &ctrl.controller_ver,
                "api_ver": &ctrl.api_ver,
                "sites": ctrl.sites.len(),
                "last_synced_at": &ctrl.last_synced_at,
            }),
            connection_type: "wired".to_string(),
            fid,
            facility_name,
        });
    }

    // --- Omada Devices ---
    let mut omada_dev_by_mac = HashMap::new();
    for dev in &omada_devices {
        let node_id = format!("omada:{}:dev:{}", dev.controller_id, dev.mac);
        omada_dev_by_mac.insert(dev.mac.clone(), node_id.clone());

        let candidate = compute_network_device_lacis_id(
            &dev.product_type,
            &dev.mac,
            default_product_code(&dev.network_device_type),
        );

        // Resolve fid from controller sites
        let ctrl = omada_controllers.iter().find(|c| c.controller_id == dev.controller_id);
        let site = ctrl
            .and_then(|c| c.sites.iter().find(|s| s.site_id == dev.site_id));
        let fid = site.and_then(|s| s.fid.clone());
        let facility_name = site.and_then(|s| s.fid_display_name.clone());

        nodes.push(RawNode {
            id: node_id.clone(),
            label: dev.name.clone(),
            node_type: dev.device_type.clone(),
            mac: Some(dev.mac.clone()),
            ip: dev.ip.clone(),
            source: "omada".to_string(),
            parent_id: Some(format!("omada:{}:ctrl", dev.controller_id)),
            lacis_id: dev.lacis_id.clone(),
            candidate_lacis_id: Some(candidate),
            product_type: Some(dev.product_type.clone()),
            network_device_type: Some(dev.network_device_type.clone()),
            status: if dev.status == 1 { "online".to_string() } else { "offline".to_string() },
            metadata: serde_json::json!({
                "model": &dev.model,
                "firmware_version": &dev.firmware_version,
                "site_id": &dev.site_id,
                "controller_id": &dev.controller_id,
            }),
            connection_type: "wired".to_string(),
            fid,
            facility_name,
        });

        edges.push(RawEdge {
            from: format!("omada:{}:ctrl", dev.controller_id),
            to: node_id,
            edge_type: "wired".to_string(),
            label: None,
        });
        device_count += 1;
    }

    // --- Omada Clients ---
    for cli in &omada_clients {
        let node_id = format!("omada:{}:cli:{}", cli.controller_id, cli.mac);

        let parent_id = if cli.wireless {
            cli.ap_mac.as_ref().and_then(|mac| {
                let norm_mac = crate::omada::client::normalize_mac(mac);
                omada_dev_by_mac.get(&norm_mac).cloned()
            })
        } else {
            cli.switch_mac.as_ref().and_then(|mac| {
                let norm_mac = crate::omada::client::normalize_mac(mac);
                omada_dev_by_mac.get(&norm_mac).cloned()
            })
        };

        let parent_id = parent_id.unwrap_or_else(|| {
            omada_devices
                .iter()
                .find(|d| {
                    d.controller_id == cli.controller_id
                        && d.site_id == cli.site_id
                        && d.device_type == "gateway"
                })
                .map(|d| format!("omada:{}:dev:{}", d.controller_id, d.mac))
                .unwrap_or_else(|| format!("omada:{}:ctrl", cli.controller_id))
        });

        let edge_type = if cli.wireless { "wireless" } else { "wired" };
        let conn_type = if cli.wireless { "wireless" } else { "wired" };

        nodes.push(RawNode {
            id: node_id.clone(),
            label: cli.name.clone()
                .or_else(|| cli.host_name.clone())
                .unwrap_or_else(|| cli.mac.clone()),
            node_type: "client".to_string(),
            mac: Some(cli.mac.clone()),
            ip: cli.ip.clone(),
            source: "omada".to_string(),
            parent_id: Some(parent_id.clone()),
            lacis_id: cli.lacis_id.clone(),
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: None,
            status: if cli.active { "active".to_string() } else { "inactive".to_string() },
            metadata: serde_json::json!({
                "vendor": &cli.vendor,
                "os_name": &cli.os_name,
                "ssid": &cli.ssid,
                "signal_level": &cli.signal_level,
                "traffic_down": cli.traffic_down,
                "traffic_up": cli.traffic_up,
                "uptime": cli.uptime,
            }),
            connection_type: conn_type.to_string(),
            fid: None,
            facility_name: None,
        });

        edges.push(RawEdge {
            from: parent_id,
            to: node_id,
            edge_type: edge_type.to_string(),
            label: cli.ssid.clone(),
        });
        client_count += 1;
    }

    // --- Omada WG Peers (now wg_peer type instead of client) ---
    for peer in &omada_wg_peers {
        let node_id = format!("omada:{}:wg:{}", peer.controller_id, peer.peer_id);

        let parent_id = omada_devices
            .iter()
            .find(|d| {
                d.controller_id == peer.controller_id
                    && d.site_id == peer.site_id
                    && d.device_type == "gateway"
            })
            .map(|d| format!("omada:{}:dev:{}", d.controller_id, d.mac))
            .unwrap_or_else(|| format!("omada:{}:ctrl", peer.controller_id));

        nodes.push(RawNode {
            id: node_id.clone(),
            label: peer.name.clone(),
            node_type: "wg_peer".to_string(),
            mac: None,
            ip: peer.allow_address.first().cloned(),
            source: "omada".to_string(),
            parent_id: Some(parent_id.clone()),
            lacis_id: None,
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: None,
            status: if peer.status { "active".to_string() } else { "inactive".to_string() },
            metadata: serde_json::json!({
                "interface_name": &peer.interface_name,
                "public_key": &peer.public_key,
                "allow_address": &peer.allow_address,
            }),
            connection_type: "vpn".to_string(),
            fid: None,
            facility_name: None,
        });

        edges.push(RawEdge {
            from: parent_id,
            to: node_id,
            edge_type: "vpn".to_string(),
            label: Some(peer.interface_name.clone()),
        });
        client_count += 1;
    }

    // --- OpenWrt Routers ---
    for router in &openwrt_routers {
        let node_id = format!("openwrt:{}:dev:{}", router.router_id, router.mac);

        let candidate = compute_network_device_lacis_id(
            &router.product_type,
            &router.mac,
            default_product_code(&router.network_device_type),
        );

        let omada_parent = omada_clients
            .iter()
            .find(|c| {
                crate::omada::client::normalize_mac(&c.mac)
                    == crate::omada::client::normalize_mac(&router.mac)
            })
            .and_then(|c| {
                if c.wireless {
                    c.ap_mac.as_ref().and_then(|mac| {
                        let norm = crate::omada::client::normalize_mac(mac);
                        omada_dev_by_mac.get(&norm).cloned()
                    })
                } else {
                    c.switch_mac.as_ref().and_then(|mac| {
                        let norm = crate::omada::client::normalize_mac(mac);
                        omada_dev_by_mac.get(&norm).cloned()
                    })
                }
            });

        nodes.push(RawNode {
            id: node_id.clone(),
            label: router.display_name.clone(),
            node_type: "router".to_string(),
            mac: Some(router.mac.clone()),
            ip: Some(router.ip.clone()),
            source: "openwrt".to_string(),
            parent_id: omada_parent.clone(),
            lacis_id: router.lacis_id.clone(),
            candidate_lacis_id: Some(candidate),
            product_type: Some(router.product_type.clone()),
            network_device_type: Some(router.network_device_type.clone()),
            status: router.status.clone(),
            metadata: serde_json::json!({
                "wan_ip": &router.wan_ip,
                "lan_ip": &router.lan_ip,
                "ssid_24g": &router.ssid_24g,
                "ssid_5g": &router.ssid_5g,
                "firmware_version": &router.firmware_version,
                "client_count": router.client_count,
                "uptime_seconds": router.uptime_seconds,
            }),
            connection_type: "wired".to_string(),
            fid: None,
            facility_name: None,
        });

        if let Some(parent) = omada_parent {
            edges.push(RawEdge {
                from: parent,
                to: node_id,
                edge_type: "wired".to_string(),
                label: Some("NAT".to_string()),
            });
        }
        device_count += 1;
    }

    // --- OpenWrt Clients ---
    for cli in &openwrt_clients {
        let node_id = format!("openwrt:{}:cli:{}", cli.router_id, cli.mac);
        let parent_id = openwrt_routers
            .iter()
            .find(|r| r.router_id == cli.router_id)
            .map(|r| format!("openwrt:{}:dev:{}", r.router_id, r.mac))
            .unwrap_or_else(|| format!("openwrt:{}:dev:unknown", cli.router_id));

        nodes.push(RawNode {
            id: node_id.clone(),
            label: cli.hostname.clone().unwrap_or_else(|| cli.mac.clone()),
            node_type: "client".to_string(),
            mac: Some(cli.mac.clone()),
            ip: Some(cli.ip.clone()),
            source: "openwrt".to_string(),
            parent_id: Some(parent_id.clone()),
            lacis_id: cli.lacis_id.clone(),
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: None,
            status: if cli.active { "active".to_string() } else { "inactive".to_string() },
            metadata: serde_json::json!({ "router_id": &cli.router_id }),
            connection_type: "wired".to_string(),
            fid: None,
            facility_name: None,
        });

        edges.push(RawEdge {
            from: parent_id,
            to: node_id,
            edge_type: "wired".to_string(),
            label: None,
        });
        client_count += 1;
    }

    // --- External Devices ---
    for dev in &external_devices {
        let node_id = format!("external:{}:dev:{}", dev.device_id, dev.mac);
        let candidate = if !dev.mac.is_empty() {
            Some(compute_network_device_lacis_id(
                &dev.product_type,
                &dev.mac,
                default_product_code(&dev.network_device_type),
            ))
        } else {
            None
        };

        let omada_parent = if !dev.mac.is_empty() {
            omada_clients
                .iter()
                .find(|c| {
                    crate::omada::client::normalize_mac(&c.mac)
                        == crate::omada::client::normalize_mac(&dev.mac)
                })
                .and_then(|c| {
                    if c.wireless {
                        c.ap_mac.as_ref().and_then(|mac| {
                            let norm = crate::omada::client::normalize_mac(mac);
                            omada_dev_by_mac.get(&norm).cloned()
                        })
                    } else {
                        c.switch_mac.as_ref().and_then(|mac| {
                            let norm = crate::omada::client::normalize_mac(mac);
                            omada_dev_by_mac.get(&norm).cloned()
                        })
                    }
                })
        } else {
            None
        };

        nodes.push(RawNode {
            id: node_id.clone(),
            label: dev.display_name.clone(),
            node_type: "external".to_string(),
            mac: Some(dev.mac.clone()),
            ip: Some(dev.ip.clone()),
            source: "external".to_string(),
            parent_id: omada_parent.clone(),
            lacis_id: dev.lacis_id.clone(),
            candidate_lacis_id: candidate,
            product_type: Some(dev.product_type.clone()),
            network_device_type: Some(dev.network_device_type.clone()),
            status: dev.status.clone(),
            metadata: serde_json::json!({
                "protocol": &dev.protocol,
                "device_model": &dev.device_model,
                "client_count": dev.client_count,
            }),
            connection_type: "wired".to_string(),
            fid: None,
            facility_name: None,
        });

        if let Some(parent) = omada_parent {
            edges.push(RawEdge {
                from: parent,
                to: node_id,
                edge_type: "wired".to_string(),
                label: Some("External".to_string()),
            });
        }
        device_count += 1;
    }

    // --- External Clients ---
    for cli in &external_clients {
        let node_id = format!("external:{}:cli:{}", cli.device_id, cli.mac);
        let parent_id = external_devices
            .iter()
            .find(|d| d.device_id == cli.device_id)
            .map(|d| format!("external:{}:dev:{}", d.device_id, d.mac))
            .unwrap_or_else(|| format!("external:{}:dev:unknown", cli.device_id));

        nodes.push(RawNode {
            id: node_id.clone(),
            label: cli.hostname.clone().unwrap_or_else(|| cli.mac.clone()),
            node_type: "client".to_string(),
            mac: Some(cli.mac.clone()),
            ip: cli.ip.clone(),
            source: "external".to_string(),
            parent_id: Some(parent_id.clone()),
            lacis_id: cli.lacis_id.clone(),
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: None,
            status: if cli.active { "active".to_string() } else { "inactive".to_string() },
            metadata: serde_json::json!({ "device_id": &cli.device_id }),
            connection_type: "wired".to_string(),
            fid: None,
            facility_name: None,
        });

        edges.push(RawEdge {
            from: parent_id,
            to: node_id,
            edge_type: "wired".to_string(),
            label: None,
        });
        client_count += 1;
    }

    // --- Logic Devices ---
    for dev in &logic_devices {
        let node_id = dev.id.clone();
        nodes.push(RawNode {
            id: node_id.clone(),
            label: dev.label.clone(),
            node_type: "logic_device".to_string(),
            mac: None,
            ip: dev.ip.clone(),
            source: "logic".to_string(),
            parent_id: dev.parent_id.clone(),
            lacis_id: dev.lacis_id.clone(),
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: Some(dev.device_type.clone()),
            status: "manual".to_string(),
            metadata: serde_json::json!({
                "location": &dev.location,
                "note": &dev.note,
                "created_at": &dev.created_at,
            }),
            connection_type: "wired".to_string(),
            fid: None,
            facility_name: None,
        });

        if let Some(ref parent) = dev.parent_id {
            edges.push(RawEdge {
                from: parent.clone(),
                to: node_id,
                edge_type: "logical".to_string(),
                label: None,
            });
        }
        device_count += 1;
    }

    (nodes, edges, device_count, client_count)
}

// ============================================================================
// Layout algorithm (mindmap-style)
// ============================================================================

fn compute_layout(nodes: &[RawNode], positions: &HashMap<String, NodePosition>) -> Vec<NodePosition> {
    // Build tree: parent_id → children
    let mut children: HashMap<String, Vec<&RawNode>> = HashMap::new();
    let mut root_nodes: Vec<&RawNode> = Vec::new();

    for node in nodes {
        if let Some(ref pid) = node.parent_id {
            children.entry(pid.clone()).or_default().push(node);
        } else {
            root_nodes.push(node);
        }
    }

    let mut result: Vec<NodePosition> = Vec::new();

    // Position root nodes vertically centered
    let root_spacing = 400.0;
    let total_root_height = (root_nodes.len() as f64 - 1.0) * root_spacing;
    let start_y = -total_root_height / 2.0;

    for (i, root) in root_nodes.iter().enumerate() {
        // If pinned, use stored position
        if let Some(stored) = positions.get(&root.id) {
            if stored.pinned {
                result.push(stored.clone());
                layout_subtree(&root.id, stored.x, stored.y, &children, positions, &mut result, 1);
                continue;
            }
        }

        let x = 0.0;
        let y = start_y + (i as f64) * root_spacing;
        result.push(NodePosition {
            node_id: root.id.clone(),
            x,
            y,
            pinned: false,
        });

        layout_subtree(&root.id, x, y, &children, positions, &mut result, 1);
    }

    result
}

fn layout_subtree(
    parent_id: &str,
    parent_x: f64,
    parent_y: f64,
    children_map: &HashMap<String, Vec<&RawNode>>,
    pinned_positions: &HashMap<String, NodePosition>,
    result: &mut Vec<NodePosition>,
    depth: usize,
) {
    let Some(kids) = children_map.get(parent_id) else {
        return;
    };

    let h_spacing = 300.0;
    let v_spacing = if depth > 2 { 80.0 } else { 120.0 };

    let total_height = (kids.len() as f64 - 1.0) * v_spacing;
    let start_y = parent_y - total_height / 2.0;
    let child_x = parent_x + h_spacing;

    for (i, kid) in kids.iter().enumerate() {
        // If pinned, use stored position
        if let Some(stored) = pinned_positions.get(&kid.id) {
            if stored.pinned {
                result.push(stored.clone());
                layout_subtree(&kid.id, stored.x, stored.y, children_map, pinned_positions, result, depth + 1);
                continue;
            }
        }

        let y = start_y + (i as f64) * v_spacing;
        result.push(NodePosition {
            node_id: kid.id.clone(),
            x: child_x,
            y,
            pinned: false,
        });

        layout_subtree(&kid.id, child_x, y, children_map, pinned_positions, result, depth + 1);
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
pub async fn get_topology(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let (raw_nodes, raw_edges, device_count, client_count) = build_raw_topology(&state).await;
    let controllers = raw_nodes.iter().filter(|n| n.node_type == "controller").count();
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
    let topo_state = mongo.get_topology_state().await.unwrap_or(TopologyStateDoc {
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
            children_map.entry(pid.clone()).or_default().push(n.id.clone());
        }
    }

    // Handle view filter
    let (visible_ids, filtered_edges) = match query.view.as_str() {
        "routes" => {
            // Get proxy routes
            let routes = state.app_state.mysql.list_routes().await.unwrap_or_default();
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
    let controllers = raw_nodes.iter().filter(|n| n.node_type == "controller").count();
    let routers = raw_nodes.iter().filter(|n| n.node_type == "router").count();
    let logic_device_count = raw_nodes.iter().filter(|n| n.source == "logic").count();

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

/// PUT /api/topology/nodes/:id/parent — reparent node
pub async fn update_node_parent(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(node_id): Path<String>,
    Json(req): Json<UpdateParentRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    // Only logic devices can be reparented
    let mongo = &state.app_state.mongo;
    let device = mongo
        .get_logic_device(&node_id)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    if device.is_none() {
        return Err(AppError::NotFound(format!(
            "Only logic devices can be reparented. Node '{}' not found in logic devices.",
            node_id
        )));
    }

    let update = mongodb::bson::doc! { "parent_id": &req.new_parent_id };
    mongo
        .update_logic_device(&node_id, update)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "node_id": node_id,
        "new_parent_id": req.new_parent_id,
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
pub async fn create_logic_device(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<CreateLogicDeviceRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let id = format!("logic:{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();

    let doc = LogicDeviceDoc {
        id: id.clone(),
        label: req.label,
        device_type: req.device_type,
        parent_id: req.parent_id,
        ip: req.ip,
        location: req.location,
        note: req.note,
        lacis_id: None,
        created_at: now.clone(),
        updated_at: now,
    };

    state
        .app_state
        .mongo
        .create_logic_device(&doc)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
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
        return Err(AppError::NotFound(format!("Logic device '{}' not found", id)));
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

    let deleted = state
        .app_state
        .mongo
        .delete_logic_device(&id)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    if !deleted {
        return Err(AppError::NotFound(format!("Logic device '{}' not found", id)));
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
