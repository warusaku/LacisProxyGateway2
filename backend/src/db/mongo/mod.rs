//! MongoDB database module

mod access_log;
pub mod external;
mod ip_history;
pub mod omada;
pub mod operation_logs;
pub mod openwrt;
mod security_events;
pub mod topology;

use mongodb::{Client, Database};
use mongodb::bson::doc;

use crate::config::Config;

pub use self::access_log::*;
pub use self::operation_logs::*;
pub use self::security_events::*;

/// MongoDB database wrapper
#[derive(Clone)]
pub struct MongoDb {
    db: Database,
}

impl MongoDb {
    /// Connect to MongoDB database
    pub async fn connect(config: &Config) -> anyhow::Result<Self> {
        let url = config
            .database
            .mongodb_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MongoDB URL not configured"))?;

        tracing::info!("Connecting to MongoDB...");

        let client = Client::with_uri_str(url).await?;
        let db = client.database("lacis_proxy");

        // Verify connection
        db.run_command(mongodb::bson::doc! { "ping": 1 }, None)
            .await?;

        tracing::info!("MongoDB connected successfully");

        Ok(Self { db })
    }

    /// Get the database handle
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Assign a lacis_id to a device in the appropriate collection.
    /// `source`: "omada" → omada_devices (match by mac), "openwrt" → openwrt_routers (match by router_id), "external" → external_devices (match by device_id)
    pub async fn assign_lacis_id(
        &self,
        source: &str,
        device_id: &str,
        lacis_id: &str,
    ) -> Result<bool, String> {
        let (collection_name, filter) = match source {
            "omada" => (
                "omada_devices",
                doc! { "mac": device_id },
            ),
            "openwrt" => (
                "openwrt_routers",
                doc! { "router_id": device_id },
            ),
            "external" => (
                "external_devices",
                doc! { "device_id": device_id },
            ),
            _ => return Err(format!("Unknown source: {}", source)),
        };

        let collection = self.db.collection::<mongodb::bson::Document>(collection_name);
        let result = collection
            .update_one(
                filter,
                doc! { "$set": { "lacis_id": lacis_id } },
                None,
            )
            .await
            .map_err(|e| format!("Failed to assign lacis_id: {}", e))?;

        Ok(result.modified_count > 0)
    }
}
