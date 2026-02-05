//! Proxy router - Path matching and route selection

use crate::models::{ProxyRoute, ProxyRouteWithDdns};

/// Proxy router with route matching
pub struct ProxyRouter {
    routes: Vec<ProxyRouteWithDdns>,
}

impl ProxyRouter {
    /// Create a new router with the given routes (with DDNS info)
    pub fn new(mut routes: Vec<ProxyRouteWithDdns>) -> Self {
        // Sort by priority (lower is higher priority)
        routes.sort_by(|a, b| a.route.priority.cmp(&b.route.priority));
        Self { routes }
    }

    /// Create a new router from routes without DDNS info
    pub fn from_routes(routes: Vec<ProxyRoute>) -> Self {
        let routes_with_ddns: Vec<ProxyRouteWithDdns> = routes
            .into_iter()
            .map(|route| ProxyRouteWithDdns {
                route,
                ddns_hostname: None,
            })
            .collect();
        Self::new(routes_with_ddns)
    }

    /// Find a matching route for the given path and host
    /// Routes with ddns_hostname set will only match if the host matches
    /// Routes without ddns_hostname (None) will match any host
    pub fn match_route(&self, path: &str, host: Option<&str>) -> Option<&ProxyRoute> {
        for route_with_ddns in &self.routes {
            // Check if DDNS hostname matches (if set)
            if let Some(ref ddns_hostname) = route_with_ddns.ddns_hostname {
                // This route is DDNS-specific
                if let Some(request_host) = host {
                    // Compare host (strip port if present)
                    let request_host_clean = request_host.split(':').next().unwrap_or(request_host);
                    if !request_host_clean.eq_ignore_ascii_case(ddns_hostname) {
                        continue; // Host doesn't match, skip this route
                    }
                } else {
                    continue; // No host header, skip DDNS-specific routes
                }
            }
            // Route either has no DDNS restriction or host matches
            if self.path_matches(&route_with_ddns.route.path, path) {
                return Some(&route_with_ddns.route);
            }
        }
        None
    }

    /// Check if a route path matches the request path
    fn path_matches(&self, route_path: &str, request_path: &str) -> bool {
        // Exact match
        if route_path == request_path {
            return true;
        }

        // Prefix match (route is a prefix of request path)
        if request_path.starts_with(route_path) {
            // Ensure it's a proper path prefix (followed by / or end)
            let remainder = &request_path[route_path.len()..];
            if remainder.is_empty() || remainder.starts_with('/') {
                return true;
            }
        }

        // Handle trailing slash variations
        let route_normalized = route_path.trim_end_matches('/');
        let request_normalized = request_path.trim_end_matches('/');

        if route_normalized == request_normalized {
            return true;
        }

        if request_normalized.starts_with(route_normalized) {
            let remainder = &request_normalized[route_normalized.len()..];
            if remainder.is_empty() || remainder.starts_with('/') {
                return true;
            }
        }

        false
    }

    /// Build the target URL for a matched route
    pub fn build_target_url(&self, route: &ProxyRoute, request_path: &str) -> String {
        let target = route.target.trim_end_matches('/');

        if route.strip_prefix {
            // Remove the route path prefix from the request path
            let route_path = route.path.trim_end_matches('/');
            let stripped = if request_path.starts_with(route_path) {
                &request_path[route_path.len()..]
            } else {
                request_path
            };

            // Ensure there's a leading slash
            if stripped.is_empty() || !stripped.starts_with('/') {
                format!("{}/{}", target, stripped.trim_start_matches('/'))
            } else {
                format!("{}{}", target, stripped)
            }
        } else {
            // Forward the full path
            format!("{}{}", target, request_path)
        }
    }

