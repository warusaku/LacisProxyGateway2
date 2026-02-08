//! Omada data persistence layer (MongoDB)
//!
//! 4 collections:
//! - `omada_controllers`: Controller registration + auth credentials
//! - `omada_devices`: Network infrastructure devices (gateway, switch, AP)
//! - `omada_clients`: Connected client endpoints
//! - `omada_wg_peers`: WireGuard peers

use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::{self, doc};
use mongodb::options::{FindOptions, UpdateOptions};
use serde::{Deserialize, Serialize};

use super::MongoDb;
use crate::omada::client::{
    device_type_to_network_device_type, device_type_to_product_type, normalize_mac,
    OmadaClientDevice, OmadaDevice, WireGuardPeer,
};

// ============================================================================
// Document types
// ============================================================================

/// Controller registration document (omada_controllers collection)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaControllerDoc {
    pub controller_id: String,
    pub display_name: String,
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub omadac_id: String,
    pub controller_ver: String,
    pub api_ver: String,
    /// "connected" | "error" | "disconnected"
    pub status: String,
    pub last_error: Option<String>,
    pub sites: Vec<OmadaSiteMapping>,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Site mapping within a controller (for CelestialGlobe future integration)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaSiteMapping {
    pub site_id: String,
    pub name: String,
    pub region: Option<String>,
    /// Facility ID (future CelestialGlobe linkage)
    pub fid: Option<String>,
    /// Tenant ID (future mobes2.0 linkage)
    pub tid: Option<String>,
    pub fid_display_name: Option<String>,
}

