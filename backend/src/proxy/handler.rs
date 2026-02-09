//! Proxy request handler

use axum::{
    body::Body,
    extract::ws::WebSocketUpgrade,
    extract::{ConnectInfo, FromRequest, Request, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use std::net::SocketAddr;
use std::time::Instant;

use super::ProxyState;
use crate::models::AccessLog;

/// Main proxy handler
pub async fn proxy_handler(
    State(state): State<ProxyState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
) -> Response {
    let start_time = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let path = uri.path();
    let client_ip = extract_client_ip(&headers, addr);

    // Check if IP is blocked
    if let Ok(blocked) = state.app_state.mysql.is_ip_blocked(&client_ip).await {
        if blocked {
            tracing::warn!("Blocked IP attempted access: {}", client_ip);
            return (StatusCode::FORBIDDEN, "Access denied").into_response();
        }
    }

    // Get host header for DDNS-based routing
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok());

    // Find matching route (considering host for DDNS routing)
    let router = state.router.read().await;
    let matched_route = match router.match_route(path, host) {
        Some(route) => route.clone(),
        None => {
            drop(router);
            // Log 404 for unmatched routes
            log_access(
                &state,
                &client_ip,
                method.as_str(),
                path,
                None,
                None,
                404,
                start_time.elapsed().as_millis() as i32,
                headers
                    .get(header::USER_AGENT)
                    .and_then(|v| v.to_str().ok()),
                headers.get(header::REFERER).and_then(|v| v.to_str().ok()),
            )
            .await;
            return (StatusCode::NOT_FOUND, "No route found").into_response();
        }
    };

    // Build target URL
    let target_url = router.build_target_url(&matched_route, path);
    let query_string = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let full_url = format!("{}{}", target_url, query_string);
    drop(router);

    tracing::debug!("Proxying {} {} -> {}", method, path, full_url);

    // WebSocket upgrade detection
    let is_websocket = headers
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    if is_websocket && matched_route.websocket_support {
        // Extract WebSocketUpgrade from the request
        let user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let referer = headers
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        match WebSocketUpgrade::from_request(req, &()).await {
            Ok(ws) => {
                return super::ws_handler::handle_websocket_upgrade(
                    ws,
                    state,
                    matched_route,
                    full_url,
                    client_ip,
                    path.to_string(),
                    user_agent,
                    referer,
                )
                .await;
            }
            Err(e) => {
                tracing::error!("WebSocket upgrade extraction failed: {}", e);
                return (StatusCode::BAD_REQUEST, "WebSocket upgrade failed").into_response();
            }
        }
    }

    if is_websocket && !matched_route.websocket_support {
        return (
            StatusCode::BAD_REQUEST,
            "WebSocket not supported on this route",
        )
            .into_response();
    }

    // Build upstream request
    let mut request_builder = state
        .http_client
        .request(convert_method(&method), &full_url);

    // Forward headers
    for (key, value) in headers.iter() {
        // Skip hop-by-hop headers
        if is_hop_by_hop_header(key.as_str()) {
            continue;
        }

        // Handle Host header
        if key == header::HOST {
            if matched_route.preserve_host {
                if let Ok(s) = value.to_str() {
                    request_builder = request_builder.header(key.as_str(), s);
                }
            }
            continue;
        }

        if let Ok(s) = value.to_str() {
            request_builder = request_builder.header(key.as_str(), s);
        }
    }

    // Add X-Forwarded-For header
    let xff = if let Some(existing) = headers.get("x-forwarded-for") {
        format!("{}, {}", existing.to_str().unwrap_or(""), client_ip)
    } else {
        client_ip.clone()
    };
    request_builder = request_builder.header("X-Forwarded-For", &xff);
    request_builder = request_builder.header("X-Real-IP", &client_ip);

    // Add X-Forwarded-Proto
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    request_builder = request_builder.header("X-Forwarded-Proto", proto);

    // Set timeout
    let timeout = std::time::Duration::from_millis(matched_route.timeout_ms as u64);
    request_builder = request_builder.timeout(timeout);

    // Send request body (100MB limit)
    let body = req.into_body();
    let body_bytes = match axum::body::to_bytes(body, 100 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };

    if !body_bytes.is_empty() {
        request_builder = request_builder.body(body_bytes.to_vec());
    }

    // Execute request
    let response = match request_builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Proxy request failed: {} -> {}: {}", path, full_url, e);

            let status = if e.is_timeout() {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::BAD_GATEWAY
            };

            log_access(
                &state,
                &client_ip,
                method.as_str(),
                path,
                Some(matched_route.id),
                Some(&matched_route.target),
                status.as_u16() as i32,
                start_time.elapsed().as_millis() as i32,
                headers
                    .get(header::USER_AGENT)
                    .and_then(|v| v.to_str().ok()),
                headers.get(header::REFERER).and_then(|v| v.to_str().ok()),
            )
            .await;

            return (status, format!("Upstream error: {}", e)).into_response();
        }
    };

    let upstream_status = response.status();
    let response_headers = response.headers().clone();

    // Read response body
    let response_body = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to read upstream response: {}", e);
            return (StatusCode::BAD_GATEWAY, "Failed to read upstream response").into_response();
        }
    };

    let elapsed_ms = start_time.elapsed().as_millis() as i32;

    // Log access
    log_access(
        &state,
        &client_ip,
        method.as_str(),
        path,
        Some(matched_route.id),
        Some(&matched_route.target),
        upstream_status.as_u16() as i32,
        elapsed_ms,
        headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok()),
        headers.get(header::REFERER).and_then(|v| v.to_str().ok()),
    )
    .await;

    // Build response - convert reqwest StatusCode to axum StatusCode
    let axum_status =
        StatusCode::from_u16(upstream_status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = Response::builder().status(axum_status);

    // Determine original path prefix for Location header rewriting
    let original_prefix = if matched_route.strip_prefix {
        Some(matched_route.path.trim_end_matches('/').to_string())
    } else {
        None
    };

    // Get the scheme and host for building absolute URLs
    let request_scheme = proto;
    let request_host = host.unwrap_or("");

    for (key, value) in response_headers.iter() {
        if is_hop_by_hop_header(key.as_str()) {
            continue;
        }

        // Rewrite Location header for redirects
        if key.as_str().eq_ignore_ascii_case("location") {
            if let Ok(location_str) = value.to_str() {
                let rewritten = rewrite_location_header(
                    location_str,
                    &matched_route.target,
                    original_prefix.as_deref(),
                    request_scheme,
                    request_host,
                );
                if let Ok(v) = axum::http::HeaderValue::from_str(&rewritten) {
                    builder = builder.header(key.as_str(), v);
                }
                continue;
            }
        }

        if let Ok(v) = axum::http::HeaderValue::from_bytes(value.as_bytes()) {
            builder = builder.header(key.as_str(), v);
        }
    }

    builder.body(Body::from(response_body)).unwrap_or_else(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "Response build failed").into_response()
    })
}

