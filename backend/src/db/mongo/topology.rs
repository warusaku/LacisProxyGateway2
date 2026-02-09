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
    pub async fn batch_upsert_node_positions(&self, positions: &[NodePosition]) -> Result<(), String> {
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
        let collection = self.db.collection::<LogicDeviceDoc>(COLLECTION_LOGIC_DEVICES);
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

    /// Get a single logic device
    pub async fn get_logic_device(&self, id: &str) -> Result<Option<LogicDeviceDoc>, String> {
        let collection = self.db.collection::<LogicDeviceDoc>(COLLECTION_LOGIC_DEVICES);
        collection
            .find_one(doc! { "id": id }, None)
            .await
            .map_err(|e| format!("Failed to get logic device: {}", e))
    }

    /// Create a logic device
    pub async fn create_logic_device(&self, device: &LogicDeviceDoc) -> Result<(), String> {
        let collection = self.db.collection::<LogicDeviceDoc>(COLLECTION_LOGIC_DEVICES);
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
    pub async fn get_all_custom_labels(&self) -> Result<std::collections::HashMap<String, String>, String> {
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

    /// Set or remove a custom label for a node
    pub async fn upsert_custom_label(&self, node_id: &str, label: &str) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION_CUSTOM_LABELS);
        let filter = doc! { "node_id": node_id };
        let update = doc! {
            "$set": {
                "node_id": node_id,
                "custom_label": label,
            }
        };
        let opts = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();
        collection
            .update_one(filter, update, Some(opts))
            .await
            .map_err(|e| format!("Failed to upsert custom label: {}", e))?;
        Ok(())
    }

    /// Remove a custom label (revert to auto-generated label)
    pub async fn delete_custom_label(&self, node_id: &str) -> Result<(), String> {
        let collection = self.db.collection::<Document>(COLLECTION_CUSTOM_LABELS);
        collection
            .delete_one(doc! { "node_id": node_id }, None)
            .await
            .map_err(|e| format!("Failed to delete custom label: {}", e))?;
        Ok(())
    }

    /// Delete a logic device
    pub async fn delete_logic_device(&self, id: &str) -> Result<bool, String> {
        let collection = self.db.collection::<LogicDeviceDoc>(COLLECTION_LOGIC_DEVICES);
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
}
