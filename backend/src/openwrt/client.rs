//! SSH Router Client for OpenWrt and AsusWrt routers
//!
//! Uses `tokio::process::Command` with `sshpass` for password-based SSH.
//! Command mapping based on is10 (aranea_ISMS) patterns.

use std::time::Duration;
use tokio::process::Command;

// ============================================================================
// Types
// ============================================================================

/// Router firmware type
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RouterFirmware {
    OpenWrt,
    AsusWrt,
}

impl RouterFirmware {
    pub fn as_str(&self) -> &'static str {
        match self {
            RouterFirmware::OpenWrt => "openwrt",
            RouterFirmware::AsusWrt => "asuswrt",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "asuswrt" => RouterFirmware::AsusWrt,
            _ => RouterFirmware::OpenWrt,
        }
    }
}

/// Router status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct RouterStatus {
    pub online: bool,
    pub wan_ip: Option<String>,
    pub lan_ip: Option<String>,
    pub ssid_24g: Option<String>,
    pub ssid_5g: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub client_count: u32,
    pub firmware_version: Option<String>,
}

/// A single client entry from router DHCP/ARP table
#[derive(Debug, Clone, serde::Serialize)]
pub struct RouterClientEntry {
    pub mac: String,
    pub ip: String,
    pub hostname: Option<String>,
}

// ============================================================================
// SSH Router Client
// ============================================================================

/// SSH client for communicating with OpenWrt/AsusWrt routers
pub struct SshRouterClient {
    pub ip: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub firmware: RouterFirmware,
}

impl SshRouterClient {
    pub fn new(
        ip: String,
        port: u16,
        username: String,
        password: String,
        firmware: RouterFirmware,
    ) -> Self {
        Self {
            ip,
            port,
            username,
            password,
            firmware,
        }
    }

    /// Execute a command via SSH
    async fn ssh_exec(&self, command: &str) -> Result<String, String> {
        let output = Command::new("sshpass")
            .arg("-p")
            .arg(&self.password)
            .arg("ssh")
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg("-o")
            .arg("ConnectTimeout=5")
            .arg("-o")
            .arg("UserKnownHostsFile=/dev/null")
            .arg("-o")
            .arg("LogLevel=ERROR")
            .arg("-p")
            .arg(self.port.to_string())
            .arg(format!("{}@{}", self.username, self.ip))
            .arg(command)
            .output()
            .await
            .map_err(|e| format!("SSH exec failed: {} (is sshpass installed?)", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("SSH command failed: {}", stderr.trim()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Test SSH connection
    pub async fn test_connection(&self) -> Result<bool, String> {
        // Simple echo test with timeout
        let result = tokio::time::timeout(Duration::from_secs(10), self.ssh_exec("echo ok")).await;

        match result {
            Ok(Ok(output)) => Ok(output.contains("ok")),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("SSH connection timeout".to_string()),
        }
    }

    /// Get router status
    pub async fn get_status(&self) -> Result<RouterStatus, String> {
        match self.firmware {
            RouterFirmware::OpenWrt => self.get_status_openwrt().await,
            RouterFirmware::AsusWrt => self.get_status_asuswrt().await,
        }
    }

    /// Get connected clients from router
    pub async fn get_clients(&self) -> Result<Vec<RouterClientEntry>, String> {
        match self.firmware {
            RouterFirmware::OpenWrt => self.get_clients_openwrt().await,
            RouterFirmware::AsusWrt => self.get_clients_asuswrt().await,
        }
    }

    // ========================================================================
    // OpenWrt commands (based on is10 ESP32 implementation)
    // ========================================================================

    async fn get_status_openwrt(&self) -> Result<RouterStatus, String> {
        // Run multiple commands in one SSH session for efficiency
        let combined = self
            .ssh_exec(concat!(
                "echo '---WAN---'; ",
                "(uci get network.wan.ipaddr 2>/dev/null || ip -4 addr show eth0.2 2>/dev/null | grep -oP 'inet \\K[\\d.]+' || echo ''); ",
                "echo '---LAN---'; ",
                "(uci get network.lan.ipaddr 2>/dev/null || ifconfig br-lan 2>/dev/null | grep -oP 'inet addr:\\K[\\d.]+' || echo ''); ",
                "echo '---SSID24---'; ",
                "(uci get wireless.@wifi-iface[0].ssid 2>/dev/null || echo ''); ",
                "echo '---SSID5---'; ",
                "(uci get wireless.@wifi-iface[1].ssid 2>/dev/null || echo ''); ",
                "echo '---UPTIME---'; ",
                "cat /proc/uptime 2>/dev/null || echo ''; ",
                "echo '---FW---'; ",
                "(. /etc/openwrt_release 2>/dev/null && echo $DISTRIB_RELEASE || echo ''); ",
                "echo '---CLIENTS---'; ",
                "cat /tmp/dhcp.leases 2>/dev/null | wc -l || echo '0'"
            ))
            .await?;

        let wan_ip = Self::extract_section(&combined, "---WAN---", "---LAN---");
        let lan_ip = Self::extract_section(&combined, "---LAN---", "---SSID24---");
        let ssid_24g = Self::extract_section(&combined, "---SSID24---", "---SSID5---");
        let ssid_5g = Self::extract_section(&combined, "---SSID5---", "---UPTIME---");
        let uptime_raw = Self::extract_section(&combined, "---UPTIME---", "---FW---");
        let firmware = Self::extract_section(&combined, "---FW---", "---CLIENTS---");
        let client_count_raw = Self::extract_after(&combined, "---CLIENTS---");

        let uptime_seconds = uptime_raw
            .and_then(|s| s.split_whitespace().next().map(String::from))
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as u64);

        let client_count = client_count_raw
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);

        Ok(RouterStatus {
            online: true,
            wan_ip,
            lan_ip,
            ssid_24g,
            ssid_5g,
            uptime_seconds,
            client_count,
            firmware_version: firmware,
        })
    }

    async fn get_clients_openwrt(&self) -> Result<Vec<RouterClientEntry>, String> {
        // OpenWrt DHCP leases format: timestamp mac ip hostname *
        let output = self
            .ssh_exec("cat /tmp/dhcp.leases 2>/dev/null || echo ''")
            .await?;
        let mut clients = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let mac = crate::omada::client::normalize_mac(parts[1]);
                let ip = parts[2].to_string();
                let hostname = if parts[3] != "*" {
                    Some(parts[3].to_string())
                } else {
                    None
                };

                clients.push(RouterClientEntry { mac, ip, hostname });
            }
        }

        Ok(clients)
    }

