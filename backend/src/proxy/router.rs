//! Proxy router - Path matching and route selection

use crate::models::ProxyRoute;

/// Proxy router with route matching
pub struct ProxyRouter {
    routes: Vec<ProxyRoute>,
}

impl ProxyRouter {
    /// Create a new router with the given routes
    pub fn new(mut routes: Vec<ProxyRoute>) -> Self {
        // Sort by priority (lower is higher priority)
        routes.sort_by(|a, b| a.priority.cmp(&b.priority));
        Self { routes }
    }

    /// Find a matching route for the given path
    pub fn match_route(&self, path: &str) -> Option<&ProxyRoute> {
        for route in &self.routes {
            if self.path_matches(&route.path, path) {
                return Some(route);
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

    /// Get all routes
    pub fn routes(&self) -> &[ProxyRoute] {
        &self.routes
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
            priority,
            active: true,
            strip_prefix,
            preserve_host: false,
            timeout_ms: 30000,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_exact_match() {
        let routes = vec![make_route("/eatyui", "http://localhost:3000", 10, true)];
        let router = ProxyRouter::new(routes);

        assert!(router.match_route("/eatyui").is_some());
        assert!(router.match_route("/eatyui/").is_some());
    }

    #[test]
    fn test_prefix_match() {
        let routes = vec![make_route("/eatyui", "http://localhost:3000", 10, true)];
        let router = ProxyRouter::new(routes);

        assert!(router.match_route("/eatyui/api/test").is_some());
        assert!(router.match_route("/eatyui/static/file.js").is_some());
    }

    #[test]
    fn test_no_match() {
        let routes = vec![make_route("/eatyui", "http://localhost:3000", 10, true)];
        let router = ProxyRouter::new(routes);

        assert!(router.match_route("/other").is_none());
        assert!(router.match_route("/eatyuiother").is_none());
    }

    #[test]
    fn test_priority() {
        let routes = vec![
            make_route("/api", "http://api:8080", 20, true),
            make_route("/api/v2", "http://api-v2:8080", 10, true),
        ];
        let router = ProxyRouter::new(routes);

        // Should match /api/v2 first due to lower priority number
        let matched = router.match_route("/api/v2/users").unwrap();
        assert_eq!(matched.target, "http://api-v2:8080");
    }

    #[test]
    fn test_build_target_url_with_strip() {
        let route = make_route("/eatyui", "http://localhost:3000", 10, true);
        let router = ProxyRouter::new(vec![route.clone()]);

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
        let router = ProxyRouter::new(vec![route.clone()]);

        assert_eq!(
            router.build_target_url(&route, "/eatyui/api/test"),
            "http://localhost:3000/eatyui/api/test"
        );
    }
}
