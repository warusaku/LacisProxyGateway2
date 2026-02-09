//! NodeOrder Ingester — Ingests data from all sources into cg_node_order SSoT
//!
//! nodeOrder absolute rules:
//! 1. nodeOrder = 唯一のSSoT。nodeOrderに存在 = 描画対象
//! 2. 全ノードは完全に等価。managed/detected/pendingの区別禁止
//! 3. ネットワーク構造: INTERNET → Gateway → Children → ...
//! 4. Gateway不在 = ネットワーク障害。孤児ノードはGatewayにフォールバック (INTERNET直結禁止)
//! 5. parentMacはLacisID登録済みノードのMACのみ (永続性保証)
//! 6. Controllerは管理ソフトウェア、物理トポロジーには含めない
//!
//! upsert rules:
//! - New MAC: insert all fields (default hierarchy: gateway→INTERNET, others→gateway child)
//! - Existing MAC: update volatile fields ONLY (status, ip, hostname, metadata, updated_at)
//!   parent_mac, depth, order, label(if customized) are NEVER overwritten

use std::sync::Arc;

use crate::db::mongo::topology::NodeOrderEntry;
use crate::db::mongo::MongoDb;
use crate::lacis_id::{compute_network_device_lacis_id, default_product_code};
use crate::omada::client::normalize_mac;

/// Generate a pseudo-MAC for WireGuard peers (no physical MAC).
/// Uses F0 prefix (IEEE locally administered bit) + first 10 hex chars of peer_id.
pub fn wg_peer_pseudo_mac(peer_id: &str) -> String {
    let hex_chars: String = peer_id
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(10)
        .collect();
    let padded = format!("{:0<10}", hex_chars);
    format!("F0{}", padded).to_uppercase()
}

/// Generate a pseudo-MAC for logic devices (no physical MAC).
/// Uses F2 prefix (IEEE locally administered bit) + first 10 hex chars of UUID.
pub fn logic_device_pseudo_mac(uuid_str: &str) -> String {
    let hex_chars: String = uuid_str
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(10)
        .collect();
    let padded = format!("{:0<10}", hex_chars);
    format!("F2{}", padded).to_uppercase()
}

/// Build a human-friendly label for client nodes.
/// Priority: name (non-MAC) → hostname (non-MAC) → vendor + short MAC → formatted MAC
fn client_label(
    name: &Option<String>,
    hostname: &Option<String>,
    vendor: &Option<String>,
    mac: &str,
) -> String {
    if let Some(n) = name {
        if !n.is_empty() && !looks_like_mac(n) {
            return n.clone();
        }
    }
    if let Some(h) = hostname {
        if !h.is_empty() && !looks_like_mac(h) {
            return h.clone();
        }
    }
    let short_mac = if mac.len() >= 6 {
        &mac[mac.len() - 6..]
    } else {
        mac
    };
    let formatted_short = if short_mac.len() == 6 {
        format!(
            "{}:{}:{}",
            &short_mac[0..2],
            &short_mac[2..4],
            &short_mac[4..6]
        )
    } else {
        short_mac.to_string()
    };
    if let Some(v) = vendor {
        if !v.is_empty() {
            return format!("{} ({})", v, formatted_short);
        }
    }
    format_mac(mac)
}

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

fn looks_like_mac(s: &str) -> bool {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    clean.len() == 12
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '-' || c == '.')
}

/// NodeOrderIngester: reads source collections and upserts into cg_node_order
pub struct NodeOrderIngester {
    mongo: Arc<MongoDb>,
}

impl NodeOrderIngester {
    pub fn new(mongo: Arc<MongoDb>) -> Self {
        Self { mongo }
    }

