//! Configuration module

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub discord: Option<DiscordConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub mysql_url: Option<String>,
    pub mongodb_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8081
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::Environment::with_prefix("LACISPROXY").separator("__"))
            .build()?;

        let config: Config = settings.try_deserialize().unwrap_or_else(|_| Config {
            server: ServerConfig {
                host: default_host(),
                port: default_port(),
            },
            database: DatabaseConfig {
                mysql_url: None,
                mongodb_url: None,
            },
            discord: None,
        });

        Ok(config)
    }
}
