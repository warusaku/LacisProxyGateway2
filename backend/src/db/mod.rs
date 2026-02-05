//! Database module - MySQL and MongoDB integration

pub mod mongo;
pub mod mysql;

use std::sync::Arc;

use crate::config::Config;

pub use self::mongo::MongoDb;
pub use self::mysql::MySqlDb;

/// Application state containing database connections
#[derive(Clone)]
pub struct AppState {
    pub mysql: Arc<MySqlDb>,
    pub mongo: Arc<MongoDb>,
    pub start_time: std::time::Instant,
}

impl AppState {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let mysql = MySqlDb::connect(config).await?;
        let mongo = MongoDb::connect(config).await?;

        Ok(Self {
            mysql: Arc::new(mysql),
            mongo: Arc::new(mongo),
            start_time: std::time::Instant::now(),
        })
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
