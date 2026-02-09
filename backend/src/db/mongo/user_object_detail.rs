//! MongoDB CRUD for `user_object_detail` collection — the SSoT for CelestialGlobe topology.
//!
//! Document key `_id`:
//!   - Infrastructure nodes: LacisID (20-digit, prefix "4" + productType + MAC + productCode)
//!   - Clients/WG Peers: MAC (12-digit uppercase HEX)
//!   - Logic devices: Pseudo-MAC "F2" + 10 hex chars
//!
//! Parent eligibility: `_id.len() == 20` (LacisID) or `_id.starts_with("F2")` (Logic Device)

use mongodb::bson::{doc, Document};
use serde::{Deserialize, Serialize};

use super::MongoDb;

const COLLECTION: &str = "user_object_detail";

/// UserObjectDetail — a single node in the CelestialGlobe topology.
///
/// Fields map 1:1 to MongoDB document fields.
/// `id` maps to `_id` in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserObjectDetail {
    pub id: String,                        // _id: LacisID (20) or MAC (12)
    pub mac: String,                       // 12-digit uppercase HEX
    pub lacis_id: Option<String>,          // Confirmed LacisID (infra only)
    pub device_type: String,               // "NetworkDevice" or "araneaDevice"
    pub parent_id: String,                 // Parent _id (LacisID) or "INTERNET"
    pub sort_order: u32,                   // Sibling order (the only layout parameter)
    pub node_type: String,                 // gateway, switch, ap, client, wg_peer, etc.
    pub state_type: String,                // online, offline, StaticOnline, StaticOffline
    pub label: String,
    pub label_customized: bool,
    pub ip: Option<String>,
    pub hostname: Option<String>,
    pub source: String,                    // omada, openwrt, external, manual
    pub source_ref_id: Option<String>,
    pub connection_type: String,           // wired, wireless, vpn
    pub product_type: Option<String>,
    pub product_code: Option<String>,
    pub network_device_type: Option<String>,
    pub candidate_lacis_id: Option<String>,
    pub fid: Option<String>,
    pub facility_name: Option<String>,
    pub ssid: Option<String>,
    pub metadata: serde_json::Value,
    pub aranea_lacis_id: Option<String>,   // araneaDevice match: prefix-3 LacisID
    pub created_at: String,
    pub updated_at: String,
}

impl UserObjectDetail {
    /// Whether this node can be a parent (LacisID format or Logic Device pseudo-MAC)
    pub fn can_be_parent(id: &str) -> bool {
        id.len() == 20 || id.starts_with("F2")
    }
}

impl MongoDb {
    /// Get all user object detail entries
    pub async fn get_all_user_object_details(&self) -> Result<Vec<UserObjectDetail>, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let mut cursor = collection
            .find(None, None)
            .await
            .map_err(|e| format!("Failed to query user_object_detail: {}", e))?;

