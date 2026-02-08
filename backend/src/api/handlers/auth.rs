//! Authentication handlers
//!
//! - POST /api/auth/login/local    - Local email+password login
//! - POST /api/auth/login/lacisoath - LacisOath JWT token login
//! - GET  /api/auth/me              - Get current authenticated user
//! - POST /api/auth/logout           - Clear session cookie

use axum::{
    extract::State,
    http::{header::SET_COOKIE, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use base64::Engine;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Deserialize;

use crate::error::AppError;
use crate::models::{
    AuthResponse, AuthUser, LacisOathLoginRequest, LocalLoginRequest, SessionClaims,
};
use crate::proxy::ProxyState;

/// POST /api/auth/login/local
/// Authenticate with local email + password (bcrypt)
pub async fn login_local(
    State(state): State<ProxyState>,
    Json(req): Json<LocalLoginRequest>,
) -> Result<Response, AppError> {
    let auth = &state.auth_config;

    // Verify email
    if req.email != auth.local_email {
        tracing::warn!("Local login failed: unknown email {}", req.email);
        return Err(AppError::BadRequest(
            "Invalid email or password".to_string(),
        ));
    }

    // Verify password hash exists
    if auth.local_password_hash.is_empty() {
        tracing::error!("Local login failed: local_password_hash is not configured");
        return Err(AppError::InternalError(
            "Local authentication is not configured".to_string(),
        ));
    }

    // Verify password with bcrypt
    let valid = bcrypt::verify(&req.password, &auth.local_password_hash)
        .map_err(|e| AppError::InternalError(format!("Password verification error: {}", e)))?;

    if !valid {
        tracing::warn!("Local login failed: wrong password for {}", req.email);
        return Err(AppError::BadRequest(
            "Invalid email or password".to_string(),
        ));
    }

    // Create session
    let user = AuthUser {
        sub: req.email.clone(),
        lacis_id: None,
        permission: 100, // local admin gets max permission
        auth_method: "local".to_string(),
    };

    let cookie = create_session_cookie(&user, auth)?;
    let body = AuthResponse {
        ok: true,
        user: user.clone(),
    };

    Ok((StatusCode::OK, [(SET_COOKIE, cookie)], Json(body)).into_response())
}

/// POST /api/auth/login/lacisoath
/// Authenticate with LacisOath JWT token (payload decode only, no RS256 verification)
pub async fn login_lacisoath(
    State(state): State<ProxyState>,
    Json(req): Json<LacisOathLoginRequest>,
) -> Result<Response, AppError> {
    let auth = &state.auth_config;

    // Decode JWT payload (base64) without signature verification
    let payload = decode_jwt_payload(&req.token)?;

    // Check expiration
    if let Some(exp) = payload.exp {
        let now = chrono::Utc::now().timestamp() as u64;
        if exp < now {
            return Err(AppError::BadRequest("Token has expired".to_string()));
        }
    }

    // Check permission
    let permission = payload.permission.unwrap_or(0);
    if permission < auth.lacisoath_required_permission {
        tracing::warn!(
            "LacisOath login denied: permission {} < required {}",
            permission,
            auth.lacisoath_required_permission
        );
        return Err(AppError::BadRequest(format!(
            "Insufficient permission: {} (required: {})",
            permission, auth.lacisoath_required_permission
        )));
    }

    // Check fid
    let fids = payload.fid.unwrap_or_default();
    let has_required_fid = fids.iter().any(|f| f == &auth.lacisoath_required_fid || f == "0000");
    if !has_required_fid {
        tracing::warn!(
            "LacisOath login denied: fid {:?} does not contain {} or 0000",
            fids,
            auth.lacisoath_required_fid
        );
        return Err(AppError::BadRequest(
            "Access not authorized for this facility".to_string(),
        ));
    }

    let lacis_id = payload.lacis_id.unwrap_or_default();

    let user = AuthUser {
        sub: lacis_id.clone(),
        lacis_id: Some(lacis_id),
        permission,
        auth_method: "lacisoath".to_string(),
    };

    let cookie = create_session_cookie(&user, auth)?;
    let body = AuthResponse {
        ok: true,
        user: user.clone(),
    };

    Ok((StatusCode::OK, [(SET_COOKIE, cookie)], Json(body)).into_response())
}

/// GET /api/auth/me
/// Return current authenticated user info
pub async fn auth_me(Extension(user): Extension<AuthUser>) -> impl IntoResponse {
    Json(AuthResponse {
        ok: true,
        user,
    })
}

/// POST /api/auth/logout
/// Clear the session cookie
pub async fn auth_logout() -> impl IntoResponse {
    let cookie = "lpg_session=; Path=/LacisProxyGateway2; HttpOnly; SameSite=Lax; Max-Age=0";
    (StatusCode::OK, [(SET_COOKIE, cookie.to_string())], Json(serde_json::json!({"ok": true}))).into_response()
}

// ============================================================================
// Helper functions
// ============================================================================

/// Create a session JWT and format as Set-Cookie header value
fn create_session_cookie(
    user: &AuthUser,
    auth: &crate::config::AuthConfig,
) -> Result<String, AppError> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(auth.session_duration_hours as i64))
        .ok_or_else(|| AppError::InternalError("Time overflow".to_string()))?
        .timestamp() as usize;

    let claims = SessionClaims {
        sub: user.sub.clone(),
        lacis_id: user.lacis_id.clone(),
        permission: user.permission,
        auth_method: user.auth_method.clone(),
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(auth.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(format!("JWT encode error: {}", e)))?;

    let max_age = auth.session_duration_hours * 3600;

    Ok(format!(
        "lpg_session={}; Path=/LacisProxyGateway2; HttpOnly; SameSite=Lax; Max-Age={}",
        token, max_age
    ))
}

/// Decoded LacisOath JWT payload (partial - only fields we need)
#[derive(Debug, Deserialize)]
struct LacisOathPayload {
    lacis_id: Option<String>,
    permission: Option<i32>,
    fid: Option<Vec<String>>,
    exp: Option<u64>,
}

/// Decode JWT payload (base64 middle segment) without signature verification
fn decode_jwt_payload(token: &str) -> Result<LacisOathPayload, AppError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::BadRequest("Invalid JWT format".to_string()));
    }

    let payload_b64 = parts[1];
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(payload_b64))
        .map_err(|e| AppError::BadRequest(format!("Invalid JWT payload encoding: {}", e)))?;

    let payload: LacisOathPayload = serde_json::from_slice(&payload_bytes)
        .map_err(|e| AppError::BadRequest(format!("Invalid JWT payload JSON: {}", e)))?;

    Ok(payload)
}