    // ========================================================================
    // AsusWrt commands (based on is10 ESP32 implementation)
    // ========================================================================

    async fn get_status_asuswrt(&self) -> Result<RouterStatus, String> {
        let combined = self
            .ssh_exec(concat!(
                "echo '---WAN---'; ",
                "nvram get wan0_ipaddr 2>/dev/null || echo ''; ",
                "echo '---LAN---'; ",
                "nvram get lan_ipaddr 2>/dev/null || echo ''; ",
                "echo '---SSID24---'; ",
                "nvram get wl0_ssid 2>/dev/null || echo ''; ",
                "echo '---SSID5---'; ",
                "nvram get wl1_ssid 2>/dev/null || echo ''; ",
                "echo '---UPTIME---'; ",
                "cat /proc/uptime 2>/dev/null || echo ''; ",
                "echo '---FW---'; ",
                "(echo $(nvram get firmver 2>/dev/null).$(nvram get buildno 2>/dev/null) || echo ''); ",
                "echo '---CLIENTS---'; ",
                "cat /proc/net/arp 2>/dev/null | grep -v 'IP address' | wc -l || echo '0'"
            ))
            .await?;

        let wan_ip = Self::extract_section(&combined, "---WAN---", "---LAN---");
        let lan_ip = Self::extract_section(&combined, "---LAN---", "---SSID24---");
        let ssid_24g = Self::extract_section(&combined, "---SSID24---", "---SSID5---");
        let ssid_5g = Self::extract_section(&combined, "---SSID5---", "---UPTIME---");
        let uptime_raw = Self::extract_section(&combined, "---UPTIME---", "---FW---");
        let firmware = Self::extract_section(&combined, "---FW---", "---CLIENTS---");
        let client_count_raw = Self::extract_after(&combined, "---CLIENTS---");

        let uptime_seconds = uptime_raw
            .and_then(|s| s.split_whitespace().next().map(String::from))
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as u64);

        let client_count = client_count_raw
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);

        Ok(RouterStatus {
            online: true,
            wan_ip,
            lan_ip,
            ssid_24g,
            ssid_5g,
            uptime_seconds,
            client_count,
            firmware_version: firmware,
        })
    }

    async fn get_clients_asuswrt(&self) -> Result<Vec<RouterClientEntry>, String> {
        // ARP table format: IP HWtype Flags HWaddress Mask Device
        let output = self
            .ssh_exec("cat /proc/net/arp 2>/dev/null | grep -v 'IP address' || echo ''")
            .await?;
        let mut clients = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let ip = parts[0].to_string();
                let mac = crate::omada::client::normalize_mac(parts[3]);
                // ARP table doesn't include hostnames
                clients.push(RouterClientEntry {
                    mac,
                    ip,
                    hostname: None,
                });
            }
        }

        Ok(clients)
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    /// Extract text between two markers
    fn extract_section(text: &str, start: &str, end: &str) -> Option<String> {
        let start_pos = text.find(start)? + start.len();
        let end_pos = text[start_pos..].find(end)? + start_pos;
        let value = text[start_pos..end_pos].trim().to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }

    /// Extract text after a marker (to end of string)
    fn extract_after(text: &str, marker: &str) -> Option<String> {
        let pos = text.find(marker)? + marker.len();
        let value = text[pos..].trim().to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }
}