        let mut entries = Vec::new();
        while {
            use futures::StreamExt;
            match cursor.next().await {
                Some(Ok(doc)) => {
                    if let Ok(entry) = doc_to_user_object_detail(&doc) {
                        entries.push(entry);
                    }
                    true
                }
                Some(Err(e)) => {
                    tracing::warn!("Error reading user_object_detail: {}", e);
                    true
                }
                None => false,
            }
        } {}
        Ok(entries)
    }

    /// Get a single user object detail by _id
    pub async fn get_user_object_detail_by_id(
        &self,
        id: &str,
    ) -> Result<Option<UserObjectDetail>, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let doc = collection
            .find_one(doc! { "_id": id }, None)
            .await
            .map_err(|e| format!("Failed to get user_object_detail: {}", e))?;

        match doc {
            Some(d) => Ok(Some(
                doc_to_user_object_detail(&d)
                    .map_err(|e| format!("Failed to parse user_object_detail: {}", e))?,
            )),
            None => Ok(None),
        }
    }

    /// Upsert a user object detail entry.
    ///
    /// If the _id already exists, respects immutable fields:
    /// - parent_id, sort_order are NOT overwritten for existing entries
    /// - label is NOT overwritten if label_customized=true
    /// - state_type, ip, hostname, metadata, updated_at ARE always updated
    pub async fn upsert_user_object_detail(&self, entry: &UserObjectDetail) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let filter = doc! { "_id": &entry.id };

        // Check if entry already exists
        let existing = self.get_user_object_detail_by_id(&entry.id).await?;

        if let Some(existing) = existing {
            // Update volatile fields only — preserve topology structure
            let mut set_doc = doc! {
                "state_type": &entry.state_type,
                "ip": entry.ip.as_ref().map(|s| mongodb::bson::Bson::String(s.clone())).unwrap_or(mongodb::bson::Bson::Null),
                "hostname": entry.hostname.as_ref().map(|s| mongodb::bson::Bson::String(s.clone())).unwrap_or(mongodb::bson::Bson::Null),
                "ssid": entry.ssid.as_ref().map(|s| mongodb::bson::Bson::String(s.clone())).unwrap_or(mongodb::bson::Bson::Null),
                "updated_at": &entry.updated_at,
            };

            // Update metadata
            if let Ok(bson_val) = mongodb::bson::to_bson(&entry.metadata) {
                set_doc.insert("metadata", bson_val);
            }

            // Update label only if not customized
            if !existing.label_customized {
                set_doc.insert("label", &entry.label);
            }

            // Update lacis_id if provided (don't overwrite existing with None)
            if entry.lacis_id.is_some() {
                set_doc.insert("lacis_id", entry.lacis_id.as_deref().unwrap());
            }
            if entry.candidate_lacis_id.is_some() {
                set_doc.insert(
                    "candidate_lacis_id",
                    entry.candidate_lacis_id.as_deref().unwrap(),
                );
            }

            // Update device_type if changed (araneaDevice detection)
            set_doc.insert("device_type", &entry.device_type);
            if entry.aranea_lacis_id.is_some() {
                set_doc.insert(
                    "aranea_lacis_id",
                    entry.aranea_lacis_id.as_deref().unwrap(),
                );
            }

            collection
                .update_one(filter, doc! { "$set": set_doc }, None)
                .await
                .map_err(|e| format!("Failed to update user_object_detail: {}", e))?;
        } else {
            // New entry: insert all fields
            let insert_doc = user_object_detail_to_doc(entry);
            collection
                .insert_one(insert_doc, None)
                .await
                .map_err(|e| format!("Failed to insert user_object_detail: {}", e))?;
        }

        Ok(())
    }

    /// Update only the parent_id of a node (for reparent operations)
    pub async fn update_user_object_detail_parent(
        &self,
        id: &str,
        new_parent_id: &str,
    ) -> Result<bool, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let result = collection
            .update_one(
                doc! { "_id": id },
                doc! { "$set": {
                    "parent_id": new_parent_id,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                }},
                None,
            )
            .await
            .map_err(|e| format!("Failed to update user_object_detail parent: {}", e))?;
        Ok(result.modified_count > 0)
    }

    /// Update sort_order for a node
    pub async fn update_user_object_detail_sort_order(
        &self,
        id: &str,
        sort_order: u32,
    ) -> Result<bool, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let result = collection
            .update_one(
                doc! { "_id": id },
                doc! { "$set": {
                    "sort_order": sort_order,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                }},
                None,
            )
            .await
            .map_err(|e| format!("Failed to update sort_order: {}", e))?;
        Ok(result.modified_count > 0)
    }

    /// Update the label of a node and set label_customized flag
    pub async fn update_user_object_detail_label(
        &self,
        id: &str,
        label: &str,
        customized: bool,
    ) -> Result<bool, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let result = collection
            .update_one(
                doc! { "_id": id },
                doc! { "$set": {
                    "label": label,
                    "label_customized": customized,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                }},
                None,
            )
            .await
            .map_err(|e| format!("Failed to update label: {}", e))?;
        Ok(result.modified_count > 0)
    }

    /// Update state_type for a node (admin manual override)
    pub async fn update_user_object_detail_state_type(
        &self,
        id: &str,
        state_type: &str,
    ) -> Result<bool, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let result = collection
            .update_one(
                doc! { "_id": id },
                doc! { "$set": {
                    "state_type": state_type,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                }},
                None,
            )
            .await
            .map_err(|e| format!("Failed to update state_type: {}", e))?;
        Ok(result.modified_count > 0)
    }

    /// Delete a user object detail entry by _id
    pub async fn delete_user_object_detail(&self, id: &str) -> Result<bool, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let result = collection
            .delete_one(doc! { "_id": id }, None)
            .await
            .map_err(|e| format!("Failed to delete user_object_detail: {}", e))?;
        Ok(result.deleted_count > 0)
    }

    /// Count user object detail entries (used for migration check)
    pub async fn count_user_object_details(&self) -> Result<u64, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        collection
            .count_documents(None, None)
            .await
            .map_err(|e| format!("Failed to count user_object_detail: {}", e))
    }

    /// Find user object detail by MAC field (not _id)
    pub async fn get_user_object_detail_by_mac(
        &self,
        mac: &str,
    ) -> Result<Option<UserObjectDetail>, String> {
        let collection = self.db.collection::<Document>(COLLECTION);
        let doc = collection
            .find_one(doc! { "mac": mac }, None)
            .await
            .map_err(|e| format!("Failed to get user_object_detail by mac: {}", e))?;

        match doc {
            Some(d) => Ok(Some(
                doc_to_user_object_detail(&d)
                    .map_err(|e| format!("Failed to parse user_object_detail: {}", e))?,
            )),
            None => Ok(None),
        }
    }
}

