//! Authentication handlers
//!
//! - POST /api/auth/login/local     - Local email+password login
//! - POST /api/auth/login/lacisoath - OAuth 2.0 Authorization Code login (mobes 2.0)
//! - GET  /api/auth/lacisoath-config - OAuth 2.0 client config (public, no secrets)
//! - GET  /api/auth/me               - Get current authenticated user
//! - POST /api/auth/logout            - Clear session cookie

use axum::{
    extract::State,
    http::{header::SET_COOKIE, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Deserialize;

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::models::{
    ApiKeyRequest, ApiKeyResponse, AuthResponse, AuthUser,
    LacisOathLoginRequest, LocalLoginRequest, SessionClaims,
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
/// OAuth 2.0 Authorization Code Flow: exchange code for token via mobes externalAuthToken
pub async fn login_lacisoath(
    State(state): State<ProxyState>,
    Json(req): Json<LacisOathLoginRequest>,
) -> Result<Response, AppError> {
    let auth = &state.auth_config;

    // Verify OAuth 2.0 client is configured
    if auth.lacisoath_client_id.is_empty() || auth.lacisoath_client_secret.is_empty() {
        return Err(AppError::InternalError(
            "LacisOath is not configured".to_string(),
        ));
    }

    // Step 1: Exchange authorization code for token
    let client = reqwest::Client::new();
    let token_resp = client
        .post(&auth.lacisoath_token_url)
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": req.code,
            "client_id": auth.lacisoath_client_id,
            "client_secret": auth.lacisoath_client_secret,
            "redirect_uri": req.redirect_uri,
        }))
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("Token exchange failed: {}", e)))?;

    if !token_resp.status().is_success() {
        let body = token_resp.text().await.unwrap_or_default();
        tracing::warn!("LacisOath token exchange failed: {}", body);
        return Err(AppError::BadRequest(format!(
            "Authentication failed: {}",
            body
        )));
    }

    // Step 2: Parse token response
    let token_data: LacisOathTokenResponse = token_resp
        .json()
        .await
        .map_err(|e| AppError::InternalError(format!("Invalid token response: {}", e)))?;

    let user_info = &token_data.data.user_info;

    // Step 3: Permission check
    if user_info.permission < auth.lacisoath_required_permission {
        tracing::warn!(
            "LacisOath login denied: permission {} < required {}",
            user_info.permission,
            auth.lacisoath_required_permission
        );
        return Err(AppError::BadRequest(format!(
            "Insufficient permission: {} (required: {})",
            user_info.permission, auth.lacisoath_required_permission
        )));
    }

    // Step 4: FID check
    let has_fid = user_info
        .fid
        .iter()
        .any(|f| f == &auth.lacisoath_required_fid || f == "0000");
    if !has_fid {
        tracing::warn!(
            "LacisOath login denied: fid {:?} does not contain {} or 0000",
            user_info.fid,
            auth.lacisoath_required_fid
        );
        return Err(AppError::BadRequest(
            "Access not authorized for this facility".to_string(),
        ));
    }

    // Step 5: Create session
    let user = AuthUser {
        sub: user_info.lacis_id.clone(),
        lacis_id: Some(user_info.lacis_id.clone()),
        permission: user_info.permission,
        auth_method: "lacisoath".to_string(),
    };

    let cookie = create_session_cookie(&user, auth)?;
    let body = AuthResponse {
        ok: true,
        user: user.clone(),
    };

    Ok((StatusCode::OK, [(SET_COOKIE, cookie)], Json(body)).into_response())
}

/// GET /api/auth/lacisoath-config
/// Returns OAuth 2.0 client config for frontend (no secrets exposed)
pub async fn lacisoath_config(State(state): State<ProxyState>) -> impl IntoResponse {
    let auth = &state.auth_config;
    let enabled =
        !auth.lacisoath_client_id.is_empty() && !auth.lacisoath_client_secret.is_empty();

    Json(serde_json::json!({
        "enabled": enabled,
        "client_id": if enabled { &auth.lacisoath_client_id } else { "" },
        "auth_url": &auth.lacisoath_auth_url,
        "redirect_uri": &auth.lacisoath_redirect_uri,
    }))
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
    (
        StatusCode::OK,
        [(SET_COOKIE, cookie.to_string())],
        Json(serde_json::json!({"ok": true})),
    )
        .into_response()
}

/// POST /api/auth/api-key
/// Issue a long-lived API key (JWT) for AI agents / CLI usage.
/// Requires permission == 100 (dangerous operation).
pub async fn create_api_key(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<ApiKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Only permission 100 users can issue API keys
    require_permission(&user, 100)?;

    // Validate key name
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("Key name is required".to_string()));
    }

    let expires_in_days = req.expires_in_days.unwrap_or(365);
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(expires_in_days as i64))
        .ok_or_else(|| AppError::InternalError("Time overflow".to_string()))?;

    let claims = SessionClaims {
        sub: user.sub.clone(),
        lacis_id: user.lacis_id.clone(),
        permission: user.permission,
        auth_method: "api_key".to_string(),
        exp: expires_at.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.auth_config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(format!("JWT encode error: {}", e)))?;

    tracing::info!(
        "API key '{}' issued for {} (expires: {})",
        req.name,
        user.sub,
        expires_at.to_rfc3339()
    );

    Ok(Json(ApiKeyResponse {
        token,
        expires_at: expires_at.to_rfc3339(),
        name: req.name,
    }))
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

// ============================================================================
// OAuth 2.0 Token Exchange Response Types
// ============================================================================

/// Response from mobes externalAuthToken endpoint
#[derive(Debug, Deserialize)]
struct LacisOathTokenResponse {
    #[allow(dead_code)]
    status: String,
    data: LacisOathTokenData,
}

#[derive(Debug, Deserialize)]
struct LacisOathTokenData {
    #[serde(rename = "accessToken")]
    #[allow(dead_code)]
    access_token: String,
    #[serde(rename = "userInfo")]
    user_info: LacisOathUserInfo,
}

#[derive(Debug, Deserialize)]
struct LacisOathUserInfo {
    #[serde(rename = "lacisId")]
    lacis_id: String,
    permission: i32,
    fid: Vec<String>,
}
