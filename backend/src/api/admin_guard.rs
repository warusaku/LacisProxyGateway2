//! Admin access guard - restricts API access to private networks only
//!
//! Allows: 192.168.0.0/16, 10.0.0.0/8, 172.16.0.0/12, 127.0.0.0/8, ::1
//! Denies: all other (internet) addresses with 403 Forbidden
//!
//! Future: replace with Firebase Authentication + 2FA for internet access

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Middleware that rejects requests from non-private networks.
pub async fn require_private_network(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let client_ip_str = extract_client_ip(req.headers(), addr);

    if is_private_network(&client_ip_str) {
        next.run(req).await
    } else {
        tracing::warn!(
            "Admin API access denied from external IP: {} (path: {})",
            client_ip_str,
            req.uri().path()
        );
        (StatusCode::FORBIDDEN, "Admin access denied from external network").into_response()
    }
}

/// Check if an IP address belongs to a private/local network (RFC 1918 + loopback)
fn is_private_network(ip_str: &str) -> bool {
    match ip_str.parse::<IpAddr>() {
        Ok(IpAddr::V4(ipv4)) => {
            ipv4.is_private()       // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                || ipv4.is_loopback()   // 127.0.0.0/8
                || ipv4 == Ipv4Addr::UNSPECIFIED // 0.0.0.0
        }
        Ok(IpAddr::V6(ipv6)) => {
            ipv6.is_loopback() // ::1
        }
        Err(_) => false,
    }
}

/// Extract client IP from headers or connection (same logic as proxy handler)
fn extract_client_ip(headers: &HeaderMap, addr: SocketAddr) -> String {
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(s) = xff.to_str() {
            if let Some(ip) = s.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(s) = xri.to_str() {
            return s.to_string();
        }
    }

    addr.ip().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_networks_allowed() {
        // 192.168.x.x
        assert!(is_private_network("192.168.1.1"));
        assert!(is_private_network("192.168.96.201"));
        assert!(is_private_network("192.168.125.246"));

        // 10.x.x.x
        assert!(is_private_network("10.0.0.1"));
        assert!(is_private_network("10.255.255.255"));

        // 172.16-31.x.x
        assert!(is_private_network("172.16.0.1"));
        assert!(is_private_network("172.31.255.255"));

        // Loopback
        assert!(is_private_network("127.0.0.1"));

        // IPv6 loopback
        assert!(is_private_network("::1"));
    }

    #[test]
    fn test_public_networks_denied() {
        assert!(!is_private_network("8.8.8.8"));
        assert!(!is_private_network("203.0.113.1"));
        assert!(!is_private_network("1.1.1.1"));
        assert!(!is_private_network("172.32.0.1")); // outside 172.16-31 range
        assert!(!is_private_network("11.0.0.1"));   // not 10.x
    }

    #[test]
    fn test_invalid_ip_denied() {
        assert!(!is_private_network("not-an-ip"));
        assert!(!is_private_network(""));
    }
}