// ============================================================================
// Document conversion helpers
// ============================================================================

fn user_object_detail_to_doc(entry: &UserObjectDetail) -> Document {
    let mut doc = doc! {
        "_id": &entry.id,
        "mac": &entry.mac,
        "device_type": &entry.device_type,
        "parent_id": &entry.parent_id,
        "sort_order": entry.sort_order,
        "node_type": &entry.node_type,
        "state_type": &entry.state_type,
        "label": &entry.label,
        "label_customized": entry.label_customized,
        "source": &entry.source,
        "connection_type": &entry.connection_type,
        "created_at": &entry.created_at,
        "updated_at": &entry.updated_at,
    };

    // Optional fields
    if let Some(ref v) = entry.lacis_id {
        doc.insert("lacis_id", v);
    }
    if let Some(ref v) = entry.ip {
        doc.insert("ip", v);
    }
    if let Some(ref v) = entry.hostname {
        doc.insert("hostname", v);
    }
    if let Some(ref v) = entry.source_ref_id {
        doc.insert("source_ref_id", v);
    }
    if let Some(ref v) = entry.product_type {
        doc.insert("product_type", v);
    }
    if let Some(ref v) = entry.product_code {
        doc.insert("product_code", v);
    }
    if let Some(ref v) = entry.network_device_type {
        doc.insert("network_device_type", v);
    }
    if let Some(ref v) = entry.candidate_lacis_id {
        doc.insert("candidate_lacis_id", v);
    }
    if let Some(ref v) = entry.fid {
        doc.insert("fid", v);
    }
    if let Some(ref v) = entry.facility_name {
        doc.insert("facility_name", v);
    }
    if let Some(ref v) = entry.ssid {
        doc.insert("ssid", v);
    }
    if let Some(ref v) = entry.aranea_lacis_id {
        doc.insert("aranea_lacis_id", v);
    }
    if let Ok(bson_val) = mongodb::bson::to_bson(&entry.metadata) {
        doc.insert("metadata", bson_val);
    }

    doc
}

fn doc_to_user_object_detail(doc: &Document) -> Result<UserObjectDetail, String> {
    let get_str = |key: &str| -> String {
        doc.get_str(key).unwrap_or_default().to_string()
    };
    let get_opt_str = |key: &str| -> Option<String> {
        doc.get_str(key).ok().map(|s| s.to_string())
    };

    Ok(UserObjectDetail {
        id: doc
            .get_str("_id")
            .map(|s| s.to_string())
            .or_else(|_| {
                // _id might be ObjectId in some edge cases
                doc.get_object_id("_id")
                    .map(|oid| oid.to_hex())
            })
            .map_err(|_| "Missing _id".to_string())?,
        mac: get_str("mac"),
        lacis_id: get_opt_str("lacis_id"),
        device_type: get_str("device_type"),
        parent_id: get_str("parent_id"),
        sort_order: doc.get_i32("sort_order").unwrap_or(0) as u32,
        node_type: get_str("node_type"),
        state_type: get_str("state_type"),
        label: get_str("label"),
        label_customized: doc.get_bool("label_customized").unwrap_or(false),
        ip: get_opt_str("ip"),
        hostname: get_opt_str("hostname"),
        source: get_str("source"),
        source_ref_id: get_opt_str("source_ref_id"),
        connection_type: get_str("connection_type"),
        product_type: get_opt_str("product_type"),
        product_code: get_opt_str("product_code"),
        network_device_type: get_opt_str("network_device_type"),
        candidate_lacis_id: get_opt_str("candidate_lacis_id"),
        fid: get_opt_str("fid"),
        facility_name: get_opt_str("facility_name"),
        ssid: get_opt_str("ssid"),
        metadata: doc
            .get("metadata")
            .and_then(|v| mongodb::bson::from_bson(v.clone()).ok())
            .unwrap_or(serde_json::json!({})),
        aranea_lacis_id: get_opt_str("aranea_lacis_id"),
        created_at: get_str("created_at"),
        updated_at: get_str("updated_at"),
    })
}
