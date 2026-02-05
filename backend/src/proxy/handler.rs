//! Proxy request handler

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
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

    // Find matching route
    let router = state.router.read().await;
    let matched_route = match router.match_route(path) {
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

    // Send request body
    let body = req.into_body();
    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
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

    for (key, value) in response_headers.iter() {
        if is_hop_by_hop_header(key.as_str()) {
            continue;
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
    };

    if let Err(e) = state.app_state.mongo.log_access(&log).await {
        tracing::warn!("Failed to log access: {}", e);
    }
}
