//! Configuration module

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub discord: Option<DiscordConfig>,
    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub geoip_db_path: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_session_hours")]
    pub session_duration_hours: u64,
    #[serde(default = "default_local_email")]
    pub local_email: String,
    #[serde(default)]
    pub local_password_hash: String,
    #[serde(default = "default_required_permission")]
    pub lacisoath_required_permission: i32,
    #[serde(default = "default_required_fid")]
    pub lacisoath_required_fid: String,
    #[serde(default)]
    pub internet_access_enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: default_jwt_secret(),
            session_duration_hours: default_session_hours(),
            local_email: default_local_email(),
            local_password_hash: String::new(),
            lacisoath_required_permission: default_required_permission(),
            lacisoath_required_fid: default_required_fid(),
            internet_access_enabled: false,
        }
    }
}

fn default_jwt_secret() -> String {
    "CHANGE_ME_IN_PRODUCTION".to_string()
}

fn default_session_hours() -> u64 {
    24
}

fn default_local_email() -> String {
    "webadmin@mijeos.com".to_string()
}

fn default_required_permission() -> i32 {
    80
}

fn default_required_fid() -> String {
    "9966".to_string()
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
                geoip_db_path: None,
            },
            database: DatabaseConfig {
                mysql_url: None,
                mongodb_url: None,
            },
            discord: None,
            auth: AuthConfig::default(),
        });

        Ok(config)
    }
}
