//! WebSocket proxy handler - bidirectional WebSocket relay

use axum::{
    extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use std::time::Instant;
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};

use super::ProxyState;
use crate::models::{AccessLog, ProxyRoute};

/// Handle WebSocket upgrade request
///
/// Receives the WebSocketUpgrade extractor and proxies the connection
/// to the upstream WebSocket server.
pub async fn handle_websocket_upgrade(
    ws: WebSocketUpgrade,
    state: ProxyState,
    route: ProxyRoute,
    target_url: String,
    client_ip: String,
    path: String,
    user_agent: Option<String>,
    referer: Option<String>,
) -> Response {
    let ws_url = match http_to_ws_url(&target_url) {
        Some(url) => url,
        None => {
            tracing::error!("Failed to convert target URL to WebSocket URL: {}", target_url);
            return (StatusCode::BAD_GATEWAY, "Invalid upstream WebSocket URL").into_response();
        }
    };

    let timeout_ms = route.timeout_ms as u64;
    let route_id = route.id;
    let route_target = route.target.clone();

    ws.on_upgrade(move |socket| {
        websocket_bridge(
            socket, ws_url, state, route_id, route_target, timeout_ms, client_ip, path,
            user_agent, referer,
        )
    })
}

/// Bidirectional WebSocket bridge between client and upstream
async fn websocket_bridge(
    client_socket: WebSocket,
    ws_url: String,
    state: ProxyState,
    route_id: i32,
    route_target: String,
    timeout_ms: u64,
    client_ip: String,
    path: String,
    user_agent: Option<String>,
    referer: Option<String>,
) {
    let start_time = Instant::now();

    // Connect to upstream WebSocket with timeout
    let connect_timeout = std::time::Duration::from_millis(timeout_ms);
    let upstream_result = tokio::time::timeout(connect_timeout, connect_async(&ws_url)).await;

    let upstream_socket = match upstream_result {
        Ok(Ok((stream, _response))) => {
            tracing::info!(
                "WebSocket upstream connected: {} -> {}",
                path,
                ws_url
            );
            stream
        }
        Ok(Err(e)) => {
            tracing::error!("WebSocket upstream connection failed: {} -> {}: {}", path, ws_url, e);
            log_ws_access(
                &state,
                &client_ip,
                &path,
                Some(route_id),
                Some(&route_target),
                502,
                start_time.elapsed().as_millis() as i32,
                user_agent.as_deref(),
                referer.as_deref(),
            )
            .await;
            return;
        }
        Err(_) => {
            tracing::error!("WebSocket upstream connection timed out: {} -> {}", path, ws_url);
            log_ws_access(
                &state,
                &client_ip,
                &path,
                Some(route_id),
                Some(&route_target),
                504,
                start_time.elapsed().as_millis() as i32,
                user_agent.as_deref(),
                referer.as_deref(),
            )
            .await;
            return;
        }
    };

    // Log successful WebSocket upgrade (101 Switching Protocols)
    log_ws_access(
        &state,
        &client_ip,
        &path,
        Some(route_id),
        Some(&route_target),
        101,
        start_time.elapsed().as_millis() as i32,
        user_agent.as_deref(),
        referer.as_deref(),
    )
    .await;

    // Split both sockets for bidirectional relay
    let (mut client_sink, mut client_stream) = client_socket.split();
    let (mut upstream_sink, mut upstream_stream) = upstream_socket.split();

    // Bidirectional message relay using tokio::select!
    loop {
        tokio::select! {
            // Client -> Upstream
            client_msg = client_stream.next() => {
                match client_msg {
                    Some(Ok(msg)) => {
                        if let Some(tung_msg) = axum_to_tungstenite(msg) {
                            if tung_msg.is_close() {
                                let _ = upstream_sink.send(tung_msg).await;
                                break;
                            }
                            if let Err(e) = upstream_sink.send(tung_msg).await {
                                tracing::debug!("WebSocket upstream send error: {}", e);
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::debug!("WebSocket client read error: {}", e);
                        break;
                    }
                    None => {
                        // Client disconnected
                        break;
                    }
                }
            }
            // Upstream -> Client
            upstream_msg = upstream_stream.next() => {
                match upstream_msg {
                    Some(Ok(msg)) => {
                        if let Some(axum_msg) = tungstenite_to_axum(msg) {
                            if matches!(&axum_msg, AxumMessage::Close(_)) {
                                let _ = client_sink.send(axum_msg).await;
                                break;
                            }
                            if let Err(e) = client_sink.send(axum_msg).await {
                                tracing::debug!("WebSocket client send error: {}", e);
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::debug!("WebSocket upstream read error: {}", e);
                        break;
                    }
                    None => {
                        // Upstream disconnected
                        break;
                    }
                }
            }
        }
    }

    let session_duration_ms = start_time.elapsed().as_millis() as i32;
    tracing::info!(
        "WebSocket session ended: {} -> {} (duration: {}ms)",
        path,
        ws_url,
        session_duration_ms
    );
}

/// Convert HTTP/HTTPS URL to WS/WSS URL
fn http_to_ws_url(url: &str) -> Option<String> {
    if let Some(rest) = url.strip_prefix("https://") {
        Some(format!("wss://{}", rest))
    } else if let Some(rest) = url.strip_prefix("http://") {
        Some(format!("ws://{}", rest))
    } else if url.starts_with("ws://") || url.starts_with("wss://") {
        Some(url.to_string())
    } else {
        None
    }
}

/// Convert axum WebSocket message to tungstenite message
fn axum_to_tungstenite(msg: AxumMessage) -> Option<TungsteniteMessage> {
    match msg {
        AxumMessage::Text(text) => Some(TungsteniteMessage::Text(text.to_string())),
        AxumMessage::Binary(data) => Some(TungsteniteMessage::Binary(data.to_vec())),
        AxumMessage::Ping(data) => Some(TungsteniteMessage::Ping(data.to_vec())),
        AxumMessage::Pong(data) => Some(TungsteniteMessage::Pong(data.to_vec())),
        AxumMessage::Close(frame) => {
            let close_frame = frame.map(|f| tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(f.code),
                reason: f.reason.to_string().into(),
            });
            Some(TungsteniteMessage::Close(close_frame))
        }
    }
}

/// Convert tungstenite message to axum WebSocket message
fn tungstenite_to_axum(msg: TungsteniteMessage) -> Option<AxumMessage> {
    match msg {
        TungsteniteMessage::Text(text) => Some(AxumMessage::Text(text.to_string().into())),
        TungsteniteMessage::Binary(data) => Some(AxumMessage::Binary(data.to_vec().into())),
        TungsteniteMessage::Ping(data) => Some(AxumMessage::Ping(data.to_vec().into())),
        TungsteniteMessage::Pong(data) => Some(AxumMessage::Pong(data.to_vec().into())),
        TungsteniteMessage::Close(frame) => {
            let close_frame = frame.map(|f| axum::extract::ws::CloseFrame {
                code: f.code.into(),
                reason: f.reason.to_string().into(),
            });
            Some(AxumMessage::Close(close_frame))
        }
        TungsteniteMessage::Frame(_) => None,
    }
}

/// Log WebSocket access to MongoDB (reuses existing AccessLog model)
async fn log_ws_access(
    state: &ProxyState,
    ip: &str,
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
        method: "WS".to_string(),
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
        tracing::warn!("Failed to log WebSocket access: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_to_ws_url() {
        assert_eq!(
            http_to_ws_url("http://localhost:8080/ws"),
            Some("ws://localhost:8080/ws".to_string())
        );
        assert_eq!(
            http_to_ws_url("https://example.com/ws"),
            Some("wss://example.com/ws".to_string())
        );
        assert_eq!(
            http_to_ws_url("ws://already:8080/ws"),
            Some("ws://already:8080/ws".to_string())
        );
        assert_eq!(
            http_to_ws_url("wss://already:8080/ws"),
            Some("wss://already:8080/ws".to_string())
        );
        assert_eq!(http_to_ws_url("ftp://invalid"), None);
    }
}
