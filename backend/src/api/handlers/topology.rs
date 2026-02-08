//! CelestialGlobe topology API
//!
//! Integrates all data sources (Omada, OpenWrt, External) into a unified
//! network topology graph for vis-network visualization.

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::error::AppError;
use crate::proxy::ProxyState;
use crate::lacis_id::{compute_network_device_lacis_id, default_product_code};

// ============================================================================
// Response types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct TopologyResponse {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub metadata: TopologyMetadata,
}

#[derive(Debug, Serialize)]
pub struct TopologyNode {
    pub id: String,
    pub label: String,
    pub node_type: String, // "gateway" | "switch" | "ap" | "router" | "client" | "external" | "controller"
    pub mac: Option<String>,
    pub ip: Option<String>,
    pub source: String, // "omada" | "openwrt" | "external"
    pub parent_id: Option<String>,
    pub lacis_id: Option<String>,
    pub candidate_lacis_id: Option<String>,
    pub product_type: Option<String>,
    pub network_device_type: Option<String>,
    pub status: String, // "online" | "offline" | "active" | "inactive"
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct TopologyEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String, // "wired" | "wireless" | "vpn" | "logical"
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TopologyMetadata {
    pub total_devices: usize,
    pub total_clients: usize,
    pub controllers: usize,
    pub routers: usize,
    pub generated_at: String,
}

// ============================================================================
// Handler
// ============================================================================

/// GET /api/topology — unified network topology from all sources
pub async fn get_topology(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let mongo = &state.app_state.mongo;

    // Fetch all data sources in parallel-ish (sequential for simplicity, all are fast)
    let omada_controllers = mongo.list_omada_controllers().await.unwrap_or_default();
    let omada_devices = mongo.get_omada_devices(None, None).await.unwrap_or_default();
    let omada_clients = mongo.get_omada_clients(None, None, None).await.unwrap_or_default();
    let omada_wg_peers = mongo.get_omada_wg_peers(None, None).await.unwrap_or_default();
    let openwrt_routers = mongo.list_openwrt_routers().await.unwrap_or_default();
    let openwrt_clients = mongo.get_openwrt_clients(None).await.unwrap_or_default();
    let external_devices = mongo.list_external_devices().await.unwrap_or_default();
    let external_clients = mongo.get_external_clients(None).await.unwrap_or_default();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Track device counts
    let mut device_count = 0usize;
    let mut client_count = 0usize;

    // --- Omada Controllers (virtual root nodes per controller) ---
    for ctrl in &omada_controllers {
        let node_id = format!("omada:{}:ctrl", ctrl.controller_id);
        nodes.push(TopologyNode {
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
        });
    }

    // --- Omada Devices (gateway, switch, AP) ---
    // Build a lookup: mac → node_id for parent resolution
    let mut omada_dev_by_mac = std::collections::HashMap::new();

    for dev in &omada_devices {
        let node_id = format!("omada:{}:dev:{}", dev.controller_id, dev.mac);
        omada_dev_by_mac.insert(dev.mac.clone(), node_id.clone());

        let candidate = compute_network_device_lacis_id(
            &dev.product_type,
            &dev.mac,
            default_product_code(&dev.network_device_type),
        );

        nodes.push(TopologyNode {
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
        });

        // Edge: controller → device
        edges.push(TopologyEdge {
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

        // Determine parent: AP (wireless) or Switch (wired) or gateway fallback
        let parent_id = if cli.wireless {
            cli.ap_mac
                .as_ref()
                .and_then(|mac| {
                    let norm_mac = crate::omada::client::normalize_mac(mac);
                    omada_dev_by_mac.get(&norm_mac).cloned()
                })
        } else {
            cli.switch_mac
                .as_ref()
                .and_then(|mac| {
                    let norm_mac = crate::omada::client::normalize_mac(mac);
                    omada_dev_by_mac.get(&norm_mac).cloned()
                })
        };

        // Fallback: find a gateway in the same controller+site
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

        let edge_type = if cli.wireless {
            "wireless"
        } else {
            "wired"
        };

        nodes.push(TopologyNode {
            id: node_id.clone(),
            label: cli
                .name
                .clone()
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
        });

        edges.push(TopologyEdge {
            from: parent_id,
            to: node_id,
            edge_type: edge_type.to_string(),
            label: cli.ssid.clone(),
        });

        client_count += 1;
    }

    // --- Omada WG Peers ---
    for peer in &omada_wg_peers {
        let node_id = format!("omada:{}:wg:{}", peer.controller_id, peer.peer_id);

        // Find gateway in same controller+site
        let parent_id = omada_devices
            .iter()
            .find(|d| {
                d.controller_id == peer.controller_id
                    && d.site_id == peer.site_id
                    && d.device_type == "gateway"
            })
            .map(|d| format!("omada:{}:dev:{}", d.controller_id, d.mac))
            .unwrap_or_else(|| format!("omada:{}:ctrl", peer.controller_id));

        nodes.push(TopologyNode {
            id: node_id.clone(),
            label: peer.name.clone(),
            node_type: "client".to_string(),
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
        });

        edges.push(TopologyEdge {
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

        // Check if this router's MAC appears as an Omada client (NAT behind Omada)
        let omada_parent = omada_clients
            .iter()
            .find(|c| {
                crate::omada::client::normalize_mac(&c.mac)
                    == crate::omada::client::normalize_mac(&router.mac)
            })
            .and_then(|c| {
                // Parent is the AP or switch the client is connected to
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

        nodes.push(TopologyNode {
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
        });

        if let Some(parent) = omada_parent {
            edges.push(TopologyEdge {
                from: parent,
                to: node_id.clone(),
                edge_type: "wired".to_string(),
                label: Some("NAT".to_string()),
            });
        }

        device_count += 1;
    }

    // --- OpenWrt Clients ---
    for cli in &openwrt_clients {
        let node_id = format!("openwrt:{}:cli:{}", cli.router_id, cli.mac);

        // Find parent router
        let parent_id = openwrt_routers
            .iter()
            .find(|r| r.router_id == cli.router_id)
            .map(|r| format!("openwrt:{}:dev:{}", r.router_id, r.mac))
            .unwrap_or_else(|| format!("openwrt:{}:dev:unknown", cli.router_id));

        nodes.push(TopologyNode {
            id: node_id.clone(),
            label: cli
                .hostname
                .clone()
                .unwrap_or_else(|| cli.mac.clone()),
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
            metadata: serde_json::json!({
                "router_id": &cli.router_id,
            }),
        });

        edges.push(TopologyEdge {
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

        // Check if this device's MAC appears as an Omada client
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

        nodes.push(TopologyNode {
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
        });

        if let Some(parent) = omada_parent {
            edges.push(TopologyEdge {
                from: parent,
                to: node_id.clone(),
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

        nodes.push(TopologyNode {
            id: node_id.clone(),
            label: cli
                .hostname
                .clone()
                .unwrap_or_else(|| cli.mac.clone()),
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
            metadata: serde_json::json!({
                "device_id": &cli.device_id,
            }),
        });

        edges.push(TopologyEdge {
            from: parent_id,
            to: node_id,
            edge_type: "wired".to_string(),
            label: None,
        });

        client_count += 1;
    }

    let now = chrono::Utc::now().to_rfc3339();

    Ok(Json(TopologyResponse {
        nodes,
        edges,
        metadata: TopologyMetadata {
            total_devices: device_count,
            total_clients: client_count,
            controllers: omada_controllers.len(),
            routers: openwrt_routers.len(),
            generated_at: now,
        },
    }))
}