    /// Ingest all Omada data for a specific controller into nodeOrder.
    /// Called after OmadaSyncer.sync_controller() completes.
    pub async fn ingest_omada(&self, controller_id: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();

        // Load controller info for fid/facility_name resolution
        let controllers = self
            .mongo
            .list_omada_controllers()
            .await
            .unwrap_or_default();
        let ctrl = controllers
            .iter()
            .find(|c| c.controller_id == controller_id);

        // Load devices for this controller
        let all_devices = self
            .mongo
            .get_omada_devices(None, None)
            .await
            .unwrap_or_default();
        let devices: Vec<_> = all_devices
            .iter()
            .filter(|d| d.controller_id == controller_id)
            .collect();

        // Find gateway MAC for this controller (used as default parent)
        let gateway_mac = devices
            .iter()
            .find(|d| d.device_type == "gateway")
            .map(|d| normalize_mac(&d.mac));

        // Find switch MAC(s) for AP parent inference.
        // Omada SDN topology: Gateway → Switch → AP (via PoE).
        // The OpenAPI does not expose device-to-device uplink info, so we infer:
        //   AP.parent = first switch in the same controller (if exists)
        let switch_mac = devices
            .iter()
            .find(|d| d.device_type == "switch")
            .map(|d| normalize_mac(&d.mac));

        // Build device MAC lookup for client parent resolution
        let dev_macs: std::collections::HashMap<String, String> = devices
            .iter()
            .map(|d| (normalize_mac(&d.mac), normalize_mac(&d.mac)))
            .collect();

        // --- Ingest devices (gateway, switch, ap) ---
        // Controller itself is NOT ingested (rule 6: Controller = management software)
        let mut order_counter = 0u32;
        for dev in &devices {
            let mac = normalize_mac(&dev.mac);
            let (parent_mac, depth) = if dev.device_type == "gateway" {
                ("INTERNET".to_string(), 1)
            } else if dev.device_type == "switch" {
                // Switch → parent is gateway
                (
                    gateway_mac
                        .clone()
                        .unwrap_or_else(|| "INTERNET".to_string()),
                    if gateway_mac.is_some() { 2 } else { 1 },
                )
            } else if dev.device_type == "ap" {
                // AP → parent is switch (PoE connection), fallback to gateway
                if let Some(ref sw) = switch_mac {
                    (sw.clone(), if gateway_mac.is_some() { 3 } else { 2 })
                } else {
                    (
                        gateway_mac
                            .clone()
                            .unwrap_or_else(|| "INTERNET".to_string()),
                        if gateway_mac.is_some() { 2 } else { 1 },
                    )
                }
            } else {
                // Unknown device type → gateway child
                (
                    gateway_mac
                        .clone()
                        .unwrap_or_else(|| "INTERNET".to_string()),
                    if gateway_mac.is_some() { 2 } else { 1 },
                )
            };

            let candidate = compute_network_device_lacis_id(
                &dev.product_type,
                &mac,
                default_product_code(&dev.network_device_type),
            );

            // Resolve fid from controller sites
            let site = ctrl.and_then(|c| c.sites.iter().find(|s| s.site_id == dev.site_id));
            let fid = site.and_then(|s| s.fid.clone());
            let facility_name = site.and_then(|s| s.fid_display_name.clone());

            let entry = NodeOrderEntry {
                mac: mac.clone(),
                parent_mac,
                depth,
                order: order_counter,
                label: dev.name.clone(),
                node_type: dev.device_type.clone(),
                ip: dev.ip.clone(),
                hostname: None,
                source: "omada".to_string(),
                source_ref_id: Some(format!("omada:{}:dev:{}", controller_id, dev.mac)),
                status: if dev.status == 1 {
                    "online".to_string()
                } else {
                    "offline".to_string()
                },
                state_type: "trackingOnline".to_string(),
                connection_type: "wired".to_string(),
                lacis_id: dev.lacis_id.clone(),
                candidate_lacis_id: Some(candidate),
                product_type: Some(dev.product_type.clone()),
                network_device_type: Some(dev.network_device_type.clone()),
                fid,
                facility_name,
                metadata: serde_json::json!({
                    "model": &dev.model,
                    "firmware_version": &dev.firmware_version,
                    "site_id": &dev.site_id,
                    "controller_id": controller_id,
                }),
                label_customized: false,
                ssid: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_node_order(&entry).await?;
            order_counter += 1;
        }

        // --- Ingest clients ---
        let all_clients = self
            .mongo
            .get_omada_clients(None, None, None)
            .await
            .unwrap_or_default();
        let clients: Vec<_> = all_clients
            .iter()
            .filter(|c| c.controller_id == controller_id)
            .collect();

        for cli in &clients {
            let mac = normalize_mac(&cli.mac);

            // Determine parent MAC
            let parent_mac = if cli.wireless {
                cli.ap_mac
                    .as_ref()
                    .map(|m| normalize_mac(m))
                    .filter(|m| dev_macs.contains_key(m))
            } else {
                cli.switch_mac
                    .as_ref()
                    .map(|m| normalize_mac(m))
                    .filter(|m| dev_macs.contains_key(m))
            };

            let parent_mac = parent_mac.unwrap_or_else(|| {
                gateway_mac
                    .clone()
                    .unwrap_or_else(|| "INTERNET".to_string())
            });

            // Calculate depth based on parent type
            let parent_depth = if parent_mac == "INTERNET" {
                0
            } else if Some(&parent_mac) == gateway_mac.as_ref() {
                1 // Gateway depth
            } else if Some(&parent_mac) == switch_mac.as_ref() {
                2 // Switch depth
            } else {
                // AP depth: 3 if under switch, 2 otherwise
                if switch_mac.is_some() { 3 } else { 2 }
            };

            let conn_type = if cli.wireless { "wireless" } else { "wired" };

            let entry = NodeOrderEntry {
                mac: mac.clone(),
                parent_mac,
                depth: parent_depth + 1,
                order: order_counter,
                label: client_label(&cli.name, &cli.host_name, &cli.vendor, &cli.mac),
                node_type: "client".to_string(),
                ip: cli.ip.clone(),
                hostname: cli.host_name.clone(),
                source: "omada".to_string(),
                source_ref_id: Some(format!("omada:{}:cli:{}", controller_id, cli.mac)),
                status: if cli.active {
                    "active".to_string()
                } else {
                    "inactive".to_string()
                },
                state_type: "trackingOnline".to_string(),
                connection_type: conn_type.to_string(),
                lacis_id: cli.lacis_id.clone(),
                candidate_lacis_id: None,
                product_type: None,
                network_device_type: None,
                fid: None,
                facility_name: None,
                metadata: serde_json::json!({
                    "vendor": &cli.vendor,
                    "os_name": &cli.os_name,
                    "ssid": &cli.ssid,
                    "signal_level": &cli.signal_level,
                    "traffic_down": cli.traffic_down,
                    "traffic_up": cli.traffic_up,
                    "uptime": cli.uptime,
                }),
                label_customized: false,
                ssid: cli.ssid.clone(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_node_order(&entry).await?;
            order_counter += 1;
        }

        // --- Ingest WG Peers ---
        let all_wg_peers = self
            .mongo
            .get_omada_wg_peers(None, None)
            .await
            .unwrap_or_default();
        let wg_peers: Vec<_> = all_wg_peers
            .iter()
            .filter(|p| p.controller_id == controller_id)
            .collect();

        for peer in &wg_peers {
            let pseudo_mac = wg_peer_pseudo_mac(&peer.peer_id);
            let parent_mac = gateway_mac
                .clone()
                .unwrap_or_else(|| "INTERNET".to_string());
            let depth = if parent_mac == "INTERNET" { 1 } else { 2 };

            let entry = NodeOrderEntry {
                mac: pseudo_mac,
                parent_mac,
                depth,
                order: order_counter,
                label: peer.name.clone(),
                node_type: "wg_peer".to_string(),
                ip: peer.allow_address.first().cloned(),
                hostname: None,
                source: "omada".to_string(),
                source_ref_id: Some(format!("omada:{}:wg:{}", controller_id, peer.peer_id)),
                status: if peer.status {
                    "active".to_string()
                } else {
                    "inactive".to_string()
                },
                state_type: "trackingOnline".to_string(),
                connection_type: "vpn".to_string(),
                lacis_id: None,
                candidate_lacis_id: None,
                product_type: None,
                network_device_type: None,
                fid: None,
                facility_name: None,
                metadata: serde_json::json!({
                    "interface_name": &peer.interface_name,
                    "public_key": &peer.public_key,
                    "allow_address": &peer.allow_address,
                    "peer_id": &peer.peer_id,
                }),
                label_customized: false,
                ssid: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_node_order(&entry).await?;
            order_counter += 1;
        }

        tracing::debug!(
            "[NodeOrder] Omada controller {} ingested: {} entries",
            controller_id,
            order_counter
        );
        Ok(())
    }

    /// Repair existing Omada device parent relationships.
    /// Fixes APs that were incorrectly set with gateway as parent.
    /// Called on startup to correct data from before the switch-heuristic fix.
    pub async fn repair_omada_device_parents(&self) -> Result<u32, String> {
        // Load all Omada devices from MongoDB
        let all_devices = self
            .mongo
            .get_omada_devices(None, None)
            .await
            .unwrap_or_default();

        // Group devices by controller_id
        let mut devices_by_ctrl: std::collections::HashMap<String, Vec<_>> =
            std::collections::HashMap::new();
        for dev in &all_devices {
            devices_by_ctrl
                .entry(dev.controller_id.clone())
                .or_default()
                .push(dev);
        }

        let mut fixed_count = 0u32;

        for (controller_id, devices) in &devices_by_ctrl {
            let gateway_mac = devices
                .iter()
                .find(|d| d.device_type == "gateway")
                .map(|d| normalize_mac(&d.mac));

            let switch_mac = devices
                .iter()
                .find(|d| d.device_type == "switch")
                .map(|d| normalize_mac(&d.mac));

            let Some(ref sw_mac) = switch_mac else {
                continue; // No switch → nothing to fix
            };

            // Find APs that should be under the switch but are under the gateway
            for dev in devices {
                if dev.device_type != "ap" {
                    continue;
                }

                let mac = normalize_mac(&dev.mac);
                let existing = self.mongo.get_node_order_by_mac(&mac).await;
                if let Ok(Some(entry)) = existing {
                    // Only fix if parent is gateway (not manually reparented to something else)
                    if Some(&entry.parent_mac) == gateway_mac.as_ref() {
                        let new_depth = if gateway_mac.is_some() { 3 } else { 2 };
                        let updated = self
                            .mongo
                            .update_node_order_parent(&mac, sw_mac, new_depth)
                            .await?;
                        if updated {
                            tracing::info!(
                                "[NodeOrder] Repaired AP {} parent: {} → {} (controller {})",
                                mac,
                                entry.parent_mac,
                                sw_mac,
                                controller_id
                            );
                            fixed_count += 1;
                        }
                    }
                }
            }

            // Also fix clients under APs — their depth needs adjustment
            let ap_macs: std::collections::HashSet<String> = devices
                .iter()
                .filter(|d| d.device_type == "ap")
                .map(|d| normalize_mac(&d.mac))
                .collect();

            let all_nodes = self.mongo.get_all_node_order().await.unwrap_or_default();
            for node in &all_nodes {
                if ap_macs.contains(&node.parent_mac) && node.depth != 4 {
                    // Client under AP should be depth 4 (INTERNET=0, GW=1, SW=2, AP=3, CLI=4)
                    if gateway_mac.is_some() {
                        let _ = self
                            .mongo
                            .update_node_order_parent(&node.mac, &node.parent_mac, 4)
                            .await;
                    }
                }
            }
        }

        if fixed_count > 0 {
            tracing::info!(
                "[NodeOrder] Repaired {} AP device parent relationships",
                fixed_count
            );
        }

        Ok(fixed_count)
    }

    /// Ingest OpenWrt router and its clients into nodeOrder.
    /// Called after OpenWrtSyncer.poll_router() completes.
    pub async fn ingest_openwrt(&self, router_id: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();

        let routers = self.mongo.list_openwrt_routers().await.unwrap_or_default();
        let router = routers
            .iter()
            .find(|r| r.router_id == router_id)
            .ok_or_else(|| format!("OpenWrt router {} not found", router_id))?;

        let mac = normalize_mac(&router.mac);

        // Determine parent: check if this MAC exists in Omada clients (NAT'd behind Omada device)
        let existing = self.mongo.get_node_order_by_mac(&mac).await?;
        let parent_mac = if let Some(ref existing) = existing {
            // Preserve existing parent (may have been reparented manually)
            existing.parent_mac.clone()
        } else {
            // New: try to find in Omada clients for NAT parent detection
            let omada_clients = self
                .mongo
                .get_omada_clients(None, None, None)
                .await
                .unwrap_or_default();
            let omada_match = omada_clients.iter().find(|c| normalize_mac(&c.mac) == mac);
            if let Some(cli) = omada_match {
                // Router appears as Omada client → parent is AP/switch MAC
                if cli.wireless {
                    cli.ap_mac
                        .as_ref()
                        .map(|m| normalize_mac(m))
                        .unwrap_or_else(|| "INTERNET".to_string())
                } else {
                    cli.switch_mac
                        .as_ref()
                        .map(|m| normalize_mac(m))
                        .unwrap_or_else(|| "INTERNET".to_string())
                }
            } else {
                "INTERNET".to_string()
            }
        };

        let depth = if parent_mac == "INTERNET" { 1 } else { 2 };

        let candidate = compute_network_device_lacis_id(
            &router.product_type,
            &mac,
            default_product_code(&router.network_device_type),
        );

        let entry = NodeOrderEntry {
            mac: mac.clone(),
            parent_mac: parent_mac.clone(),
            depth,
            order: 0,
            label: router.display_name.clone(),
            node_type: "router".to_string(),
            ip: Some(router.ip.clone()),
            hostname: None,
            source: "openwrt".to_string(),
            source_ref_id: Some(format!("openwrt:{}:dev:{}", router_id, router.mac)),
            status: router.status.clone(),
            state_type: "trackingOnline".to_string(),
            connection_type: "wired".to_string(),
            lacis_id: router.lacis_id.clone(),
            candidate_lacis_id: Some(candidate),
            product_type: Some(router.product_type.clone()),
            network_device_type: Some(router.network_device_type.clone()),
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({
                "wan_ip": &router.wan_ip,
                "lan_ip": &router.lan_ip,
                "ssid_24g": &router.ssid_24g,
                "ssid_5g": &router.ssid_5g,
                "firmware_version": &router.firmware_version,
                "client_count": router.client_count,
                "uptime_seconds": router.uptime_seconds,
                "router_id": router_id,
            }),
            label_customized: false,
            ssid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        self.mongo.upsert_node_order(&entry).await?;

        // --- Ingest OpenWrt clients ---
        let all_clients = self
            .mongo
            .get_openwrt_clients(None)
            .await
            .unwrap_or_default();
        let clients: Vec<_> = all_clients
            .iter()
            .filter(|c| c.router_id == router_id)
            .collect();

        for (i, cli) in clients.iter().enumerate() {
            let cli_mac = normalize_mac(&cli.mac);

            let entry = NodeOrderEntry {
                mac: cli_mac.clone(),
                parent_mac: mac.clone(),
                depth: depth + 1,
                order: i as u32,
                label: client_label(&cli.hostname, &None, &None, &cli.mac),
                node_type: "client".to_string(),
                ip: Some(cli.ip.clone()),
                hostname: cli.hostname.clone(),
                source: "openwrt".to_string(),
                source_ref_id: Some(format!("openwrt:{}:cli:{}", router_id, cli.mac)),
                status: if cli.active {
                    "active".to_string()
                } else {
                    "inactive".to_string()
                },
                state_type: "trackingOnline".to_string(),
                connection_type: "wired".to_string(),
                lacis_id: cli.lacis_id.clone(),
                candidate_lacis_id: None,
                product_type: None,
                network_device_type: None,
                fid: None,
                facility_name: None,
                metadata: serde_json::json!({ "router_id": router_id }),
                label_customized: false,
                ssid: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_node_order(&entry).await?;
        }

        tracing::debug!(
            "[NodeOrder] OpenWrt router {} ingested: 1 router + {} clients",
            router_id,
            clients.len()
        );
        Ok(())
    }

    /// Ingest external device and its clients into nodeOrder.
    /// Called after ExternalSyncer.poll_device() completes.
    pub async fn ingest_external(&self, device_id: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();

        let devices = self.mongo.list_external_devices().await.unwrap_or_default();
        let dev = devices
            .iter()
            .find(|d| d.device_id == device_id)
            .ok_or_else(|| format!("External device {} not found", device_id))?;

        if dev.mac.is_empty() {
            return Ok(()); // Cannot ingest without MAC
        }

        let mac = normalize_mac(&dev.mac);

        // Determine parent: check if this MAC exists in Omada clients
        let existing = self.mongo.get_node_order_by_mac(&mac).await?;
        let parent_mac = if let Some(ref existing) = existing {
            existing.parent_mac.clone()
        } else {
            let omada_clients = self
                .mongo
                .get_omada_clients(None, None, None)
                .await
                .unwrap_or_default();
            let omada_match = omada_clients.iter().find(|c| normalize_mac(&c.mac) == mac);
            if let Some(cli) = omada_match {
                if cli.wireless {
                    cli.ap_mac
                        .as_ref()
                        .map(|m| normalize_mac(m))
                        .unwrap_or_else(|| "INTERNET".to_string())
                } else {
                    cli.switch_mac
                        .as_ref()
                        .map(|m| normalize_mac(m))
                        .unwrap_or_else(|| "INTERNET".to_string())
                }
            } else {
                "INTERNET".to_string()
            }
        };

        let depth = if parent_mac == "INTERNET" { 1 } else { 2 };

        let candidate = compute_network_device_lacis_id(
            &dev.product_type,
            &mac,
            default_product_code(&dev.network_device_type),
        );

        let entry = NodeOrderEntry {
            mac: mac.clone(),
            parent_mac: parent_mac.clone(),
            depth,
            order: 0,
            label: dev.display_name.clone(),
            node_type: "external".to_string(),
            ip: Some(dev.ip.clone()),
            hostname: None,
            source: "external".to_string(),
            source_ref_id: Some(format!("external:{}:dev:{}", device_id, dev.mac)),
            status: dev.status.clone(),
            state_type: "trackingOnline".to_string(),
            connection_type: "wired".to_string(),
            lacis_id: dev.lacis_id.clone(),
            candidate_lacis_id: Some(candidate),
            product_type: Some(dev.product_type.clone()),
            network_device_type: Some(dev.network_device_type.clone()),
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({
                "protocol": &dev.protocol,
                "device_model": &dev.device_model,
                "client_count": dev.client_count,
                "device_id": device_id,
            }),
            label_customized: false,
            ssid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        self.mongo.upsert_node_order(&entry).await?;

        // --- Ingest external clients ---
        let all_clients = self
            .mongo
            .get_external_clients(None)
            .await
            .unwrap_or_default();
        let clients: Vec<_> = all_clients
            .iter()
            .filter(|c| c.device_id == device_id)
            .collect();

        for (i, cli) in clients.iter().enumerate() {
            let cli_mac = normalize_mac(&cli.mac);

            let entry = NodeOrderEntry {
                mac: cli_mac.clone(),
                parent_mac: mac.clone(),
                depth: depth + 1,
                order: i as u32,
                label: client_label(&cli.hostname, &None, &None, &cli.mac),
                node_type: "client".to_string(),
                ip: cli.ip.clone(),
                hostname: cli.hostname.clone(),
                source: "external".to_string(),
                source_ref_id: Some(format!("external:{}:cli:{}", device_id, cli.mac)),
                status: if cli.active {
                    "active".to_string()
                } else {
                    "inactive".to_string()
                },
                state_type: "trackingOnline".to_string(),
                connection_type: "wired".to_string(),
                lacis_id: cli.lacis_id.clone(),
                candidate_lacis_id: None,
                product_type: None,
                network_device_type: None,
                fid: None,
                facility_name: None,
                metadata: serde_json::json!({ "device_id": device_id }),
                label_customized: false,
                ssid: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_node_order(&entry).await?;
        }

        tracing::debug!(
            "[NodeOrder] External device {} ingested: 1 device + {} clients",
            device_id,
            clients.len()
        );
        Ok(())
    }
}

// ============================================================================
// Migration: Initial population of cg_node_order from existing data
// ============================================================================

/// Migrate existing data to cg_node_order (one-time, runs on startup if collection is empty).
/// Also migrates cg_node_positions and cg_state IDs from old format to MAC format.
pub async fn migrate_to_node_order(mongo: &Arc<MongoDb>) -> Result<(), String> {
    let count = mongo.count_node_order().await?;
    if count > 0 {
        tracing::debug!(
            "[NodeOrder] Migration skipped: cg_node_order already has {} entries",
            count
        );
        return Ok(());
    }

    tracing::info!("[NodeOrder] Starting migration to cg_node_order...");
    let now = chrono::Utc::now().to_rfc3339();

    // ID migration map: old_id → new_mac_id
    let mut id_migration: Vec<(String, String)> = Vec::new();

    // --- 1. Omada Devices ---
    let omada_controllers = mongo.list_omada_controllers().await.unwrap_or_default();
    let omada_devices = mongo
        .get_omada_devices(None, None)
        .await
        .unwrap_or_default();
    let omada_clients = mongo
        .get_omada_clients(None, None, None)
        .await
        .unwrap_or_default();
    let omada_wg_peers = mongo
        .get_omada_wg_peers(None, None)
        .await
        .unwrap_or_default();

    // Find gateway MAC per controller
    let mut gateway_by_ctrl: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for dev in &omada_devices {
        if dev.device_type == "gateway" {
            gateway_by_ctrl.insert(dev.controller_id.clone(), normalize_mac(&dev.mac));
        }
    }

    // Find switch MAC per controller (for AP parent inference)
    let mut switch_by_ctrl: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for dev in &omada_devices {
        if dev.device_type == "switch" {
            switch_by_ctrl
                .entry(dev.controller_id.clone())
                .or_insert_with(|| normalize_mac(&dev.mac));
        }
    }

    let mut order = 0u32;
    for dev in &omada_devices {
        let mac = normalize_mac(&dev.mac);
        let old_id = format!("omada:{}:dev:{}", dev.controller_id, dev.mac);
        id_migration.push((old_id.clone(), mac.clone()));

        let gw = gateway_by_ctrl.get(&dev.controller_id);
        let sw = switch_by_ctrl.get(&dev.controller_id);

        let (parent_mac, depth) = if dev.device_type == "gateway" {
            ("INTERNET".to_string(), 1)
        } else if dev.device_type == "switch" {
            (
                gw.cloned().unwrap_or_else(|| "INTERNET".to_string()),
                if gw.is_some() { 2 } else { 1 },
            )
        } else if dev.device_type == "ap" {
            // AP → parent is switch (PoE), fallback to gateway
            if let Some(sw_mac) = sw {
                (sw_mac.clone(), if gw.is_some() { 3 } else { 2 })
            } else {
                (
                    gw.cloned().unwrap_or_else(|| "INTERNET".to_string()),
                    if gw.is_some() { 2 } else { 1 },
                )
            }
        } else {
            (
                gw.cloned().unwrap_or_else(|| "INTERNET".to_string()),
                if gw.is_some() { 2 } else { 1 },
            )
        };

        let candidate = compute_network_device_lacis_id(
            &dev.product_type,
            &mac,
            default_product_code(&dev.network_device_type),
        );

        let ctrl = omada_controllers
            .iter()
            .find(|c| c.controller_id == dev.controller_id);
        let site = ctrl.and_then(|c| c.sites.iter().find(|s| s.site_id == dev.site_id));
        let fid = site.and_then(|s| s.fid.clone());
        let facility_name = site.and_then(|s| s.fid_display_name.clone());

        let entry = NodeOrderEntry {
            mac: mac.clone(),
            parent_mac,
            depth,
            order,
            label: dev.name.clone(),
            node_type: dev.device_type.clone(),
            ip: dev.ip.clone(),
            hostname: None,
            source: "omada".to_string(),
            source_ref_id: Some(old_id),
            status: if dev.status == 1 {
                "online".to_string()
            } else {
                "offline".to_string()
            },
            state_type: "trackingOnline".to_string(),
            connection_type: "wired".to_string(),
            lacis_id: dev.lacis_id.clone(),
            candidate_lacis_id: Some(candidate),
            product_type: Some(dev.product_type.clone()),
            network_device_type: Some(dev.network_device_type.clone()),
            fid,
            facility_name,
            metadata: serde_json::json!({
                "model": &dev.model,
                "firmware_version": &dev.firmware_version,
                "site_id": &dev.site_id,
                "controller_id": &dev.controller_id,
            }),
            label_customized: false,
            ssid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        mongo.upsert_node_order(&entry).await?;
        order += 1;
    }

    // Build device MAC set for client parent resolution
    let dev_macs: std::collections::HashSet<String> = omada_devices
        .iter()
        .map(|d| normalize_mac(&d.mac))
        .collect();

    // --- 2. Omada Clients ---
    for cli in &omada_clients {
        let mac = normalize_mac(&cli.mac);
        let old_id = format!("omada:{}:cli:{}", cli.controller_id, cli.mac);
        id_migration.push((old_id.clone(), mac.clone()));

        let parent_mac = if cli.wireless {
            cli.ap_mac
                .as_ref()
                .map(|m| normalize_mac(m))
                .filter(|m| dev_macs.contains(m))
        } else {
            cli.switch_mac
                .as_ref()
                .map(|m| normalize_mac(m))
                .filter(|m| dev_macs.contains(m))
        };
        let parent_mac = parent_mac.unwrap_or_else(|| {
            gateway_by_ctrl
                .get(&cli.controller_id)
                .cloned()
                .unwrap_or_else(|| "INTERNET".to_string())
        });

        let parent_depth = if parent_mac == "INTERNET" {
            0
        } else if gateway_by_ctrl.values().any(|gw| gw == &parent_mac) {
            1 // Gateway depth
        } else if switch_by_ctrl.values().any(|sw| sw == &parent_mac) {
            2 // Switch depth
        } else {
            // AP depth: 3 if switch exists in this controller, 2 otherwise
            if switch_by_ctrl.contains_key(&cli.controller_id) {
                3
            } else {
                2
            }
        };
        let conn_type = if cli.wireless { "wireless" } else { "wired" };

        let entry = NodeOrderEntry {
            mac: mac.clone(),
            parent_mac,
            depth: parent_depth + 1,
            order,
            label: client_label(&cli.name, &cli.host_name, &cli.vendor, &cli.mac),
            node_type: "client".to_string(),
            ip: cli.ip.clone(),
            hostname: cli.host_name.clone(),
            source: "omada".to_string(),
            source_ref_id: Some(old_id),
            status: if cli.active {
                "active".to_string()
            } else {
                "inactive".to_string()
            },
            state_type: "trackingOnline".to_string(),
            connection_type: conn_type.to_string(),
            lacis_id: cli.lacis_id.clone(),
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: None,
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({
                "vendor": &cli.vendor,
                "os_name": &cli.os_name,
                "ssid": &cli.ssid,
                "signal_level": &cli.signal_level,
                "traffic_down": cli.traffic_down,
                "traffic_up": cli.traffic_up,
                "uptime": cli.uptime,
            }),
            label_customized: false,
            ssid: cli.ssid.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        mongo.upsert_node_order(&entry).await?;
        order += 1;
    }

    // --- 3. Omada WG Peers ---
    for peer in &omada_wg_peers {
        let pseudo_mac = wg_peer_pseudo_mac(&peer.peer_id);
        let old_id = format!("omada:{}:wg:{}", peer.controller_id, peer.peer_id);
        id_migration.push((old_id.clone(), pseudo_mac.clone()));

        let gw = gateway_by_ctrl.get(&peer.controller_id);
        let parent_mac = gw.cloned().unwrap_or_else(|| "INTERNET".to_string());
        let depth = if parent_mac == "INTERNET" { 1 } else { 2 };

        let entry = NodeOrderEntry {
            mac: pseudo_mac,
            parent_mac,
            depth,
            order,
            label: peer.name.clone(),
            node_type: "wg_peer".to_string(),
            ip: peer.allow_address.first().cloned(),
            hostname: None,
            source: "omada".to_string(),
            source_ref_id: Some(old_id),
            status: if peer.status {
                "active".to_string()
            } else {
                "inactive".to_string()
            },
            state_type: "trackingOnline".to_string(),
            connection_type: "vpn".to_string(),
            lacis_id: None,
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: None,
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({
                "interface_name": &peer.interface_name,
                "public_key": &peer.public_key,
                "allow_address": &peer.allow_address,
                "peer_id": &peer.peer_id,
            }),
            label_customized: false,
            ssid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        mongo.upsert_node_order(&entry).await?;
        order += 1;
    }

    // --- 4. OpenWrt Routers + Clients ---
    let openwrt_routers = mongo.list_openwrt_routers().await.unwrap_or_default();
    let openwrt_clients = mongo.get_openwrt_clients(None).await.unwrap_or_default();

    for router in &openwrt_routers {
        let mac = normalize_mac(&router.mac);
        let old_id = format!("openwrt:{}:dev:{}", router.router_id, router.mac);
        id_migration.push((old_id.clone(), mac.clone()));

        // Check if this MAC already exists (e.g., also an Omada client)
        if mongo.get_node_order_by_mac(&mac).await?.is_some() {
            continue; // Already ingested from Omada, skip duplicate
        }

        // Try to find parent from Omada clients
        let omada_match = omada_clients.iter().find(|c| normalize_mac(&c.mac) == mac);
        let parent_mac = if let Some(cli) = omada_match {
            if cli.wireless {
                cli.ap_mac
                    .as_ref()
                    .map(|m| normalize_mac(m))
                    .filter(|m| dev_macs.contains(m))
            } else {
                cli.switch_mac
                    .as_ref()
                    .map(|m| normalize_mac(m))
                    .filter(|m| dev_macs.contains(m))
            }
            .unwrap_or_else(|| "INTERNET".to_string())
        } else {
            "INTERNET".to_string()
        };
        let depth = if parent_mac == "INTERNET" { 1 } else { 2 };

        let candidate = compute_network_device_lacis_id(
            &router.product_type,
            &mac,
            default_product_code(&router.network_device_type),
        );

        let entry = NodeOrderEntry {
            mac: mac.clone(),
            parent_mac,
            depth,
            order,
            label: router.display_name.clone(),
            node_type: "router".to_string(),
            ip: Some(router.ip.clone()),
            hostname: None,
            source: "openwrt".to_string(),
            source_ref_id: Some(old_id),
            status: router.status.clone(),
            state_type: "trackingOnline".to_string(),
            connection_type: "wired".to_string(),
            lacis_id: router.lacis_id.clone(),
            candidate_lacis_id: Some(candidate),
            product_type: Some(router.product_type.clone()),
            network_device_type: Some(router.network_device_type.clone()),
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({
                "wan_ip": &router.wan_ip,
                "lan_ip": &router.lan_ip,
                "ssid_24g": &router.ssid_24g,
                "ssid_5g": &router.ssid_5g,
                "firmware_version": &router.firmware_version,
                "client_count": router.client_count,
                "uptime_seconds": router.uptime_seconds,
                "router_id": &router.router_id,
            }),
            label_customized: false,
            ssid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        mongo.upsert_node_order(&entry).await?;
        order += 1;
    }

    for cli in &openwrt_clients {
        let mac = normalize_mac(&cli.mac);
        let old_id = format!("openwrt:{}:cli:{}", cli.router_id, cli.mac);
        id_migration.push((old_id.clone(), mac.clone()));

        if mongo.get_node_order_by_mac(&mac).await?.is_some() {
            continue;
        }

        let router = openwrt_routers
            .iter()
            .find(|r| r.router_id == cli.router_id);
        let parent_mac = router
            .map(|r| normalize_mac(&r.mac))
            .unwrap_or_else(|| "INTERNET".to_string());
        let depth = if parent_mac == "INTERNET" { 1 } else { 3 };

        let entry = NodeOrderEntry {
            mac: mac.clone(),
            parent_mac,
            depth,
            order,
            label: client_label(&cli.hostname, &None, &None, &cli.mac),
            node_type: "client".to_string(),
            ip: Some(cli.ip.clone()),
            hostname: cli.hostname.clone(),
            source: "openwrt".to_string(),
            source_ref_id: Some(old_id),
            status: if cli.active {
                "active".to_string()
            } else {
                "inactive".to_string()
            },
            state_type: "trackingOnline".to_string(),
            connection_type: "wired".to_string(),
            lacis_id: cli.lacis_id.clone(),
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: None,
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({ "router_id": &cli.router_id }),
            label_customized: false,
            ssid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        mongo.upsert_node_order(&entry).await?;
        order += 1;
    }

    // --- 5. External Devices + Clients ---
    let external_devices = mongo.list_external_devices().await.unwrap_or_default();
    let external_clients = mongo.get_external_clients(None).await.unwrap_or_default();

    for dev in &external_devices {
        if dev.mac.is_empty() {
            continue;
        }
        let mac = normalize_mac(&dev.mac);
        let old_id = format!("external:{}:dev:{}", dev.device_id, dev.mac);
        id_migration.push((old_id.clone(), mac.clone()));

        if mongo.get_node_order_by_mac(&mac).await?.is_some() {
            continue;
        }

        let omada_match = omada_clients.iter().find(|c| normalize_mac(&c.mac) == mac);
        let parent_mac = if let Some(cli) = omada_match {
            if cli.wireless {
                cli.ap_mac
                    .as_ref()
                    .map(|m| normalize_mac(m))
                    .filter(|m| dev_macs.contains(m))
            } else {
                cli.switch_mac
                    .as_ref()
                    .map(|m| normalize_mac(m))
                    .filter(|m| dev_macs.contains(m))
            }
            .unwrap_or_else(|| "INTERNET".to_string())
        } else {
            "INTERNET".to_string()
        };
        let depth = if parent_mac == "INTERNET" { 1 } else { 2 };

        let candidate = compute_network_device_lacis_id(
            &dev.product_type,
            &mac,
            default_product_code(&dev.network_device_type),
        );

        let entry = NodeOrderEntry {
            mac: mac.clone(),
            parent_mac,
            depth,
            order,
            label: dev.display_name.clone(),
            node_type: "external".to_string(),
            ip: Some(dev.ip.clone()),
            hostname: None,
            source: "external".to_string(),
            source_ref_id: Some(old_id),
            status: dev.status.clone(),
            state_type: "trackingOnline".to_string(),
            connection_type: "wired".to_string(),
            lacis_id: dev.lacis_id.clone(),
            candidate_lacis_id: Some(candidate),
            product_type: Some(dev.product_type.clone()),
            network_device_type: Some(dev.network_device_type.clone()),
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({
                "protocol": &dev.protocol,
                "device_model": &dev.device_model,
                "client_count": dev.client_count,
                "device_id": &dev.device_id,
            }),
            label_customized: false,
            ssid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        mongo.upsert_node_order(&entry).await?;
        order += 1;
    }

    for cli in &external_clients {
        let mac = normalize_mac(&cli.mac);
        let old_id = format!("external:{}:cli:{}", cli.device_id, cli.mac);
        id_migration.push((old_id.clone(), mac.clone()));

        if mongo.get_node_order_by_mac(&mac).await?.is_some() {
            continue;
        }

        let dev = external_devices
            .iter()
            .find(|d| d.device_id == cli.device_id);
        let parent_mac = dev
            .map(|d| normalize_mac(&d.mac))
            .unwrap_or_else(|| "INTERNET".to_string());
        let depth = if parent_mac == "INTERNET" { 1 } else { 3 };

        let entry = NodeOrderEntry {
            mac: mac.clone(),
            parent_mac,
            depth,
            order,
            label: client_label(&cli.hostname, &None, &None, &cli.mac),
            node_type: "client".to_string(),
            ip: cli.ip.clone(),
            hostname: cli.hostname.clone(),
            source: "external".to_string(),
            source_ref_id: Some(old_id),
            status: if cli.active {
                "active".to_string()
            } else {
                "inactive".to_string()
            },
            state_type: "trackingOnline".to_string(),
            connection_type: "wired".to_string(),
            lacis_id: cli.lacis_id.clone(),
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: None,
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({ "device_id": &cli.device_id }),
            label_customized: false,
            ssid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        mongo.upsert_node_order(&entry).await?;
        order += 1;
    }

    // --- 6. Logic Devices ---
    let logic_devices = mongo.list_logic_devices().await.unwrap_or_default();
    for dev in &logic_devices {
        // Extract UUID from "logic:{uuid}" format
        let uuid_part = dev.id.strip_prefix("logic:").unwrap_or(&dev.id);
        let pseudo_mac = logic_device_pseudo_mac(uuid_part);
        id_migration.push((dev.id.clone(), pseudo_mac.clone()));

        // Resolve parent: logic devices have old-format parent_id
        let parent_mac = if let Some(ref pid) = dev.parent_id {
            // Try to find the new MAC for this old parent_id
            id_migration
                .iter()
                .find(|(old, _)| old == pid)
                .map(|(_, new)| new.clone())
                .unwrap_or_else(|| "INTERNET".to_string())
        } else {
            "INTERNET".to_string()
        };

        let depth = if parent_mac == "INTERNET" { 1 } else { 3 };

        let entry = NodeOrderEntry {
            mac: pseudo_mac,
            parent_mac,
            depth,
            order,
            label: dev.label.clone(),
            node_type: "logic_device".to_string(),
            ip: dev.ip.clone(),
            hostname: None,
            source: "manual".to_string(),
            source_ref_id: Some(dev.id.clone()),
            status: "manual".to_string(),
            state_type: "manual".to_string(),
            connection_type: "wired".to_string(),
            lacis_id: dev.lacis_id.clone(),
            candidate_lacis_id: None,
            product_type: None,
            network_device_type: Some(dev.device_type.clone()),
            fid: None,
            facility_name: None,
            metadata: serde_json::json!({
                "location": &dev.location,
                "note": &dev.note,
                "logic_device_id": &dev.id,
            }),
            label_customized: false,
            ssid: None,
            created_at: dev.created_at.clone(),
            updated_at: now.clone(),
        };
        mongo.upsert_node_order(&entry).await?;
        order += 1;
    }

    // --- 7. Migrate custom labels into nodeOrder ---
    let custom_labels = mongo.get_all_custom_labels().await.unwrap_or_default();
    for (old_node_id, label) in &custom_labels {
        // Find the new MAC for this old node_id
        if let Some((_, new_mac)) = id_migration.iter().find(|(old, _)| old == old_node_id) {
            let _ = mongo.update_node_order_label(new_mac, label, true).await;
        }
    }

    // --- 8. Migrate cg_node_positions and cg_state IDs ---
    // Also add controller old IDs → skip (controllers are not in nodeOrder)
    for (old_id, new_mac) in &id_migration {
        if old_id != new_mac {
            let _ = mongo.migrate_node_position_id(old_id, new_mac).await;
            let _ = mongo.migrate_collapsed_node_id(old_id, new_mac).await;
        }
    }

    let final_count = mongo.count_node_order().await?;
    tracing::info!(
        "[NodeOrder] Migration complete: {} entries created, {} IDs migrated",
        final_count,
        id_migration.len()
    );

    Ok(())
}
