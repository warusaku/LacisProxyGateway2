//! MongoDB CRUD for OpenWrt routers and clients
//!
//! Collections: `openwrt_routers`, `openwrt_clients`

use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::{self, doc};
use mongodb::options::{FindOptions, UpdateOptions};
use serde::{Deserialize, Serialize};

use super::MongoDb;

// ============================================================================
// Document types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWrtRouterDoc {
    pub router_id: String,
    pub display_name: String,
    pub mac: String,
    pub ip: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub firmware: String,
    pub status: String,
    pub wan_ip: Option<String>,
    pub lan_ip: Option<String>,
    pub ssid_24g: Option<String>,
    pub ssid_5g: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub client_count: u32,
    pub firmware_version: Option<String>,
    pub last_error: Option<String>,
    // Omada linkage
    pub omada_controller_id: Option<String>,
    pub omada_site_id: Option<String>,
    // mobes2.0 compatible
    pub lacis_id: Option<String>,
    pub product_type: String,
    pub network_device_type: String,
    pub last_polled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWrtClientDoc {
    pub mac: String,
    pub router_id: String,
    pub ip: String,
    pub hostname: Option<String>,
    pub lacis_id: Option<String>,
    pub active: bool,
    pub last_seen_at: String,
    pub synced_at: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// MongoDB operations
// ============================================================================

impl MongoDb {
    // ---- Routers ----

    /// Upsert a router document (key: router_id)
    pub async fn upsert_openwrt_router(&self, doc: &OpenWrtRouterDoc) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("openwrt_routers");
        let filter = doc! { "router_id": &doc.router_id };

        let bson_doc = bson::to_document(doc).map_err(|e| format!("Serialize router: {}", e))?;
        let update = doc! { "$set": bson_doc };

        let options = UpdateOptions::builder().upsert(true).build();
        collection
            .update_one(filter, update, Some(options))
            .await
            .map_err(|e| format!("Upsert router {}: {}", doc.router_id, e))?;

        Ok(())
    }

    /// Update router status and polling data
    pub async fn update_openwrt_router_status(
        &self,
        router_id: &str,
        status: &str,
        wan_ip: Option<&str>,
        lan_ip: Option<&str>,
        ssid_24g: Option<&str>,
        ssid_5g: Option<&str>,
        uptime_seconds: Option<u64>,
        client_count: u32,
        firmware_version: Option<&str>,
        last_error: Option<&str>,
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("openwrt_routers");
        let now = Utc::now().to_rfc3339();
        let filter = doc! { "router_id": router_id };

        let update = doc! {
            "$set": {
                "status": status,
                "wan_ip": wan_ip,
                "lan_ip": lan_ip,
                "ssid_24g": ssid_24g,
                "ssid_5g": ssid_5g,
                "uptime_seconds": uptime_seconds.map(|u| u as i64),
                "client_count": client_count as i32,
                "firmware_version": firmware_version,
                "last_error": last_error,
                "last_polled_at": &now,
                "updated_at": &now,
            }
        };

        collection
            .update_one(filter, update, None)
            .await
            .map_err(|e| format!("Update router status {}: {}", router_id, e))?;

        Ok(())
    }

    /// List all routers
    pub async fn list_openwrt_routers(&self) -> Result<Vec<OpenWrtRouterDoc>, String> {
        let collection = self.db.collection::<bson::Document>("openwrt_routers");
        let options = FindOptions::builder()
            .sort(doc! { "display_name": 1 })
            .build();

        let mut cursor = collection
            .find(doc! {}, Some(options))
            .await
            .map_err(|e| format!("List routers: {}", e))?;

        let mut routers = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("Cursor routers: {}", e))?
        {
            if let Ok(router) = bson::from_document(doc) {
                routers.push(router);
            }
        }

        Ok(routers)
    }

    /// Get a single router by ID
    pub async fn get_openwrt_router(
        &self,
        router_id: &str,
    ) -> Result<Option<OpenWrtRouterDoc>, String> {
        let collection = self.db.collection::<bson::Document>("openwrt_routers");
        let filter = doc! { "router_id": router_id };

        let doc = collection
            .find_one(filter, None)
            .await
            .map_err(|e| format!("Get router {}: {}", router_id, e))?;

        match doc {
            Some(d) => {
                let router =
                    bson::from_document(d).map_err(|e| format!("Deserialize router: {}", e))?;
                Ok(Some(router))
            }
            None => Ok(None),
        }
    }

    /// Delete a router and its clients
    pub async fn delete_openwrt_router(&self, router_id: &str) -> Result<(), String> {
        self.db
            .collection::<bson::Document>("openwrt_routers")
            .delete_one(doc! { "router_id": router_id }, None)
            .await
            .map_err(|e| format!("Delete router {}: {}", router_id, e))?;

        self.db
            .collection::<bson::Document>("openwrt_clients")
            .delete_many(doc! { "router_id": router_id }, None)
            .await
            .map_err(|e| format!("Delete router clients {}: {}", router_id, e))?;

        Ok(())
    }

    // ---- Clients ----

    /// Upsert clients for a router (key: mac + router_id)
    pub async fn upsert_openwrt_clients(
        &self,
        router_id: &str,
        clients: &[crate::openwrt::client::RouterClientEntry],
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("openwrt_clients");
        let now = Utc::now().to_rfc3339();

        // Mark all existing clients for this router as inactive
        collection
            .update_many(
                doc! { "router_id": router_id, "active": true },
                doc! { "$set": { "active": false, "updated_at": &now } },
                None,
            )
            .await
            .map_err(|e| format!("Mark inactive: {}", e))?;

        // Upsert current clients
        for client in clients {
            let filter = doc! { "mac": &client.mac, "router_id": router_id };

            let update = doc! {
                "$set": {
                    "mac": &client.mac,
                    "router_id": router_id,
                    "ip": &client.ip,
                    "hostname": &client.hostname,
                    "active": true,
                    "last_seen_at": &now,
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
                .map_err(|e| format!("Upsert client {}: {}", client.mac, e))?;
        }

        Ok(())
    }

    /// Get clients with optional router_id filter
    pub async fn get_openwrt_clients(
        &self,
        router_id: Option<&str>,
    ) -> Result<Vec<OpenWrtClientDoc>, String> {
        let collection = self.db.collection::<bson::Document>("openwrt_clients");

        let mut filter = doc! {};
        if let Some(rid) = router_id {
            filter.insert("router_id", rid);
        }

        let options = FindOptions::builder()
            .sort(doc! { "active": -1, "last_seen_at": -1 })
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

    /// Get OpenWrt summary counts
    pub async fn get_openwrt_summary(&self) -> Result<serde_json::Value, String> {
        let total_routers = self
            .db
            .collection::<bson::Document>("openwrt_routers")
            .count_documents(doc! {}, None)
            .await
            .unwrap_or(0);

        let online_routers = self
            .db
            .collection::<bson::Document>("openwrt_routers")
            .count_documents(doc! { "status": "online" }, None)
            .await
            .unwrap_or(0);

        let total_clients = self
            .db
            .collection::<bson::Document>("openwrt_clients")
            .count_documents(doc! {}, None)
            .await
            .unwrap_or(0);

        let active_clients = self
            .db
            .collection::<bson::Document>("openwrt_clients")
            .count_documents(doc! { "active": true }, None)
            .await
            .unwrap_or(0);

        Ok(serde_json::json!({
            "total_routers": total_routers,
            "online_routers": online_routers,
            "total_clients": total_clients,
            "active_clients": active_clients,
        }))
    }
}
