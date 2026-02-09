//! UserObjectDetail Ingester — Ingests data from all sources into user_object_detail SSoT
//!
//! userObjectDetail absolute rules:
//! 1. user_object_detail = 唯一のSSoT。user_object_detailに存在 = 描画対象
//! 2. 全ノードは完全に等価。managed/detected/pendingの区別禁止
//! 3. ネットワーク構造: INTERNET → Gateway → Children → ...
//! 4. Gateway不在 = ネットワーク障害。孤児ノードはGatewayにフォールバック (INTERNET直結禁止)
//! 5. インフラデバイスの_idはLacisID (prefix=4)、クライアントの_idはMAC
//! 6. Controllerは管理ソフトウェア、物理トポロジーには含めない
//!
//! _id rules:
//! - Gateway/Switch/AP/Router/External: LacisID = 4{productType}{MAC}{productCode} (20桁)
//! - Client: MAC (12桁大文字HEX)
//! - WG Peer: Pseudo-MAC F0{10chars} (12桁)
//! - Logic Device: Pseudo-MAC F2{10chars} (12桁)
//!
//! upsert rules:
//! - New _id: insert all fields
//! - Existing _id: update volatile fields ONLY (state_type, ip, hostname, metadata, updated_at)
//!   parent_id, sort_order, label(if customized) are NEVER overwritten

use std::sync::Arc;

use crate::db::mongo::user_object_detail::UserObjectDetail;
use crate::db::mongo::MongoDb;
use crate::db::mysql::MySqlDb;
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