    /// Get route count
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Check if router has no routes
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_route(path: &str, target: &str, priority: i32, strip_prefix: bool) -> ProxyRoute {
        ProxyRoute {
            id: 1,
            path: path.to_string(),
            target: target.to_string(),
            ddns_config_id: None,
            priority,
            active: true,
            strip_prefix,
            preserve_host: false,
            timeout_ms: 30000,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_route_with_ddns(
        path: &str,
        target: &str,
        priority: i32,
        ddns_hostname: Option<&str>,
    ) -> ProxyRouteWithDdns {
        ProxyRouteWithDdns {
            route: ProxyRoute {
                id: 1,
                path: path.to_string(),
                target: target.to_string(),
                ddns_config_id: ddns_hostname.map(|_| 1),
                priority,
                active: true,
                strip_prefix: true,
                preserve_host: false,
                timeout_ms: 30000,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            ddns_hostname: ddns_hostname.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_exact_match() {
        let routes = vec![make_route("/eatyui", "http://localhost:3000", 10, true)];
        let router = ProxyRouter::from_routes(routes);

        assert!(router.match_route("/eatyui", None).is_some());
        assert!(router.match_route("/eatyui/", None).is_some());
    }

    #[test]
    fn test_prefix_match() {
        let routes = vec![make_route("/eatyui", "http://localhost:3000", 10, true)];
        let router = ProxyRouter::from_routes(routes);

        assert!(router.match_route("/eatyui/api/test", None).is_some());
        assert!(router.match_route("/eatyui/static/file.js", None).is_some());
    }

    #[test]
    fn test_no_match() {
        let routes = vec![make_route("/eatyui", "http://localhost:3000", 10, true)];
        let router = ProxyRouter::from_routes(routes);

        assert!(router.match_route("/other", None).is_none());
        assert!(router.match_route("/eatyuiother", None).is_none());
    }

    #[test]
    fn test_ddns_specific_route() {
        let routes = vec![
            make_route_with_ddns("/api", "http://api1:8080", 10, Some("domain1.dyndns.org")),
            make_route_with_ddns("/api", "http://api2:8080", 20, Some("domain2.dyndns.org")),
            make_route_with_ddns("/api", "http://default:8080", 100, None), // Fallback
        ];
        let router = ProxyRouter::new(routes);

        // Should match domain1's route
        let matched = router.match_route("/api/test", Some("domain1.dyndns.org")).unwrap();
        assert_eq!(matched.target, "http://api1:8080");

        // Should match domain2's route
        let matched = router.match_route("/api/test", Some("domain2.dyndns.org")).unwrap();
        assert_eq!(matched.target, "http://api2:8080");

        // Unknown host should fall back to non-DDNS route
        let matched = router.match_route("/api/test", Some("unknown.com")).unwrap();
        assert_eq!(matched.target, "http://default:8080");

        // No host should fall back to non-DDNS route
        let matched = router.match_route("/api/test", None).unwrap();
        assert_eq!(matched.target, "http://default:8080");
    }

    #[test]
    fn test_priority() {
        let routes = vec![
            make_route("/api", "http://api:8080", 20, true),
            make_route("/api/v2", "http://api-v2:8080", 10, true),
        ];
        let router = ProxyRouter::from_routes(routes);

        // Should match /api/v2 first due to lower priority number
        let matched = router.match_route("/api/v2/users", None).unwrap();
        assert_eq!(matched.target, "http://api-v2:8080");
    }

    #[test]
    fn test_build_target_url_with_strip() {
        let route = make_route("/eatyui", "http://localhost:3000", 10, true);
        let router = ProxyRouter::from_routes(vec![route.clone()]);

        assert_eq!(
            router.build_target_url(&route, "/eatyui/api/test"),
            "http://localhost:3000/api/test"
        );
        assert_eq!(
            router.build_target_url(&route, "/eatyui"),
            "http://localhost:3000/"
        );
    }

    #[test]
    fn test_build_target_url_without_strip() {
        let route = make_route("/eatyui", "http://localhost:3000", 10, false);
        let router = ProxyRouter::from_routes(vec![route.clone()]);

        assert_eq!(
            router.build_target_url(&route, "/eatyui/api/test"),
            "http://localhost:3000/eatyui/api/test"
        );
    }
}
