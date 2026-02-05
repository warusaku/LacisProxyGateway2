//! MongoDB database module

mod access_log;
mod security_events;

use mongodb::{Client, Database};

use crate::config::Config;

pub use self::access_log::*;
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
}
