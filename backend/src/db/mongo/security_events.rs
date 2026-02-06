//! Security events operations (MongoDB)

use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::{self, doc};
use mongodb::options::FindOptions;

use crate::error::AppError;
use crate::models::{SecurityEvent, SecurityEventSearchQuery, SecurityEventType, Severity};

use super::MongoDb;

impl MongoDb {
    /// Log a security event
    pub async fn log_security_event(&self, event: &SecurityEvent) -> Result<(), AppError> {
        let collection = self.db.collection::<bson::Document>("security_events");

        let doc = bson::to_document(event).map_err(|e| AppError::InternalError(e.to_string()))?;

        collection
            .insert_one(doc, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(())
    }

    /// Log an IP blocked event
    pub async fn log_ip_blocked(
        &self,
        ip: &str,
        reason: &str,
        severity: Severity,
    ) -> Result<(), AppError> {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::IpBlocked,
            ip: Some(ip.to_string()),
            details: serde_json::json!({ "reason": reason }),
            severity,
            notified: false,
        };

        self.log_security_event(&event).await
    }

    /// Log a rate limit exceeded event
    pub async fn log_rate_limit_exceeded(&self, ip: &str, requests: i32) -> Result<(), AppError> {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::RateLimitExceeded,
            ip: Some(ip.to_string()),
            details: serde_json::json!({ "requests": requests }),
            severity: Severity::Medium,
            notified: false,
        };

        self.log_security_event(&event).await
    }

    /// Log a DDNS failure event
    pub async fn log_ddns_failure(
        &self,
        hostname: &str,
        provider: &str,
        error: &str,
    ) -> Result<(), AppError> {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::DdnsFailure,
            ip: None,
            details: serde_json::json!({
                "hostname": hostname,
                "provider": provider,
                "error": error
            }),
            severity: Severity::High,
            notified: false,
        };

        self.log_security_event(&event).await
    }

    /// Log a health check failure event
    pub async fn log_health_check_failure(
        &self,
        route_id: i32,
        target: &str,
        consecutive_failures: u32,
    ) -> Result<(), AppError> {
        let severity = if consecutive_failures >= 5 {
            Severity::Critical
        } else if consecutive_failures >= 3 {
            Severity::High
        } else {
            Severity::Medium
        };

        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type: SecurityEventType::HealthCheckFailure,
            ip: None,
            details: serde_json::json!({
                "route_id": route_id,
                "target": target,
                "consecutive_failures": consecutive_failures
            }),
            severity,
            notified: false,
        };

        self.log_security_event(&event).await
    }

    /// Get recent security events
    pub async fn get_security_events(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SecurityEvent>, AppError> {
        let collection = self.db.collection::<bson::Document>("security_events");

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .skip(offset as u64)
            .limit(limit)
            .build();

        let mut cursor = collection
            .find(doc! {}, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut events = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(event) = bson::from_document(doc) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get security events by type
    pub async fn get_security_events_by_type(
        &self,
        event_type: SecurityEventType,
        limit: i64,
    ) -> Result<Vec<SecurityEvent>, AppError> {
        let collection = self.db.collection::<bson::Document>("security_events");

        let type_str = match event_type {
            SecurityEventType::IpBlocked => "ip_blocked",
            SecurityEventType::RateLimitExceeded => "rate_limit_exceeded",
            SecurityEventType::SuspiciousActivity => "suspicious_activity",
            SecurityEventType::DdnsFailure => "ddns_failure",
            SecurityEventType::HealthCheckFailure => "health_check_failure",
        };

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let mut cursor = collection
            .find(doc! { "event_type": type_str }, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut events = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(event) = bson::from_document(doc) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get security events for a specific IP
    pub async fn get_security_events_by_ip(
        &self,
        ip: &str,
        limit: i64,
    ) -> Result<Vec<SecurityEvent>, AppError> {
        let collection = self.db.collection::<bson::Document>("security_events");

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let mut cursor = collection
            .find(doc! { "ip": ip }, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut events = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(event) = bson::from_document(doc) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get unnotified security events
    pub async fn get_unnotified_events(&self) -> Result<Vec<SecurityEvent>, AppError> {
        let collection = self.db.collection::<bson::Document>("security_events");

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(100)
            .build();

        let mut cursor = collection
            .find(doc! { "notified": false }, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut events = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(event) = bson::from_document(doc) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Mark events as notified
    pub async fn mark_events_notified(
        &self,
        event_type: SecurityEventType,
    ) -> Result<(), AppError> {
        let collection = self.db.collection::<bson::Document>("security_events");

        let type_str = match event_type {
            SecurityEventType::IpBlocked => "ip_blocked",
            SecurityEventType::RateLimitExceeded => "rate_limit_exceeded",
            SecurityEventType::SuspiciousActivity => "suspicious_activity",
            SecurityEventType::DdnsFailure => "ddns_failure",
            SecurityEventType::HealthCheckFailure => "health_check_failure",
        };

        collection
            .update_many(
                doc! { "event_type": type_str, "notified": false },
                doc! { "$set": { "notified": true } },
                None,
            )
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(())
    }

    /// Advanced search: time range + severity + event_type + ip
    pub async fn search_security_events(
        &self,
        query: &SecurityEventSearchQuery,
    ) -> Result<Vec<SecurityEvent>, AppError> {
        let collection = self.db.collection::<bson::Document>("security_events");

        let mut filter = doc! {};

        // Time range (timestamp stored as ISO 8601 string, string comparison works)
        let mut time_filter = doc! {};
        if let Some(from) = query.from {
            time_filter.insert("$gte", from.to_rfc3339());
        }
        if let Some(to) = query.to {
            time_filter.insert("$lte", to.to_rfc3339());
        }
        if !time_filter.is_empty() {
            filter.insert("timestamp", time_filter);
        }

        // Severity
        if let Some(ref severity) = query.severity {
            if !severity.is_empty() {
                filter.insert("severity", severity.as_str());
            }
        }

        // Event type
        if let Some(ref event_type) = query.event_type {
            if !event_type.is_empty() {
                filter.insert("event_type", event_type.as_str());
            }
        }

        // IP
        if let Some(ref ip) = query.ip {
            if !ip.is_empty() {
                filter.insert("ip", ip.as_str());
            }
        }

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .skip(query.offset as u64)
            .limit(query.limit)
            .build();

        let mut cursor = collection
            .find(filter, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut events = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(event) = bson::from_document(doc) {
                events.push(event);
            }
        }

        Ok(events)
    }
}