/// Extract client IP from headers or connection
fn extract_client_ip(headers: &HeaderMap, addr: SocketAddr) -> String {
    // Check X-Forwarded-For first
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(s) = xff.to_str() {
            if let Some(ip) = s.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP
    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(s) = xri.to_str() {
            return s.to_string();
        }
    }

    // Fall back to connection address
    addr.ip().to_string()
}

/// Convert axum Method to reqwest Method
fn convert_method(method: &axum::http::Method) -> reqwest::Method {
    match method.as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        "TRACE" => reqwest::Method::TRACE,
        "CONNECT" => reqwest::Method::CONNECT,
        _ => reqwest::Method::GET,
    }
}

/// Check if a header is hop-by-hop
fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

/// Rewrite Location header for redirect responses
///
/// When a backend returns a redirect to an absolute path (e.g., `/login`),
/// we need to prepend the original route prefix (e.g., `/paraclate/login`)
/// so the client is redirected to the correct proxied path.
fn rewrite_location_header(
    location: &str,
    target_base: &str,
    original_prefix: Option<&str>,
    request_scheme: &str,
    request_host: &str,
) -> String {
    // Parse the target base URL to extract host/scheme
    let target_parsed = url::Url::parse(target_base);

    // Case 1: Absolute URL (starts with http:// or https://)
    if location.starts_with("http://") || location.starts_with("https://") {
        if let Ok(loc_url) = url::Url::parse(location) {
            // Check if this URL points to our backend
            if let Ok(ref target_url) = target_parsed {
                if loc_url.host_str() == target_url.host_str() {
                    // Rewrite to point to our proxy
                    let path = loc_url.path();
                    let query = loc_url
                        .query()
                        .map(|q| format!("?{}", q))
                        .unwrap_or_default();

                    let new_path = if let Some(prefix) = original_prefix {
                        format!("{}{}{}", prefix, path, query)
                    } else {
                        format!("{}{}", path, query)
                    };

                    if !request_host.is_empty() {
                        return format!("{}://{}{}", request_scheme, request_host, new_path);
                    } else {
                        return new_path;
                    }
                }
            }
        }
        // Not pointing to our backend, pass through unchanged
        return location.to_string();
    }

    // Case 2: Absolute path (starts with /)
    if location.starts_with('/') {
        if let Some(prefix) = original_prefix {
            // Don't add prefix if location already starts with it
            if location.starts_with(prefix) {
                return location.to_string();
            }
            return format!("{}{}", prefix, location);
        }
        return location.to_string();
    }

    // Case 3: Relative path - pass through unchanged
    location.to_string()
}

/// Log access to MongoDB
async fn log_access(
    state: &ProxyState,
    ip: &str,
    method: &str,
    path: &str,
    route_id: Option<i32>,
    target: Option<&str>,
    status: i32,
    response_time_ms: i32,
    user_agent: Option<&str>,
    referer: Option<&str>,
) {
    // GeoIP lookup (non-blocking, memory-mapped read)
    let geo = state.geoip.as_ref().and_then(|reader| reader.lookup(ip));

    let log = AccessLog {
        timestamp: Utc::now(),
        ip: ip.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        route_id,
        target: target.map(|s| s.to_string()),
        status,
        response_time_ms,
        request_size: None,
        response_size: None,
        user_agent: user_agent.map(|s| s.to_string()),
        referer: referer.map(|s| s.to_string()),
        country_code: geo.as_ref().and_then(|g| g.country_code.clone()),
        country: geo.as_ref().and_then(|g| g.country.clone()),
        city: geo.as_ref().and_then(|g| g.city.clone()),
        latitude: geo.as_ref().and_then(|g| g.latitude),
        longitude: geo.as_ref().and_then(|g| g.longitude),
    };

    if let Err(e) = state.app_state.mongo.log_access(&log).await {
        tracing::warn!("Failed to log access: {}", e);
    }
}