/// Compute the _id for an infrastructure network device.
/// Returns (document_id, lacis_id_candidate)
pub fn compute_infra_id(product_type: &str, mac: &str, network_device_type: &str) -> String {
    compute_network_device_lacis_id(product_type, mac, default_product_code(network_device_type))
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

/// Map syncer status strings to state_type
fn map_state_type(status: &str) -> String {
    match status {
        "online" | "active" => "online".to_string(),
        "offline" | "inactive" => "offline".to_string(),
        "manual" => "StaticOnline".to_string(),
        _ => "offline".to_string(),
    }
}

/// UserObjectIngester: reads source collections and upserts into user_object_detail
pub struct UserObjectIngester {
    mongo: Arc<MongoDb>,
    mysql: Arc<MySqlDb>,
}

impl UserObjectIngester {
    pub fn new(mongo: Arc<MongoDb>, mysql: Arc<MySqlDb>) -> Self {
        Self { mongo, mysql }
    }

    /// Record state change if state_type differs from existing
    async fn check_and_record_state_change(
        &self,
        id: &str,
        new_state: &str,
        existing: &Option<UserObjectDetail>,
    ) {
        if let Some(ref ex) = existing {
            if ex.state_type != new_state
                && !ex.state_type.starts_with("Static") // Don't override admin manual overrides
            {
                let _ = self
                    .mysql
                    .insert_device_state_change(id, new_state, Some(&ex.state_type), "syncer")
                    .await;
            }
        }
    }

    /// Helper to resolve parent_id for infra device.
    /// Returns (parent_id, gateway_id for this controller)
    fn resolve_infra_parent(
        &self,
        is_gateway: bool,
        gateway_lacis_id: &Option<String>,
    ) -> String {
        if is_gateway {
            "INTERNET".to_string()
        } else {
            gateway_lacis_id
                .clone()
                .unwrap_or_else(|| "INTERNET".to_string())
        }
    }

    /// Ingest all Omada data for a specific controller into user_object_detail.
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

        // Find gateway LacisID for this controller (used as default parent)
        let gateway_lacis_id = devices.iter().find(|d| d.device_type == "gateway").map(
            |d| {
                let mac = normalize_mac(&d.mac);
                compute_infra_id(&d.product_type, &mac, &d.network_device_type)
            },
        );

        // Build device MAC → LacisID lookup for client parent resolution
        let dev_mac_to_lacis_id: std::collections::HashMap<String, String> = devices
            .iter()
            .map(|d| {
                let mac = normalize_mac(&d.mac);
                let lacis_id =
                    compute_infra_id(&d.product_type, &mac, &d.network_device_type);
                (mac, lacis_id)
            })
            .collect();

        // --- Ingest devices (gateway, switch, ap) ---
        // Controller itself is NOT ingested (rule 6: Controller = management software)
        let mut order_counter = 0u32;
        for dev in &devices {
            let mac = normalize_mac(&dev.mac);
            let doc_id = compute_infra_id(&dev.product_type, &mac, &dev.network_device_type);

            let parent_id =
                self.resolve_infra_parent(dev.device_type == "gateway", &gateway_lacis_id);

            // Resolve fid from controller sites
            let site = ctrl.and_then(|c| c.sites.iter().find(|s| s.site_id == dev.site_id));
            let fid = site.and_then(|s| s.fid.clone());
            let facility_name = site.and_then(|s| s.fid_display_name.clone());

            let state_type = if dev.status == 1 { "online" } else { "offline" };

            // Check for state change
            let existing = self.mongo.get_user_object_detail_by_id(&doc_id).await.ok().flatten();
            self.check_and_record_state_change(&doc_id, state_type, &existing).await;

            let entry = UserObjectDetail {
                id: doc_id.clone(),
                mac: mac.clone(),
                lacis_id: dev.lacis_id.clone(),
                device_type: "NetworkDevice".to_string(),
                parent_id,
                sort_order: existing.as_ref().map(|e| e.sort_order).unwrap_or(order_counter),
                node_type: dev.device_type.clone(),
                state_type: state_type.to_string(),
                label: dev.name.clone(),
                label_customized: existing.as_ref().map(|e| e.label_customized).unwrap_or(false),
                ip: dev.ip.clone(),
                hostname: None,
                source: "omada".to_string(),
                source_ref_id: Some(format!("omada:{}:dev:{}", controller_id, dev.mac)),
                connection_type: "wired".to_string(),
                product_type: Some(dev.product_type.clone()),
                product_code: Some(default_product_code(&dev.network_device_type).to_string()),
                network_device_type: Some(dev.network_device_type.clone()),
                candidate_lacis_id: Some(doc_id.clone()),
                fid,
                facility_name,
                ssid: None,
                metadata: serde_json::json!({
                    "model": &dev.model,
                    "firmware_version": &dev.firmware_version,
                    "site_id": &dev.site_id,
                    "controller_id": controller_id,
                }),
                aranea_lacis_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_user_object_detail(&entry).await?;
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

            // Determine parent LacisID
            let parent_lacis_id = if cli.wireless {
                cli.ap_mac
                    .as_ref()
                    .map(|m| normalize_mac(m))
                    .and_then(|m| dev_mac_to_lacis_id.get(&m).cloned())
            } else {
                cli.switch_mac
                    .as_ref()
                    .map(|m| normalize_mac(m))
                    .and_then(|m| dev_mac_to_lacis_id.get(&m).cloned())
            };

            let parent_id = parent_lacis_id.unwrap_or_else(|| {
                gateway_lacis_id
                    .clone()
                    .unwrap_or_else(|| "INTERNET".to_string())
            });

            let conn_type = if cli.wireless { "wireless" } else { "wired" };
            let state_type = map_state_type(if cli.active { "active" } else { "inactive" });

            // Client _id = MAC
            let existing = self.mongo.get_user_object_detail_by_id(&mac).await.ok().flatten();
            self.check_and_record_state_change(&mac, &state_type, &existing).await;

            let entry = UserObjectDetail {
                id: mac.clone(),
                mac: mac.clone(),
                lacis_id: cli.lacis_id.clone(),
                device_type: "NetworkDevice".to_string(),
                parent_id,
                sort_order: existing.as_ref().map(|e| e.sort_order).unwrap_or(order_counter),
                node_type: "client".to_string(),
                state_type,
                label: client_label(&cli.name, &cli.host_name, &cli.vendor, &cli.mac),
                label_customized: existing.as_ref().map(|e| e.label_customized).unwrap_or(false),
                ip: cli.ip.clone(),
                hostname: cli.host_name.clone(),
                source: "omada".to_string(),
                source_ref_id: Some(format!("omada:{}:cli:{}", controller_id, cli.mac)),
                connection_type: conn_type.to_string(),
                product_type: None,
                product_code: None,
                network_device_type: None,
                candidate_lacis_id: None,
                fid: None,
                facility_name: None,
                ssid: cli.ssid.clone(),
                metadata: serde_json::json!({
                    "vendor": &cli.vendor,
                    "os_name": &cli.os_name,
                    "ssid": &cli.ssid,
                    "signal_level": &cli.signal_level,
                    "traffic_down": cli.traffic_down,
                    "traffic_up": cli.traffic_up,
                    "uptime": cli.uptime,
                }),
                aranea_lacis_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_user_object_detail(&entry).await?;
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
            let parent_id = gateway_lacis_id
                .clone()
                .unwrap_or_else(|| "INTERNET".to_string());

            let state_type = map_state_type(if peer.status { "active" } else { "inactive" });

            let existing = self.mongo.get_user_object_detail_by_id(&pseudo_mac).await.ok().flatten();
            self.check_and_record_state_change(&pseudo_mac, &state_type, &existing).await;

            let entry = UserObjectDetail {
                id: pseudo_mac.clone(),
                mac: pseudo_mac.clone(),
                lacis_id: None,
                device_type: "NetworkDevice".to_string(),
                parent_id,
                sort_order: existing.as_ref().map(|e| e.sort_order).unwrap_or(order_counter),
                node_type: "wg_peer".to_string(),
                state_type,
                label: peer.name.clone(),
                label_customized: existing.as_ref().map(|e| e.label_customized).unwrap_or(false),
                ip: peer.allow_address.first().cloned(),
                hostname: None,
                source: "omada".to_string(),
                source_ref_id: Some(format!("omada:{}:wg:{}", controller_id, peer.peer_id)),
                connection_type: "vpn".to_string(),
                product_type: None,
                product_code: None,
                network_device_type: None,
                candidate_lacis_id: None,
                fid: None,
                facility_name: None,
                ssid: None,
                metadata: serde_json::json!({
                    "interface_name": &peer.interface_name,
                    "public_key": &peer.public_key,
                    "allow_address": &peer.allow_address,
                    "peer_id": &peer.peer_id,
                }),
                aranea_lacis_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_user_object_detail(&entry).await?;
            order_counter += 1;
        }

        tracing::debug!(
            "[UserObjectIngester] Omada controller {} ingested: {} entries",
            controller_id,
            order_counter
        );
        Ok(())
    }

    /// Ingest OpenWrt router and its clients into user_object_detail.
    /// Called after OpenWrtSyncer.poll_router() completes.
    pub async fn ingest_openwrt(&self, router_id: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();

        let routers = self.mongo.list_openwrt_routers().await.unwrap_or_default();
        let router = routers
            .iter()
            .find(|r| r.router_id == router_id)
            .ok_or_else(|| format!("OpenWrt router {} not found", router_id))?;

        let mac = normalize_mac(&router.mac);
        let doc_id = compute_infra_id(&router.product_type, &mac, &router.network_device_type);

        // Determine parent: check if this _id already exists
        let existing = self.mongo.get_user_object_detail_by_id(&doc_id).await.ok().flatten();
        let parent_id = if let Some(ref existing) = existing {
            // Preserve existing parent (may have been reparented manually)
            existing.parent_id.clone()
        } else {
            // New: try to find parent from Omada device MAC→LacisID mapping
            let omada_clients = self
                .mongo
                .get_omada_clients(None, None, None)
                .await
                .unwrap_or_default();
            let omada_match = omada_clients.iter().find(|c| normalize_mac(&c.mac) == mac);
            if let Some(cli) = omada_match {
                // Router appears as Omada client → find parent device LacisID
                let parent_mac = if cli.wireless {
                    cli.ap_mac.as_ref().map(|m| normalize_mac(m))
                } else {
                    cli.switch_mac.as_ref().map(|m| normalize_mac(m))
                };
                if let Some(pmac) = parent_mac {
                    // Look up parent's LacisID from omada_devices
                    let all_devices = self.mongo.get_omada_devices(None, None).await.unwrap_or_default();
                    all_devices
                        .iter()
                        .find(|d| normalize_mac(&d.mac) == pmac)
                        .map(|d| compute_infra_id(&d.product_type, &pmac, &d.network_device_type))
                        .unwrap_or_else(|| "INTERNET".to_string())
                } else {
                    "INTERNET".to_string()
                }
            } else {
                "INTERNET".to_string()
            }
        };

        let state_type = map_state_type(&router.status);
        self.check_and_record_state_change(&doc_id, &state_type, &existing).await;

        let entry = UserObjectDetail {
            id: doc_id.clone(),
            mac: mac.clone(),
            lacis_id: router.lacis_id.clone(),
            device_type: "NetworkDevice".to_string(),
            parent_id: parent_id.clone(),
            sort_order: existing.as_ref().map(|e| e.sort_order).unwrap_or(0),
            node_type: "router".to_string(),
            state_type,
            label: router.display_name.clone(),
            label_customized: existing.as_ref().map(|e| e.label_customized).unwrap_or(false),
            ip: Some(router.ip.clone()),
            hostname: None,
            source: "openwrt".to_string(),
            source_ref_id: Some(format!("openwrt:{}:dev:{}", router_id, router.mac)),
            connection_type: "wired".to_string(),
            product_type: Some(router.product_type.clone()),
            product_code: Some(default_product_code(&router.network_device_type).to_string()),
            network_device_type: Some(router.network_device_type.clone()),
            candidate_lacis_id: Some(doc_id.clone()),
            fid: None,
            facility_name: None,
            ssid: None,
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
            aranea_lacis_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        self.mongo.upsert_user_object_detail(&entry).await?;

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
            let state_type = map_state_type(if cli.active { "active" } else { "inactive" });

            let cli_existing = self.mongo.get_user_object_detail_by_id(&cli_mac).await.ok().flatten();
            self.check_and_record_state_change(&cli_mac, &state_type, &cli_existing).await;

            let entry = UserObjectDetail {
                id: cli_mac.clone(),
                mac: cli_mac.clone(),
                lacis_id: cli.lacis_id.clone(),
                device_type: "NetworkDevice".to_string(),
                parent_id: doc_id.clone(),
                sort_order: cli_existing.as_ref().map(|e| e.sort_order).unwrap_or(i as u32),
                node_type: "client".to_string(),
                state_type,
                label: client_label(&cli.hostname, &None, &None, &cli.mac),
                label_customized: cli_existing.as_ref().map(|e| e.label_customized).unwrap_or(false),
                ip: Some(cli.ip.clone()),
                hostname: cli.hostname.clone(),
                source: "openwrt".to_string(),
                source_ref_id: Some(format!("openwrt:{}:cli:{}", router_id, cli.mac)),
                connection_type: "wired".to_string(),
                product_type: None,
                product_code: None,
                network_device_type: None,
                candidate_lacis_id: None,
                fid: None,
                facility_name: None,
                ssid: None,
                metadata: serde_json::json!({ "router_id": router_id }),
                aranea_lacis_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_user_object_detail(&entry).await?;
        }

        tracing::debug!(
            "[UserObjectIngester] OpenWrt router {} ingested: 1 router + {} clients",
            router_id,
            clients.len()
        );
        Ok(())
    }

    /// Ingest external device and its clients into user_object_detail.
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
        let doc_id = compute_infra_id(&dev.product_type, &mac, &dev.network_device_type);

        // Determine parent
        let existing = self.mongo.get_user_object_detail_by_id(&doc_id).await.ok().flatten();
        let parent_id = if let Some(ref existing) = existing {
            existing.parent_id.clone()
        } else {
            let omada_clients = self
                .mongo
                .get_omada_clients(None, None, None)
                .await
                .unwrap_or_default();
            let omada_match = omada_clients.iter().find(|c| normalize_mac(&c.mac) == mac);
            if let Some(cli) = omada_match {
                let parent_mac = if cli.wireless {
                    cli.ap_mac.as_ref().map(|m| normalize_mac(m))
                } else {
                    cli.switch_mac.as_ref().map(|m| normalize_mac(m))
                };
                if let Some(pmac) = parent_mac {
                    let all_devices = self.mongo.get_omada_devices(None, None).await.unwrap_or_default();
                    all_devices
                        .iter()
                        .find(|d| normalize_mac(&d.mac) == pmac)
                        .map(|d| compute_infra_id(&d.product_type, &pmac, &d.network_device_type))
                        .unwrap_or_else(|| "INTERNET".to_string())
                } else {
                    "INTERNET".to_string()
                }
            } else {
                "INTERNET".to_string()
            }
        };

        let state_type = map_state_type(&dev.status);
        self.check_and_record_state_change(&doc_id, &state_type, &existing).await;

        let entry = UserObjectDetail {
            id: doc_id.clone(),
            mac: mac.clone(),
            lacis_id: dev.lacis_id.clone(),
            device_type: "NetworkDevice".to_string(),
            parent_id: parent_id.clone(),
            sort_order: existing.as_ref().map(|e| e.sort_order).unwrap_or(0),
            node_type: "external".to_string(),
            state_type,
            label: dev.display_name.clone(),
            label_customized: existing.as_ref().map(|e| e.label_customized).unwrap_or(false),
            ip: Some(dev.ip.clone()),
            hostname: None,
            source: "external".to_string(),
            source_ref_id: Some(format!("external:{}:dev:{}", device_id, dev.mac)),
            connection_type: "wired".to_string(),
            product_type: Some(dev.product_type.clone()),
            product_code: Some(default_product_code(&dev.network_device_type).to_string()),
            network_device_type: Some(dev.network_device_type.clone()),
            candidate_lacis_id: Some(doc_id.clone()),
            fid: None,
            facility_name: None,
            ssid: None,
            metadata: serde_json::json!({
                "protocol": &dev.protocol,
                "device_model": &dev.device_model,
                "client_count": dev.client_count,
                "device_id": device_id,
            }),
            aranea_lacis_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        self.mongo.upsert_user_object_detail(&entry).await?;

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
            let state_type = map_state_type(if cli.active { "active" } else { "inactive" });

            let cli_existing = self.mongo.get_user_object_detail_by_id(&cli_mac).await.ok().flatten();
            self.check_and_record_state_change(&cli_mac, &state_type, &cli_existing).await;

            let entry = UserObjectDetail {
                id: cli_mac.clone(),
                mac: cli_mac.clone(),
                lacis_id: cli.lacis_id.clone(),
                device_type: "NetworkDevice".to_string(),
                parent_id: doc_id.clone(),
                sort_order: cli_existing.as_ref().map(|e| e.sort_order).unwrap_or(i as u32),
                node_type: "client".to_string(),
                state_type,
                label: client_label(&cli.hostname, &None, &None, &cli.mac),
                label_customized: cli_existing.as_ref().map(|e| e.label_customized).unwrap_or(false),
                ip: cli.ip.clone(),
                hostname: cli.hostname.clone(),
                source: "external".to_string(),
                source_ref_id: Some(format!("external:{}:cli:{}", device_id, cli.mac)),
                connection_type: "wired".to_string(),
                product_type: None,
                product_code: None,
                network_device_type: None,
                candidate_lacis_id: None,
                fid: None,
                facility_name: None,
                ssid: None,
                metadata: serde_json::json!({ "device_id": device_id }),
                aranea_lacis_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            self.mongo.upsert_user_object_detail(&entry).await?;
        }

        tracing::debug!(
            "[UserObjectIngester] External device {} ingested: 1 device + {} clients",
            device_id,
            clients.len()
        );
        Ok(())
    }
}

// ============================================================================
// Migration: cg_node_order → user_object_detail (one-time, startup)
// ============================================================================

/// Migrate cg_node_order to user_object_detail (one-time, runs on startup if user_object_detail is empty).
/// Also migrates cg_state collapsed_node_ids from MAC to LacisID/MAC format.
pub async fn migrate_to_user_object_detail(mongo: &Arc<MongoDb>) -> Result<(), String> {
    let count = mongo.count_user_object_details().await?;
    if count > 0 {
        tracing::debug!(
            "[UserObjectDetail] Migration skipped: user_object_detail already has {} entries",
            count
        );
        return Ok(());
    }

    // Check if cg_node_order has data to migrate
    let node_order_count = mongo.count_node_order().await?;
    if node_order_count == 0 {
        tracing::debug!("[UserObjectDetail] Migration skipped: cg_node_order is also empty");
        return Ok(());
    }

    tracing::info!(
        "[UserObjectDetail] Starting migration from cg_node_order ({} entries)...",
        node_order_count
    );
    let now = chrono::Utc::now().to_rfc3339();

    let entries = mongo.get_all_node_order().await?;

    // Phase 1: Build MAC → LacisID mapping for infrastructure devices
    let mut mac_to_lacis_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for entry in &entries {
        let is_infra = matches!(
            entry.node_type.as_str(),
            "gateway" | "switch" | "ap" | "router" | "external" | "lpg_server"
        );
        if is_infra {
            if let (Some(ref pt), Some(ref ndt)) = (&entry.product_type, &entry.network_device_type)
            {
                let lacis_id = compute_infra_id(pt, &entry.mac, ndt);
                mac_to_lacis_id.insert(entry.mac.clone(), lacis_id);
            }
        }
    }

    // Phase 2: Migrate each entry
    let mut migrated = 0u32;
    // Old MAC → new _id mapping (for collapsed state migration)
    let mut id_migration: Vec<(String, String)> = Vec::new();

    for entry in &entries {
        let is_infra = matches!(
            entry.node_type.as_str(),
            "gateway" | "switch" | "ap" | "router" | "external" | "lpg_server"
        );

        // Compute _id
        let doc_id = if is_infra {
            mac_to_lacis_id
                .get(&entry.mac)
                .cloned()
                .unwrap_or_else(|| entry.mac.clone())
        } else {
            // Client, WG Peer, Logic Device: _id = MAC (or pseudo-MAC)
            entry.mac.clone()
        };

        // Track old MAC → new _id for migration
        if entry.mac != doc_id {
            id_migration.push((entry.mac.clone(), doc_id.clone()));
        }

        // Resolve parent_id: convert parent_mac to parent LacisID
        let parent_id = if entry.parent_mac == "INTERNET" {
            "INTERNET".to_string()
        } else {
            // Try to find parent's LacisID
            mac_to_lacis_id
                .get(&entry.parent_mac)
                .cloned()
                .unwrap_or_else(|| {
                    // Parent is not infra → use MAC as-is (shouldn't happen for valid topology)
                    // Or parent_mac references something we don't have a LacisID for
                    entry.parent_mac.clone()
                })
        };

        let state_type = map_state_type(&entry.status);

        let detail = UserObjectDetail {
            id: doc_id.clone(),
            mac: entry.mac.clone(),
            lacis_id: entry.lacis_id.clone(),
            device_type: "NetworkDevice".to_string(),
            parent_id,
            sort_order: entry.order,
            node_type: entry.node_type.clone(),
            state_type,
            label: entry.label.clone(),
            label_customized: entry.label_customized,
            ip: entry.ip.clone(),
            hostname: entry.hostname.clone(),
            source: entry.source.clone(),
            source_ref_id: entry.source_ref_id.clone(),
            connection_type: entry.connection_type.clone(),
            product_type: entry.product_type.clone(),
            product_code: Some(
                entry
                    .network_device_type
                    .as_ref()
                    .map(|ndt| default_product_code(ndt).to_string())
                    .unwrap_or_else(|| "0000".to_string()),
            ),
            network_device_type: entry.network_device_type.clone(),
            candidate_lacis_id: entry.candidate_lacis_id.clone(),
            fid: entry.fid.clone(),
            facility_name: entry.facility_name.clone(),
            ssid: entry.ssid.clone(),
            metadata: entry.metadata.clone(),
            aranea_lacis_id: None,
            created_at: entry.created_at.clone(),
            updated_at: now.clone(),
        };

        mongo.upsert_user_object_detail(&detail).await?;
        migrated += 1;
    }

    // Phase 3: Migrate cg_state collapsed_node_ids (MAC → LacisID)
    for (old_mac, new_id) in &id_migration {
        let _ = mongo.migrate_collapsed_node_id(old_mac, new_id).await;
    }

    let final_count = mongo.count_user_object_details().await?;
    tracing::info!(
        "[UserObjectDetail] Migration complete: {} entries created (from {} cg_node_order entries), {} IDs migrated",
        final_count,
        node_order_count,
        id_migration.len()
    );

    Ok(())
}