/// Network infrastructure device document (omada_devices collection)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaDeviceDoc {
    /// Normalized MAC address (uppercase, no separators)
    pub mac: String,
    pub controller_id: String,
    pub site_id: String,
    pub name: String,
    /// Omada device type: "gateway" | "switch" | "ap"
    pub device_type: String,
    pub model: Option<String>,
    pub ip: Option<String>,
    /// 0=offline, 1=online
    pub status: i32,
    pub firmware_version: Option<String>,
    // mobes2.0 compatible fields
    /// Reserved for future mobes2.0 integration (null until assigned by lacisIdService)
    pub lacis_id: Option<String>,
    /// mobes2.0 ProductType code: "101"=Router, "102"=Switch, "103"=AP, "191"=Unknown
    pub product_type: String,
    /// mobes2.0 NetworkDeviceType: "Router" | "Switch" | "AccessPoint" | "Unknown"
    pub network_device_type: String,
    pub synced_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Connected client document (omada_clients collection)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaClientDoc {
    /// Normalized MAC address (uppercase, no separators)
    pub mac: String,
    pub controller_id: String,
    pub site_id: String,
    pub name: Option<String>,
    pub host_name: Option<String>,
    pub ip: Option<String>,
    pub ipv6_list: Vec<String>,
    pub vendor: Option<String>,
    pub device_type: Option<String>,
    pub device_category: Option<String>,
    pub os_name: Option<String>,
    pub model: Option<String>,
    pub connect_type: Option<i32>,
    pub wireless: bool,
    pub ssid: Option<String>,
    pub signal_level: Option<i32>,
    pub rssi: Option<i32>,
    pub ap_mac: Option<String>,
    pub ap_name: Option<String>,
    pub wifi_mode: Option<i32>,
    pub channel: Option<i32>,
    pub switch_mac: Option<String>,
    pub switch_name: Option<String>,
    pub port: Option<i32>,
    pub vid: Option<i32>,
    pub traffic_down: i64,
    pub traffic_up: i64,
    pub uptime: i64,
    pub active: bool,
    pub blocked: bool,
    pub guest: bool,
    /// Reserved for future mobes2.0 integration
    pub lacis_id: Option<String>,
    pub last_seen_at: Option<String>,
    pub synced_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// WireGuard peer document (omada_wg_peers collection)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaWgPeerDoc {
    pub peer_id: String,
    pub controller_id: String,
    pub site_id: String,
    pub name: String,
    pub status: bool,
    pub interface_id: String,
    pub interface_name: String,
    pub public_key: String,
    pub allow_address: Vec<String>,
    pub keep_alive: i32,
    pub comment: Option<String>,
    pub synced_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Aggregated summary across all controllers
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaSummaryDoc {
    pub total_controllers: u64,
    pub connected_controllers: u64,
    pub total_devices: u64,
    pub online_devices: u64,
    pub total_clients: u64,
    pub active_clients: u64,
    pub total_wg_peers: u64,
    pub active_wg_peers: u64,
}

// ============================================================================
// Controller CRUD
// ============================================================================

impl MongoDb {
    /// Upsert a controller document (key: controller_id)
    pub async fn upsert_omada_controller(
        &self,
        doc: &OmadaControllerDoc,
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("omada_controllers");

        let filter = doc! { "controller_id": &doc.controller_id };
        let bson_doc =
            bson::to_document(doc).map_err(|e| format!("Serialize controller: {}", e))?;

        let update = doc! {
            "$set": bson_doc,
            "$setOnInsert": {
                "created_at": &doc.created_at,
            }
        };

        let options = UpdateOptions::builder().upsert(true).build();
        collection
            .update_one(filter, update, Some(options))
            .await
            .map_err(|e| format!("Upsert controller: {}", e))?;

        Ok(())
    }

    /// List all registered controllers
    pub async fn list_omada_controllers(&self) -> Result<Vec<OmadaControllerDoc>, String> {
        let collection = self.db.collection::<bson::Document>("omada_controllers");

        let options = FindOptions::builder()
            .sort(doc! { "display_name": 1 })
            .build();

        let mut cursor = collection
            .find(doc! {}, Some(options))
            .await
            .map_err(|e| format!("List controllers: {}", e))?;

        let mut controllers = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("Cursor controllers: {}", e))?
        {
            if let Ok(ctrl) = bson::from_document(doc) {
                controllers.push(ctrl);
            }
        }

        Ok(controllers)
    }

    /// Get a single controller by controller_id
    pub async fn get_omada_controller(
        &self,
        controller_id: &str,
    ) -> Result<Option<OmadaControllerDoc>, String> {
        let collection = self.db.collection::<bson::Document>("omada_controllers");

        let doc = collection
            .find_one(doc! { "controller_id": controller_id }, None)
            .await
            .map_err(|e| format!("Get controller: {}", e))?;

        match doc {
            Some(d) => {
                let ctrl = bson::from_document(d)
                    .map_err(|e| format!("Deserialize controller: {}", e))?;
                Ok(Some(ctrl))
            }
            None => Ok(None),
        }
    }

    /// Delete a controller and all its associated data
    pub async fn delete_omada_controller(&self, controller_id: &str) -> Result<(), String> {
        let filter = doc! { "controller_id": controller_id };

        // Delete from all 4 collections
        self.db
            .collection::<bson::Document>("omada_controllers")
            .delete_one(filter.clone(), None)
            .await
            .map_err(|e| format!("Delete controller: {}", e))?;

        self.db
            .collection::<bson::Document>("omada_devices")
            .delete_many(filter.clone(), None)
            .await
            .map_err(|e| format!("Delete controller devices: {}", e))?;

        self.db
            .collection::<bson::Document>("omada_clients")
            .delete_many(filter.clone(), None)
            .await
            .map_err(|e| format!("Delete controller clients: {}", e))?;

        self.db
            .collection::<bson::Document>("omada_wg_peers")
            .delete_many(filter, None)
            .await
            .map_err(|e| format!("Delete controller wg_peers: {}", e))?;

        Ok(())
    }

    /// Update controller status and optional error message
    pub async fn update_omada_controller_status(
        &self,
        controller_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("omada_controllers");
        let now = Utc::now().to_rfc3339();

        let mut set_doc = doc! {
            "status": status,
            "updated_at": &now,
        };

        if status == "connected" {
            set_doc.insert("last_synced_at", &now);
            set_doc.insert("last_error", bson::Bson::Null);
        }

        if let Some(err) = error {
            set_doc.insert("last_error", err);
        }

        collection
            .update_one(
                doc! { "controller_id": controller_id },
                doc! { "$set": set_doc },
                None,
            )
            .await
            .map_err(|e| format!("Update controller status: {}", e))?;

        Ok(())
    }

    // ========================================================================
    // Devices
    // ========================================================================

    /// Upsert devices for a controller+site (key: mac + controller_id)
    pub async fn upsert_omada_devices(
        &self,
        controller_id: &str,
        site_id: &str,
        devices: &[OmadaDevice],
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("omada_devices");
        let now = Utc::now().to_rfc3339();

        for device in devices {
            let mac = normalize_mac(&device.mac);
            let product_type = device_type_to_product_type(&device.device_type).to_string();
            let network_device_type =
                device_type_to_network_device_type(&device.device_type).to_string();

            let filter = doc! {
                "mac": &mac,
                "controller_id": controller_id,
            };

            let update = doc! {
                "$set": {
                    "mac": &mac,
                    "controller_id": controller_id,
                    "site_id": site_id,
                    "name": &device.name,
                    "device_type": &device.device_type,
                    "model": &device.model,
                    "ip": &device.ip,
                    "status": device.status,
                    "firmware_version": &device.firmware_version,
                    "product_type": &product_type,
                    "network_device_type": &network_device_type,
                    "synced_at": &now,
                    "updated_at": &now,
                },
                "$setOnInsert": {
                    "lacis_id": bson::Bson::Null,
                    "created_at": &now,
                }
            };

            let options = UpdateOptions::builder().upsert(true).build();
            collection
                .update_one(filter, update, Some(options))
                .await
                .map_err(|e| format!("Upsert device {}: {}", mac, e))?;
        }

        Ok(())
    }

    /// Get devices with optional filters
    pub async fn get_omada_devices(
        &self,
        controller_id: Option<&str>,
        site_id: Option<&str>,
    ) -> Result<Vec<OmadaDeviceDoc>, String> {
        let collection = self.db.collection::<bson::Document>("omada_devices");

        let mut filter = doc! {};
        if let Some(cid) = controller_id {
            filter.insert("controller_id", cid);
        }
        if let Some(sid) = site_id {
            filter.insert("site_id", sid);
        }

        let options = FindOptions::builder()
            .sort(doc! { "device_type": 1, "name": 1 })
            .build();

        let mut cursor = collection
            .find(filter, Some(options))
            .await
            .map_err(|e| format!("Get devices: {}", e))?;

        let mut devices = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("Cursor devices: {}", e))?
        {
            if let Ok(device) = bson::from_document(doc) {
                devices.push(device);
            }
        }

        Ok(devices)
    }

    // ========================================================================
    // Clients
    // ========================================================================

    /// Upsert clients for a controller+site (key: mac + controller_id + site_id)
    pub async fn upsert_omada_clients(
        &self,
        controller_id: &str,
        site_id: &str,
        clients: &[OmadaClientDevice],
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("omada_clients");
        let now = Utc::now().to_rfc3339();

        for client in clients {
            let mac = normalize_mac(&client.mac);

            let filter = doc! {
                "mac": &mac,
                "controller_id": controller_id,
                "site_id": site_id,
            };

            // Convert last_seen epoch (ms) to ISO 8601 string if available
            let last_seen_at = client.last_seen.map(|ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            });

            let update = doc! {
                "$set": {
                    "mac": &mac,
                    "controller_id": controller_id,
                    "site_id": site_id,
                    "name": &client.name,
                    "host_name": &client.host_name,
                    "ip": &client.ip,
                    "ipv6_list": client.ipv6_list.as_ref().unwrap_or(&vec![]),
                    "vendor": &client.vendor,
                    "device_type": &client.device_type,
                    "device_category": &client.device_category,
                    "os_name": &client.os_name,
                    "model": &client.model,
                    "connect_type": &client.connect_type,
                    "wireless": client.wireless.unwrap_or(false),
                    "ssid": &client.ssid,
                    "signal_level": &client.signal_level,
                    "rssi": &client.rssi,
                    "ap_mac": &client.ap_mac,
                    "ap_name": &client.ap_name,
                    "wifi_mode": &client.wifi_mode,
                    "channel": &client.channel,
                    "switch_mac": &client.switch_mac,
                    "switch_name": &client.switch_name,
                    "port": &client.port,
                    "vid": &client.vid,
                    "traffic_down": client.traffic_down.unwrap_or(0) as i64,
                    "traffic_up": client.traffic_up.unwrap_or(0) as i64,
                    "uptime": client.uptime.unwrap_or(0) as i64,
                    "active": client.active.unwrap_or(false),
                    "blocked": client.blocked.unwrap_or(false),
                    "guest": client.guest.unwrap_or(false),
                    "last_seen_at": &last_seen_at,
                    "synced_at": &now,
                    "updated_at": &now,
                },
                "$setOnInsert": {
                    "lacis_id": bson::Bson::Null,
                    "created_at": &now,
                }
            };

            let options = UpdateOptions::builder().upsert(true).build();
            collection
                .update_one(filter, update, Some(options))
                .await
                .map_err(|e| format!("Upsert client {}: {}", mac, e))?;
        }

        Ok(())
    }

    /// Get clients with optional filters
    pub async fn get_omada_clients(
        &self,
        controller_id: Option<&str>,
        site_id: Option<&str>,
        active_only: Option<bool>,
    ) -> Result<Vec<OmadaClientDoc>, String> {
        let collection = self.db.collection::<bson::Document>("omada_clients");

        let mut filter = doc! {};
        if let Some(cid) = controller_id {
            filter.insert("controller_id", cid);
        }
        if let Some(sid) = site_id {
            filter.insert("site_id", sid);
        }
        if active_only == Some(true) {
            filter.insert("active", true);
        }

        let options = FindOptions::builder()
            .sort(doc! { "active": -1, "name": 1 })
            .build();

        let mut cursor = collection
            .find(filter, Some(options))
            .await
            .map_err(|e| format!("Get clients: {}", e))?;

        let mut clients = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("Cursor clients: {}", e))?
        {
            if let Ok(client) = bson::from_document(doc) {
                clients.push(client);
            }
        }

        Ok(clients)
    }

    // ========================================================================
    // WireGuard Peers
    // ========================================================================

    /// Upsert WireGuard peers for a controller+site (key: peer_id)
    pub async fn upsert_omada_wg_peers(
        &self,
        controller_id: &str,
        site_id: &str,
        peers: &[WireGuardPeer],
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("omada_wg_peers");
        let now = Utc::now().to_rfc3339();

        for peer in peers {
            let filter = doc! { "peer_id": &peer.id };

            let update = doc! {
                "$set": {
                    "peer_id": &peer.id,
                    "controller_id": controller_id,
                    "site_id": site_id,
                    "name": &peer.name,
                    "status": peer.status,
                    "interface_id": &peer.interface_id,
                    "interface_name": &peer.interface_name,
                    "public_key": &peer.public_key,
                    "allow_address": &peer.allow_address,
                    "keep_alive": peer.keep_alive.unwrap_or(0),
                    "comment": &peer.comment,
                    "synced_at": &now,
                    "updated_at": &now,
                },
                "$setOnInsert": {
                    "created_at": &now,
                }
            };

            let options = UpdateOptions::builder().upsert(true).build();
            collection
                .update_one(filter, update, Some(options))
                .await
                .map_err(|e| format!("Upsert wg_peer {}: {}", peer.id, e))?;
        }

        Ok(())
    }

    /// Get WireGuard peers with optional filters
    pub async fn get_omada_wg_peers(
        &self,
        controller_id: Option<&str>,
        site_id: Option<&str>,
    ) -> Result<Vec<OmadaWgPeerDoc>, String> {
        let collection = self.db.collection::<bson::Document>("omada_wg_peers");

        let mut filter = doc! {};
        if let Some(cid) = controller_id {
            filter.insert("controller_id", cid);
        }
        if let Some(sid) = site_id {
            filter.insert("site_id", sid);
        }

        let options = FindOptions::builder()
            .sort(doc! { "interface_name": 1, "name": 1 })
            .build();

        let mut cursor = collection
            .find(filter, Some(options))
            .await
            .map_err(|e| format!("Get wg_peers: {}", e))?;

        let mut peers = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("Cursor wg_peers: {}", e))?
        {
            if let Ok(peer) = bson::from_document(doc) {
                peers.push(peer);
            }
        }

        Ok(peers)
    }

    // ========================================================================
    // Summary (aggregated counts across all controllers)
    // ========================================================================

    /// Get aggregated summary counts across all controllers
    pub async fn get_omada_summary(&self) -> Result<OmadaSummaryDoc, String> {
        let total_controllers = self
            .db
            .collection::<bson::Document>("omada_controllers")
            .count_documents(doc! {}, None)
            .await
            .unwrap_or(0);

        let connected_controllers = self
            .db
            .collection::<bson::Document>("omada_controllers")
            .count_documents(doc! { "status": "connected" }, None)
            .await
            .unwrap_or(0);

        let total_devices = self
            .db
            .collection::<bson::Document>("omada_devices")
            .count_documents(doc! {}, None)
            .await
            .unwrap_or(0);

        let online_devices = self
            .db
            .collection::<bson::Document>("omada_devices")
            .count_documents(doc! { "status": 1 }, None)
            .await
            .unwrap_or(0);

        let total_clients = self
            .db
            .collection::<bson::Document>("omada_clients")
            .count_documents(doc! {}, None)
            .await
            .unwrap_or(0);

        let active_clients = self
            .db
            .collection::<bson::Document>("omada_clients")
            .count_documents(doc! { "active": true }, None)
            .await
            .unwrap_or(0);

        let total_wg_peers = self
            .db
            .collection::<bson::Document>("omada_wg_peers")
            .count_documents(doc! {}, None)
            .await
            .unwrap_or(0);

        let active_wg_peers = self
            .db
            .collection::<bson::Document>("omada_wg_peers")
            .count_documents(doc! { "status": true }, None)
            .await
            .unwrap_or(0);

        Ok(OmadaSummaryDoc {
            total_controllers,
            connected_controllers,
            total_devices,
            online_devices,
            total_clients,
            active_clients,
            total_wg_peers,
            active_wg_peers,
        })
    }
}
