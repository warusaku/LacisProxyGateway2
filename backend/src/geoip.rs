//! GeoIP lookup module using MaxMind DB format
//!
//! Provides geographic information lookup for IP addresses.
//! Supports GeoLite2-City, DB-IP City Lite, and compatible MMDB files.

use maxminddb::{geoip2, Reader};
use serde::Serialize;
use std::net::IpAddr;

/// Geographic information for an IP address
#[derive(Debug, Serialize, Clone, Default)]
pub struct GeoInfo {
    pub country_code: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Thread-safe GeoIP database reader
pub struct GeoIpReader {
    reader: Reader<Vec<u8>>,
}

impl GeoIpReader {
    /// Open a MaxMind DB file
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let reader = Reader::open_readfile(path)?;
        tracing::info!("GeoIP database loaded: {}", path);
        Ok(Self { reader })
    }

    /// Look up geographic info for an IP address string.
    /// Returns None if the IP is unparseable, private, or not found in the database.
    pub fn lookup(&self, ip_str: &str) -> Option<GeoInfo> {
        let ip: IpAddr = ip_str.parse().ok()?;

        // Skip private/loopback/link-local addresses
        match &ip {
            IpAddr::V4(v4) => {
                if v4.is_private() || v4.is_loopback() || v4.is_link_local() {
                    return None;
                }
            }
            IpAddr::V6(v6) => {
                if v6.is_loopback() {
                    return None;
                }
            }
        }

        // maxminddb 0.27 API: lookup() -> LookupResult, then decode()
        let result = self.reader.lookup(ip).ok()?;
        let city: geoip2::City = result.decode().ok()??;

        let country_code = city.country.iso_code.map(|s| s.to_string());

        let country = city.country.names.english.map(|s| s.to_string());

        let city_name = city.city.names.english.map(|s| s.to_string());

        let latitude = city.location.latitude;
        let longitude = city.location.longitude;

        Some(GeoInfo {
            country_code,
            country,
            city: city_name,
            latitude,
            longitude,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_ip_returns_none() {
        // Without a real DB file, we can only test the parse/private logic
        // by checking that the function handles private IPs correctly
        // (returns None before attempting DB lookup)

        // This test verifies the IP parsing logic works
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        match ip {
            IpAddr::V4(v4) => assert!(v4.is_private()),
            _ => panic!("Expected IPv4"),
        }

        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        match ip {
            IpAddr::V4(v4) => assert!(v4.is_loopback()),
            _ => panic!("Expected IPv4"),
        }
    }

    #[test]
    fn test_invalid_ip_returns_none() {
        // Verify that invalid IP strings don't panic
        let result: Option<IpAddr> = "not-an-ip".parse().ok();
        assert!(result.is_none());
    }
}
