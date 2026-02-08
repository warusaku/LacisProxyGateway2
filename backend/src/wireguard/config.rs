//! WireGuard client configuration file generator

use serde::Deserialize;

/// Parameters for generating a WireGuard client config file
#[derive(Debug, Deserialize)]
pub struct WgClientConfigParams {
    pub private_key: String,
    pub address: String,
    pub dns: String,
    pub server_public_key: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub persistent_keepalive: Option<u32>,
}

/// Generate a WireGuard client configuration string (.conf format)
pub fn generate_config(params: &WgClientConfigParams) -> String {
    let keepalive = params
        .persistent_keepalive
        .map(|k| format!("PersistentKeepalive = {}\n", k))
        .unwrap_or_default();

    format!(
        "[Interface]\n\
         PrivateKey = {}\n\
         Address = {}\n\
         DNS = {}\n\
         \n\
         [Peer]\n\
         PublicKey = {}\n\
         Endpoint = {}\n\
         AllowedIPs = {}\n\
         {}",
        params.private_key,
        params.address,
        params.dns,
        params.server_public_key,
        params.endpoint,
        params.allowed_ips,
        keepalive,
    )
}
