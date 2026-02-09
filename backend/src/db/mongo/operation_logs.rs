//! MongoDB CRUD for operation logs
//!
//! Collection: `operation_logs`
//! Tracks sync operations, tool executions, and device registrations.

use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::{self, doc};
use mongodb::options::FindOptions;
use serde::{Deserialize, Serialize};

use super::MongoDb;

// ============================================================================
// Document type
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLogDoc {
    pub operation_id: String,
    /// "sync_omada" | "sync_openwrt" | "sync_external" | "ddns_update"
    /// | "ping" | "dns" | "curl" | "device_register"
    pub operation_type: String,
    /// "manual" | "scheduler" | "api"
    pub initiated_by: String,
    /// controller_id, router_id, hostname, etc.
    pub target: Option<String>,
    /// "success" | "error" | "running"
    pub status: String,
    /// Operation result (JSON)
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
    pub created_at: String,
    /// Operator info for audit trail (populated when initiated via authenticated API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<OperatorInfo>,
}

/// Operator info attached to operation logs for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorInfo {
    pub sub: String,
    pub auth_method: String,
    pub permission: i32,
}

// ============================================================================
// Query parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OperationLogQuery {
    pub operation_type: Option<String>,
    pub status: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<u64>,
}

// ============================================================================
// MongoDB operations
// ============================================================================

impl MongoDb {
    /// Insert a new operation log
    pub async fn insert_operation_log(&self, log: &OperationLogDoc) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("operation_logs");
        let bson_doc =
            bson::to_document(log).map_err(|e| format!("Serialize operation_log: {}", e))?;

        collection
            .insert_one(bson_doc, None)
            .await
            .map_err(|e| format!("Insert operation_log: {}", e))?;

        Ok(())
    }

    /// Update operation log status and result
    pub async fn update_operation_log(
        &self,
        operation_id: &str,
        status: &str,
        result: Option<&serde_json::Value>,
        error: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Result<(), String> {
        let collection = self.db.collection::<bson::Document>("operation_logs");
        let filter = doc! { "operation_id": operation_id };

        let mut set_doc = doc! { "status": status };
        if let Some(r) = result {
            let bson_val = bson::to_bson(r).map_err(|e| format!("Serialize result: {}", e))?;
            set_doc.insert("result", bson_val);
        }
        if let Some(e) = error {
            set_doc.insert("error", e);
        }
        if let Some(d) = duration_ms {
            set_doc.insert("duration_ms", d as i64);
        }

        collection
            .update_one(filter, doc! { "$set": set_doc }, None)
            .await
            .map_err(|e| format!("Update operation_log: {}", e))?;

        Ok(())
    }

    /// Query operation logs with filters
    pub async fn query_operation_logs(
        &self,
        query: &OperationLogQuery,
    ) -> Result<Vec<OperationLogDoc>, String> {
        let collection = self.db.collection::<bson::Document>("operation_logs");

        let mut filter = doc! {};
        if let Some(op_type) = &query.operation_type {
            filter.insert("operation_type", op_type);
        }
        if let Some(status) = &query.status {
            filter.insert("status", status);
        }
        if let Some(from) = &query.from {
            filter
                .entry("created_at".to_string())
                .or_insert_with(|| bson::Bson::Document(doc! {}))
                .as_document_mut()
                .map(|d| d.insert("$gte", from.as_str()));
        }
        if let Some(to) = &query.to {
            filter
                .entry("created_at".to_string())
                .or_insert_with(|| bson::Bson::Document(doc! {}))
                .as_document_mut()
                .map(|d| d.insert("$lte", to.as_str()));
        }

        let limit = query.limit.unwrap_or(100);
        let skip = query.offset.unwrap_or(0);

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .limit(limit)
            .skip(skip)
            .build();

        let mut cursor = collection
            .find(filter, Some(options))
            .await
            .map_err(|e| format!("Query operation_logs: {}", e))?;

        let mut logs = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("Cursor operation_logs: {}", e))?
        {
            if let Ok(log) = bson::from_document(doc) {
                logs.push(log);
            }
        }

        Ok(logs)
    }

    /// Get a single operation log by ID
    pub async fn get_operation_log(
        &self,
        operation_id: &str,
    ) -> Result<Option<OperationLogDoc>, String> {
        let collection = self.db.collection::<bson::Document>("operation_logs");

        let doc = collection
            .find_one(doc! { "operation_id": operation_id }, None)
            .await
            .map_err(|e| format!("Get operation_log: {}", e))?;

        match doc {
            Some(d) => {
                let log = bson::from_document(d)
                    .map_err(|e| format!("Deserialize operation_log: {}", e))?;
                Ok(Some(log))
            }
            None => Ok(None),
        }
    }

    /// Get operation log summary (count by type and status)
    pub async fn get_operation_log_summary(&self) -> Result<serde_json::Value, String> {
        let collection = self.db.collection::<bson::Document>("operation_logs");

        // Count recent operations (last 24h)
        let now = Utc::now().to_rfc3339();
        let yesterday = (Utc::now() - chrono::Duration::hours(24)).to_rfc3339();

        let total = collection.count_documents(doc! {}, None).await.unwrap_or(0);

        let recent = collection
            .count_documents(doc! { "created_at": { "$gte": &yesterday } }, None)
            .await
            .unwrap_or(0);

        let recent_errors = collection
            .count_documents(
                doc! { "status": "error", "created_at": { "$gte": &yesterday } },
                None,
            )
            .await
            .unwrap_or(0);

        let recent_success = collection
            .count_documents(
                doc! { "status": "success", "created_at": { "$gte": &yesterday } },
                None,
            )
            .await
            .unwrap_or(0);

        Ok(serde_json::json!({
            "total": total,
            "recent_24h": recent,
            "recent_errors": recent_errors,
            "recent_success": recent_success,
            "generated_at": now,
        }))
    }

    /// Create an operation log entry with "running" status and return its ID
    pub async fn start_operation_log(
        &self,
        operation_type: &str,
        initiated_by: &str,
        target: Option<&str>,
    ) -> Result<String, String> {
        self.start_operation_log_with_operator(operation_type, initiated_by, target, None)
            .await
    }

    /// Create an operation log entry with operator info for audit trail
    pub async fn start_operation_log_with_operator(
        &self,
        operation_type: &str,
        initiated_by: &str,
        target: Option<&str>,
        operator: Option<OperatorInfo>,
    ) -> Result<String, String> {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let log = OperationLogDoc {
            operation_id: operation_id.clone(),
            operation_type: operation_type.to_string(),
            initiated_by: initiated_by.to_string(),
            target: target.map(|s| s.to_string()),
            status: "running".to_string(),
            result: None,
            error: None,
            duration_ms: None,
            created_at: now,
            operator,
        };

        self.insert_operation_log(&log).await?;
        Ok(operation_id)
    }

    /// Complete an operation log with success
    pub async fn complete_operation_log(
        &self,
        operation_id: &str,
        result: Option<&serde_json::Value>,
        duration_ms: u64,
    ) -> Result<(), String> {
        self.update_operation_log(operation_id, "success", result, None, Some(duration_ms))
            .await
    }

    /// Fail an operation log with error
    pub async fn fail_operation_log(
        &self,
        operation_id: &str,
        error: &str,
        duration_ms: u64,
    ) -> Result<(), String> {
        self.update_operation_log(operation_id, "error", None, Some(error), Some(duration_ms))
            .await
    }
}
