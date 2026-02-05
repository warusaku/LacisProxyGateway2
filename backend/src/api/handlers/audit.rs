//! Audit log handlers

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::error::AppError;
use crate::proxy::ProxyState;

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// GET /api/audit - Get audit logs
pub async fn get_audit_logs(
    State(state): State<ProxyState>,
    Query(query): Query<AuditLogQuery>,
) -> Result<impl IntoResponse, AppError> {
    let logs = state
        .app_state
        .mysql
        .get_audit_logs(query.limit, query.offset)
        .await?;

    Ok(Json(logs))
}
