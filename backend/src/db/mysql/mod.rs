//! MySQL database module

mod blocked_ips;
mod ddns;
mod routes;
mod settings;

use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

use crate::config::Config;

pub use self::blocked_ips::*;
pub use self::ddns::*;
pub use self::routes::*;
pub use self::settings::*;

/// MySQL database wrapper
#[derive(Clone)]
pub struct MySqlDb {
    pool: MySqlPool,
}

impl MySqlDb {
    /// Connect to MySQL database
    pub async fn connect(config: &Config) -> anyhow::Result<Self> {
        let url = config
            .database
            .mysql_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MySQL URL not configured"))?;

        tracing::info!("Connecting to MySQL...");

        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .connect(url)
            .await?;

        tracing::info!("MySQL connected successfully");

        Ok(Self { pool })
    }

    /// Get the connection pool
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}
