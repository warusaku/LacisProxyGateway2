//! MongoDB CRUD for external devices and clients
//!
//! Collections: `external_devices`, `external_clients`

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
pub struct ExternalDeviceDoc {
    pub device_id: String,
    pub display_name: String,
    pub mac: String,
    pub ip: String,
    pub protocol: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub status: String,
    pub device_model: Option<String>,
    pub client_count: u32,
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
pub struct ExternalClientDoc {
    pub mac: String,
    pub device_id: String,
    pub ip: Option<String>,
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
    // ---- Devices ----

    /// Upsert a device document (key: device_id)
    pub async fn upsert_external_device(&self, doc: &ExternalDeviceDoc) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("external_devices");
        let filter = doc! { "device_id": &doc.device_id };

        let bson_doc = bson::to_document(doc).map_err(|e| format!("Serialize device: {}", e))?;
        let update = doc! { "$set": bson_doc };

        let options = UpdateOptions::builder().upsert(true).build();
        collection
            .update_one(filter, update, Some(options))
            .await
            .map_err(|e| format!("Upsert device {}: {}", doc.device_id, e))?;

        Ok(())
    }

    /// Update device status and polling data
    pub async fn update_external_device_status(
        &self,
        device_id: &str,
        status: &str,
        device_model: Option<&str>,
        client_count: u32,
        last_error: Option<&str>,
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("external_devices");
        let now = Utc::now().to_rfc3339();
        let filter = doc! { "device_id": device_id };

        let update = doc! {
            "$set": {
                "status": status,
                "device_model": device_model,
                "client_count": client_count as i32,
                "last_error": last_error,
                "last_polled_at": &now,
                "updated_at": &now,
            }
        };

        collection
            .update_one(filter, update, None)
            .await
            .map_err(|e| format!("Update device status {}: {}", device_id, e))?;

        Ok(())
    }

    /// List all devices
    pub async fn list_external_devices(&self) -> Result<Vec<ExternalDeviceDoc>, String> {
        let collection = self.db.collection::<bson::Document>("external_devices");
        let options = FindOptions::builder()
            .sort(doc! { "display_name": 1 })
            .build();

        let mut cursor = collection
            .find(doc! {}, Some(options))
            .await
            .map_err(|e| format!("List devices: {}", e))?;

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

    /// Get a single device by ID
    pub async fn get_external_device(
        &self,
        device_id: &str,
    ) -> Result<Option<ExternalDeviceDoc>, String> {
        let collection = self.db.collection::<bson::Document>("external_devices");
        let filter = doc! { "device_id": device_id };

        let doc = collection
            .find_one(filter, None)
            .await
            .map_err(|e| format!("Get device {}: {}", device_id, e))?;

        match doc {
            Some(d) => {
                let device =
                    bson::from_document(d).map_err(|e| format!("Deserialize device: {}", e))?;
                Ok(Some(device))
            }
            None => Ok(None),
        }
    }

    /// Delete a device and its clients
    pub async fn delete_external_device(&self, device_id: &str) -> Result<(), String> {
        self.db
            .collection::<bson::Document>("external_devices")
            .delete_one(doc! { "device_id": device_id }, None)
            .await
            .map_err(|e| format!("Delete device {}: {}", device_id, e))?;

        self.db
            .collection::<bson::Document>("external_clients")
            .delete_many(doc! { "device_id": device_id }, None)
            .await
            .map_err(|e| format!("Delete device clients {}: {}", device_id, e))?;

        Ok(())
    }

    // ---- Clients ----

    /// Upsert clients for a device (key: mac + device_id)
    pub async fn upsert_external_clients(
        &self,
        device_id: &str,
        clients: &[crate::external::mercury::MercuryClientInfo],
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("external_clients");
        let now = Utc::now().to_rfc3339();

        // Mark all existing clients for this device as inactive
        collection
            .update_many(
                doc! { "device_id": device_id, "active": true },
                doc! { "$set": { "active": false, "updated_at": &now } },
                None,
            )
            .await
            .map_err(|e| format!("Mark inactive: {}", e))?;

        // Upsert current clients
        for client in clients {
            let filter = doc! { "mac": &client.mac, "device_id": device_id };

            let update = doc! {
                "$set": {
                    "mac": &client.mac,
                    "device_id": device_id,
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

    /// Get clients with optional device_id filter
    pub async fn get_external_clients(
        &self,
        device_id: Option<&str>,
    ) -> Result<Vec<ExternalClientDoc>, String> {
        let collection = self.db.collection::<bson::Document>("external_clients");

        let mut filter = doc! {};
        if let Some(did) = device_id {
            filter.insert("device_id", did);
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

    /// Get external device summary counts
    pub async fn get_external_summary(&self) -> Result<serde_json::Value, String> {
        let total_devices = self
            .db
            .collection::<bson::Document>("external_devices")
            .count_documents(doc! {}, None)
            .await
            .unwrap_or(0);

        let online_devices = self
            .db
            .collection::<bson::Document>("external_devices")
            .count_documents(doc! { "status": "online" }, None)
            .await
            .unwrap_or(0);

        let total_clients = self
            .db
            .collection::<bson::Document>("external_clients")
            .count_documents(doc! {}, None)
            .await
            .unwrap_or(0);

        let active_clients = self
            .db
            .collection::<bson::Document>("external_clients")
            .count_documents(doc! { "active": true }, None)
            .await
            .unwrap_or(0);

        Ok(serde_json::json!({
            "total_devices": total_devices,
            "online_devices": online_devices,
            "total_clients": total_clients,
            "active_clients": active_clients,
        }))
    }
}
