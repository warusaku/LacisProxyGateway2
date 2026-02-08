//! Authentication middleware - validates session JWT from HttpOnly cookie
//!
//! Extracts the `lpg_session` cookie, verifies the HS256 JWT,
//! and injects AuthUser into request extensions for downstream handlers.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

use crate::models::{AuthUser, SessionClaims};
use crate::proxy::ProxyState;

/// Middleware that requires a valid session cookie.
/// On success, injects AuthUser into request extensions.
/// On failure, returns 401 Unauthorized.
pub async fn require_auth(
    State(state): State<ProxyState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let token = extract_session_cookie(req.headers());

    match token {
        Some(t) => {
            match decode_session(&t, &state.auth_config.jwt_secret) {
                Ok(claims) => {
                    req.extensions_mut().insert(AuthUser::from(claims));
                    next.run(req).await
                }
                Err(e) => {
                    tracing::debug!("Invalid session token: {}", e);
                    unauthorized_response()
                }
            }
        }
        None => unauthorized_response(),
    }
}

/// Extract lpg_session value from Cookie header
fn extract_session_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get_all("cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(';'))
        .find_map(|cookie| {
            let cookie = cookie.trim();
            if let Some(value) = cookie.strip_prefix("lpg_session=") {
                Some(value.to_string())
            } else {
                None
            }
        })
}

/// Decode and validate a session JWT (HS256)
fn decode_session(token: &str, secret: &str) -> Result<SessionClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<SessionClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "Authentication required",
            "status": 401
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_session_cookie_present() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_static("lpg_session=abc123; other=xyz"),
        );
        assert_eq!(extract_session_cookie(&headers), Some("abc123".to_string()));
    }

    #[test]
    fn test_extract_session_cookie_absent() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("cookie", HeaderValue::from_static("other=xyz"));
        assert_eq!(extract_session_cookie(&headers), None);
    }

    #[test]
    fn test_extract_session_cookie_no_cookie_header() {
        let headers = axum::http::HeaderMap::new();
        assert_eq!(extract_session_cookie(&headers), None);
    }

    #[test]
    fn test_decode_session_valid() {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let secret = "test_secret";
        let claims = SessionClaims {
            sub: "test@example.com".to_string(),
            lacis_id: None,
            permission: 100,
            auth_method: "local".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let decoded = decode_session(&token, secret).unwrap();
        assert_eq!(decoded.sub, "test@example.com");
        assert_eq!(decoded.permission, 100);
    }

    #[test]
    fn test_decode_session_wrong_secret() {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let claims = SessionClaims {
            sub: "test@example.com".to_string(),
            lacis_id: None,
            permission: 100,
            auth_method: "local".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"secret1"),
        )
        .unwrap();

        assert!(decode_session(&token, "secret2").is_err());
    }

    #[test]
    fn test_decode_session_expired() {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let claims = SessionClaims {
            sub: "test@example.com".to_string(),
            lacis_id: None,
            permission: 100,
            auth_method: "local".to_string(),
            exp: 1000, // expired long ago
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();

        assert!(decode_session(&token, "secret").is_err());
    }
}
