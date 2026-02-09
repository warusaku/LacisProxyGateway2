//! MongoDB persistence for CelestialGlobe topology state
//!
//! Collection: `celestial_globe_topology`
//! Stores: node positions, collapsed state, logic devices

use mongodb::bson::{doc, Document};
use serde::{Deserialize, Serialize};

use super::MongoDb;

const COLLECTION_POSITIONS: &str = "cg_node_positions";
const COLLECTION_LOGIC_DEVICES: &str = "cg_logic_devices";
const COLLECTION_STATE: &str = "cg_state";
const COLLECTION_CUSTOM_LABELS: &str = "cg_custom_labels";
const COLLECTION_NODE_ORDER: &str = "cg_node_order";

// ============================================================================
// Data models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicDeviceDoc {
    pub id: String,
    pub label: String,
    pub device_type: String,
    pub parent_id: Option<String>,
    pub ip: Option<String>,
    pub location: Option<String>,
    pub note: Option<String>,
    pub lacis_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyStateDoc {
    pub key: String, // always "global"
    pub collapsed_node_ids: Vec<String>,
    pub last_layout_at: String,
}

/// nodeOrder SSoT entry — the single source of truth for CelestialGlobe topology.
/// Collection: `cg_node_order`, keyed by `mac` (12-digit uppercase HEX).
///
/// nodeOrder absolute rules:
/// 1. nodeOrder = 唯一のSSoT。nodeOrderに存在 = 描画対象
/// 2. 全ノードは完全に等価。managed/detected/pendingの区別禁止
/// 3. ネットワーク構造: INTERNET → Gateway → Children → ...
/// 4. Gateway不在 = ネットワーク障害。孤児ノードはGatewayにフォールバック (INTERNET直結禁止)
/// 5. parentMacはLacisID登録済みノードのMACのみ (永続性保証)
/// 6. Controllerは管理ソフトウェア、物理トポロジーには含めない
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOrderEntry {
    pub mac: String,        // 12桁大文字HEX (ドキュメントキー)
    pub parent_mac: String, // 親MAC or "INTERNET"
    pub depth: u32,         // 階層深度 (1=gateway)
    pub order: u32,         // 兄弟順序
    pub label: String,      // 表示名
    pub node_type: String,  // gateway, switch, ap, client, wg_peer, etc.
    pub ip: Option<String>,
    pub hostname: Option<String>,
    pub source: String,                // omada, openwrt, external, manual
    pub source_ref_id: Option<String>, // 元のソースID (逆引き用)
    pub status: String,                // online, offline, active, inactive
    pub state_type: String,            // trackingOnline等 (Phase2で活用)
    pub connection_type: String,       // wired, wireless, vpn
    pub lacis_id: Option<String>,
    pub candidate_lacis_id: Option<String>,
    pub product_type: Option<String>,
    pub network_device_type: Option<String>,
    pub fid: Option<String>,
    pub facility_name: Option<String>,
    pub metadata: serde_json::Value, // model, firmware, vendor等
    pub label_customized: bool,      // ユーザーによるラベル上書き
    pub ssid: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Node Positions CRUD
// ============================================================================

impl MongoDb {
    /// Get all node positions
    pub async fn get_all_node_positions(&self) -> Result<Vec<NodePosition>, String> {
        let collection = self.db.collection::<NodePosition>(COLLECTION_POSITIONS);
        let mut cursor = collection
            .find(None, None)
            .await
            .map_err(|e| format!("Failed to query node positions: {}", e))?;

        let mut positions = Vec::new();
        while {
            use futures::StreamExt;
            match cursor.next().await {
                Some(Ok(pos)) => {
                    positions.push(pos);
                    true
                }
                Some(Err(e)) => {
                    tracing::warn!("Error reading node position: {}", e);
                    true
                }
                None => false,
            }
        } {}
        Ok(positions)
    }

    /// Upsert a single node position
    pub async fn upsert_node_position(&self, pos: &NodePosition) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION_POSITIONS);
        let filter = doc! { "node_id": &pos.node_id };
        let update = doc! {
            "$set": {
                "node_id": &pos.node_id,
                "x": pos.x,
                "y": pos.y,
                "pinned": pos.pinned,
            }
        };
        let opts = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();
        collection
            .update_one(filter, update, Some(opts))
            .await
            .map_err(|e| format!("Failed to upsert node position: {}", e))?;
        Ok(())
    }

    /// Batch upsert node positions (for layout recalc)
    pub async fn batch_upsert_node_positions(
        &self,
        positions: &[NodePosition],
    ) -> Result<(), String> {
        for pos in positions {
            self.upsert_node_position(pos).await?;
        }
        Ok(())
    }

    // ========================================================================
    // Topology State (collapsed nodes, layout timestamp)
    // ========================================================================

    /// Get topology state
    pub async fn get_topology_state(&self) -> Result<TopologyStateDoc, String> {
        let collection = self.db.collection::<TopologyStateDoc>(COLLECTION_STATE);
        let result = collection
            .find_one(doc! { "key": "global" }, None)
            .await
            .map_err(|e| format!("Failed to get topology state: {}", e))?;
        Ok(result.unwrap_or(TopologyStateDoc {
            key: "global".to_string(),
            collapsed_node_ids: Vec::new(),
            last_layout_at: String::new(),
        }))
    }

    /// Update collapsed state for a node
    pub async fn set_node_collapsed(&self, node_id: &str, collapsed: bool) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION_STATE);
        let opts = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();
        if collapsed {
            collection
                .update_one(
                    doc! { "key": "global" },
                    doc! { "$addToSet": { "collapsed_node_ids": node_id } },
                    Some(opts),
                )
                .await
                .map_err(|e| format!("Failed to add collapse: {}", e))?;
        } else {
            collection
                .update_one(
                    doc! { "key": "global" },
                    doc! { "$pull": { "collapsed_node_ids": node_id } },
                    Some(opts),
                )
                .await
                .map_err(|e| format!("Failed to remove collapse: {}", e))?;
        }
        Ok(())
    }

    /// Update last_layout_at timestamp
    pub async fn set_last_layout_at(&self, timestamp: &str) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION_STATE);
        let opts = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();
        collection
            .update_one(
                doc! { "key": "global" },
                doc! {
                    "$set": { "last_layout_at": timestamp },
                    "$setOnInsert": { "key": "global", "collapsed_node_ids": mongodb::bson::Bson::Array(vec![]) }
                },
                Some(opts),
            )
            .await
            .map_err(|e| format!("Failed to set last_layout_at: {}", e))?;
        Ok(())
    }

    // ========================================================================
    // Logic Devices CRUD
    // ========================================================================

    /// List all logic devices
    pub async fn list_logic_devices(&self) -> Result<Vec<LogicDeviceDoc>, String> {
        let collection = self
            .db
            .collection::<LogicDeviceDoc>(COLLECTION_LOGIC_DEVICES);
        let mut cursor = collection
            .find(None, None)
            .await
            .map_err(|e| format!("Failed to list logic devices: {}", e))?;

        let mut devices = Vec::new();
        while {
            use futures::StreamExt;
            match cursor.next().await {
                Some(Ok(dev)) => {
                    devices.push(dev);
                    true
                }
                Some(Err(e)) => {
                    tracing::warn!("Error reading logic device: {}", e);
                    true
                }
                None => false,
            }
        } {}
        Ok(devices)
    }

    // get_logic_device() removed — nodeOrder SSoT replaces direct lookup

    /// Create a logic device
    pub async fn create_logic_device(&self, device: &LogicDeviceDoc) -> Result<(), String> {
        let collection = self
            .db
            .collection::<LogicDeviceDoc>(COLLECTION_LOGIC_DEVICES);
        collection
            .insert_one(device, None)
            .await
            .map_err(|e| format!("Failed to create logic device: {}", e))?;
        Ok(())
    }

    /// Update a logic device
    pub async fn update_logic_device(&self, id: &str, update: Document) -> Result<bool, String> {
        let collection = self.db.collection::<Document>(COLLECTION_LOGIC_DEVICES);
        let result = collection
            .update_one(doc! { "id": id }, doc! { "$set": update }, None)
            .await
            .map_err(|e| format!("Failed to update logic device: {}", e))?;
        Ok(result.modified_count > 0)
    }

    // ========================================================================
    // Custom Labels
    // ========================================================================

    /// Get all custom labels as HashMap<node_id, label>
    pub async fn get_all_custom_labels(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, String> {
        let collection = self.db.collection::<Document>(COLLECTION_CUSTOM_LABELS);
        let mut cursor = collection
            .find(None, None)
            .await
            .map_err(|e| format!("Failed to query custom labels: {}", e))?;

        let mut labels = std::collections::HashMap::new();
        while {
            use futures::StreamExt;
            match cursor.next().await {
                Some(Ok(doc)) => {
                    if let (Some(id), Some(label)) = (
                        doc.get_str("node_id").ok(),
                        doc.get_str("custom_label").ok(),
                    ) {
                        labels.insert(id.to_string(), label.to_string());
                    }
                    true
                }
                Some(Err(e)) => {
                    tracing::warn!("Error reading custom label: {}", e);
                    true
                }
                None => false,
            }
        } {}
        Ok(labels)
    }

    // upsert_custom_label() / delete_custom_label() removed
    // — nodeOrder.label + label_customized が代替 (SSoT)

    /// Delete a logic device
    pub async fn delete_logic_device(&self, id: &str) -> Result<bool, String> {
        let collection = self
            .db
            .collection::<LogicDeviceDoc>(COLLECTION_LOGIC_DEVICES);
        let result = collection
            .delete_one(doc! { "id": id }, None)
            .await
            .map_err(|e| format!("Failed to delete logic device: {}", e))?;

        // Also remove its position
        let pos_collection = self.db.collection::<Document>(COLLECTION_POSITIONS);
        let _ = pos_collection
            .delete_one(doc! { "node_id": id }, None)
            .await;

        Ok(result.deleted_count > 0)
    }

    // ========================================================================
    // NodeOrder SSoT CRUD (cg_node_order collection)
    // ========================================================================

    /// Get all node order entries
    pub async fn get_all_node_order(&self) -> Result<Vec<NodeOrderEntry>, String> {
        let collection = self.db.collection::<NodeOrderEntry>(COLLECTION_NODE_ORDER);
        let mut cursor = collection
            .find(None, None)
            .await
            .map_err(|e| format!("Failed to query node order: {}", e))?;

        let mut entries = Vec::new();
        while {
            use futures::StreamExt;
            match cursor.next().await {
                Some(Ok(entry)) => {
                    entries.push(entry);
                    true
                }
                Some(Err(e)) => {
                    tracing::warn!("Error reading node order entry: {}", e);
                    true
                }
                None => false,
            }
        } {}
        Ok(entries)
    }

    /// Get a single node order entry by MAC
    pub async fn get_node_order_by_mac(&self, mac: &str) -> Result<Option<NodeOrderEntry>, String> {
        let collection = self.db.collection::<NodeOrderEntry>(COLLECTION_NODE_ORDER);
        collection
            .find_one(doc! { "mac": mac }, None)
            .await
            .map_err(|e| format!("Failed to get node order entry: {}", e))
    }

    /// Full upsert for a node order entry (used during initial ingestion/migration).
    /// If the MAC already exists, this respects immutable fields:
    /// - parent_mac, depth, order are NOT overwritten for existing entries
    /// - label is NOT overwritten if label_customized=true
    /// - status, ip, hostname, metadata, updated_at ARE always updated
    pub async fn upsert_node_order(&self, entry: &NodeOrderEntry) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION_NODE_ORDER);
        let filter = doc! { "mac": &entry.mac };

        // Check if entry already exists
        let existing = self.get_node_order_by_mac(&entry.mac).await?;

        if let Some(existing) = existing {
            // Update volatile fields only — preserve topology structure
            let mut set_doc = doc! {
                "status": &entry.status,
                "ip": entry.ip.as_ref().map(|s| mongodb::bson::Bson::String(s.clone())).unwrap_or(mongodb::bson::Bson::Null),
                "hostname": entry.hostname.as_ref().map(|s| mongodb::bson::Bson::String(s.clone())).unwrap_or(mongodb::bson::Bson::Null),
                "updated_at": &entry.updated_at,
                "ssid": entry.ssid.as_ref().map(|s| mongodb::bson::Bson::String(s.clone())).unwrap_or(mongodb::bson::Bson::Null),
            };

            // Update metadata (merge, not replace)
            if let Ok(bson_val) = mongodb::bson::to_bson(&entry.metadata) {
                set_doc.insert("metadata", bson_val);
            }

            // Update label only if not customized
            if !existing.label_customized {
                set_doc.insert("label", &entry.label);
            }

            // Update lacis_id / candidate_lacis_id if provided (don't overwrite existing with None)
            if entry.lacis_id.is_some() {
                set_doc.insert("lacis_id", entry.lacis_id.as_deref().unwrap());
            }
            if entry.candidate_lacis_id.is_some() {
                set_doc.insert(
                    "candidate_lacis_id",
                    entry.candidate_lacis_id.as_deref().unwrap(),
                );
            }

            collection
                .update_one(filter, doc! { "$set": set_doc }, None)
                .await
                .map_err(|e| format!("Failed to update node order: {}", e))?;
        } else {
            // New entry: insert all fields
            let mut insert_doc = doc! {
                "mac": &entry.mac,
                "parent_mac": &entry.parent_mac,
                "depth": entry.depth,
                "order": entry.order,
                "label": &entry.label,
                "node_type": &entry.node_type,
                "source": &entry.source,
                "status": &entry.status,
                "state_type": &entry.state_type,
                "connection_type": &entry.connection_type,
                "label_customized": entry.label_customized,
                "created_at": &entry.created_at,
                "updated_at": &entry.updated_at,
            };

            // Optional fields
            if let Some(ref v) = entry.ip {
                insert_doc.insert("ip", v);
            }
            if let Some(ref v) = entry.hostname {
                insert_doc.insert("hostname", v);
            }
            if let Some(ref v) = entry.source_ref_id {
                insert_doc.insert("source_ref_id", v);
            }
            if let Some(ref v) = entry.lacis_id {
                insert_doc.insert("lacis_id", v);
            }
            if let Some(ref v) = entry.candidate_lacis_id {
                insert_doc.insert("candidate_lacis_id", v);
            }
            if let Some(ref v) = entry.product_type {
                insert_doc.insert("product_type", v);
            }
            if let Some(ref v) = entry.network_device_type {
                insert_doc.insert("network_device_type", v);
            }
            if let Some(ref v) = entry.fid {
                insert_doc.insert("fid", v);
            }
            if let Some(ref v) = entry.facility_name {
                insert_doc.insert("facility_name", v);
            }
            if let Some(ref v) = entry.ssid {
                insert_doc.insert("ssid", v);
            }
            if let Ok(bson_val) = mongodb::bson::to_bson(&entry.metadata) {
                insert_doc.insert("metadata", bson_val);
            }

            collection
                .insert_one(insert_doc, None)
                .await
                .map_err(|e| format!("Failed to insert node order: {}", e))?;
        }

        Ok(())
    }

    /// Update only the parent_mac and depth of a node (for reparent operations)
    pub async fn update_node_order_parent(
        &self,
        mac: &str,
        new_parent_mac: &str,
        new_depth: u32,
    ) -> Result<bool, String> {
        let collection = self.db.collection::<Document>(COLLECTION_NODE_ORDER);
        let result = collection
            .update_one(
                doc! { "mac": mac },
                doc! { "$set": {
                    "parent_mac": new_parent_mac,
                    "depth": new_depth,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                }},
                None,
            )
            .await
            .map_err(|e| format!("Failed to update node order parent: {}", e))?;
        Ok(result.modified_count > 0)
    }

    /// Update the label of a node and set label_customized flag
    pub async fn update_node_order_label(
        &self,
        mac: &str,
        label: &str,
        customized: bool,
    ) -> Result<bool, String> {
        let collection = self.db.collection::<Document>(COLLECTION_NODE_ORDER);
        let result = collection
            .update_one(
                doc! { "mac": mac },
                doc! { "$set": {
                    "label": label,
                    "label_customized": customized,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                }},
                None,
            )
            .await
            .map_err(|e| format!("Failed to update node order label: {}", e))?;
        Ok(result.modified_count > 0)
    }

    /// Delete a node order entry by MAC
    pub async fn delete_node_order(&self, mac: &str) -> Result<bool, String> {
        let collection = self.db.collection::<NodeOrderEntry>(COLLECTION_NODE_ORDER);
        let result = collection
            .delete_one(doc! { "mac": mac }, None)
            .await
            .map_err(|e| format!("Failed to delete node order: {}", e))?;

        // Also remove position and collapse state for this MAC
        let pos_collection = self.db.collection::<Document>(COLLECTION_POSITIONS);
        let _ = pos_collection
            .delete_one(doc! { "node_id": mac }, None)
            .await;

        Ok(result.deleted_count > 0)
    }

    /// Count node order entries (used for migration check)
    pub async fn count_node_order(&self) -> Result<u64, String> {
        let collection = self.db.collection::<NodeOrderEntry>(COLLECTION_NODE_ORDER);
        collection
            .count_documents(None, None)
            .await
            .map_err(|e| format!("Failed to count node order: {}", e))
    }

    /// Batch update node_id in cg_node_positions (for ID migration)
    pub async fn migrate_node_position_id(&self, old_id: &str, new_id: &str) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION_POSITIONS);
        let _ = collection
            .update_one(
                doc! { "node_id": old_id },
                doc! { "$set": { "node_id": new_id } },
                None,
            )
            .await
            .map_err(|e| format!("Failed to migrate position ID: {}", e))?;
        Ok(())
    }

    /// Batch update collapsed_node_ids in cg_state (for ID migration)
    pub async fn migrate_collapsed_node_id(
        &self,
        old_id: &str,
        new_id: &str,
    ) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION_STATE);
        // Pull old, add new
        let _ = collection
            .update_one(
                doc! { "key": "global", "collapsed_node_ids": old_id },
                doc! { "$set": { "collapsed_node_ids.$": new_id } },
                None,
            )
            .await
            .map_err(|e| format!("Failed to migrate collapsed node ID: {}", e))?;
        Ok(())
    }
}
