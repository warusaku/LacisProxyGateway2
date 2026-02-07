//! IP address history tracking for exclusion filters
//!
//! Records when each IP was first/last seen and its source (server or admin).
//! Used to exclude all historical IPs from access log analysis, even after IP changes.

use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::{self, doc};
use mongodb::options::{FindOptions, UpdateOptions};

use super::MongoDb;
use crate::error::AppError;

/// IP history entry
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IpHistoryEntry {
    /// IP address
    pub ip: String,
    /// Source: "server" (LPG global IP) or "admin" (management access IP)
    pub source: String,
    /// First seen timestamp (ISO 8601 string)
    pub first_seen: String,
    /// Last seen timestamp (ISO 8601 string)
    pub last_seen: String,
}

impl MongoDb {
    /// Upsert an IP address into ip_history.
    /// - If the IP+source combination already exists, update `last_seen`.
    /// - If new, set both `first_seen` and `last_seen`.
    pub async fn upsert_ip_history(&self, ip: &str, source: &str) -> Result<(), AppError> {
        let collection = self.db.collection::<bson::Document>("ip_history");
        let now = Utc::now().to_rfc3339();

        let filter = doc! { "ip": ip, "source": source };
        let update = doc! {
            "$set": {
                "last_seen": &now,
            },
            "$setOnInsert": {
                "ip": ip,
                "source": source,
                "first_seen": &now,
            }
        };

        let options = UpdateOptions::builder().upsert(true).build();

        collection
            .update_one(filter, update, Some(options))
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to upsert ip_history: {}", e)))?;

        Ok(())
    }

    /// Get all historical IPs for a given source ("server" or "admin").
    /// Returns a list of unique IP strings, ordered by first_seen descending.
    pub async fn get_ip_history(&self, source: &str) -> Result<Vec<String>, AppError> {
        let collection = self.db.collection::<bson::Document>("ip_history");

        let filter = doc! { "source": source };
        let options = FindOptions::builder()
            .sort(doc! { "first_seen": -1 })
            .build();

        let mut cursor = collection
            .find(filter, Some(options))
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to query ip_history: {}", e)))?;

        let mut ips = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(ip) = doc.get_str("ip") {
                ips.push(ip.to_string());
            }
        }

        Ok(ips)
    }
}
