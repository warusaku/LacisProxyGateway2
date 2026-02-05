//! Access log operations (MongoDB)

use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::{self, doc, DateTime as BsonDateTime};
use mongodb::options::FindOptions;

use crate::error::AppError;
use crate::models::{AccessLog, HealthCheck};

use super::MongoDb;

impl MongoDb {
    /// Log an access event
    pub async fn log_access(&self, log: &AccessLog) -> Result<(), AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let doc = bson::to_document(log).map_err(|e| AppError::InternalError(e.to_string()))?;

        collection
            .insert_one(doc, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(())
    }

    /// Get recent access logs
    pub async fn get_access_logs(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AccessLog>, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .skip(offset as u64)
            .limit(limit)
            .build();

        let mut cursor = collection
            .find(doc! {}, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut logs = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(log) = bson::from_document(doc) {
                logs.push(log);
            }
        }

        Ok(logs)
    }

    /// Get access logs for a specific path
    pub async fn get_access_logs_by_path(
        &self,
        path: &str,
        limit: i64,
    ) -> Result<Vec<AccessLog>, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let mut cursor = collection
            .find(doc! { "path": { "$regex": path } }, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut logs = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(log) = bson::from_document(doc) {
                logs.push(log);
            }
        }

        Ok(logs)
    }

    /// Get access logs for a specific IP
    pub async fn get_access_logs_by_ip(
        &self,
        ip: &str,
        limit: i64,
    ) -> Result<Vec<AccessLog>, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let mut cursor = collection
            .find(doc! { "ip": ip }, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut logs = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(log) = bson::from_document(doc) {
                logs.push(log);
            }
        }

        Ok(logs)
    }

    /// Get total request count for today
    pub async fn get_today_request_count(&self) -> Result<u64, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        let count = collection
            .count_documents(doc! {
                "timestamp": { "$gte": BsonDateTime::from_millis(today_start.timestamp_millis()) }
            }, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(count)
    }

    /// Get request count by status code for today
    pub async fn get_today_status_distribution(&self) -> Result<Vec<(i32, u64)>, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        let pipeline = vec![
            doc! { "$match": { "timestamp": { "$gte": BsonDateTime::from_millis(today_start.timestamp_millis()) } } },
            doc! { "$group": { "_id": "$status", "count": { "$sum": 1 } } },
            doc! { "$sort": { "_id": 1 } },
        ];

        let mut cursor = collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut distribution = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let (Ok(status), Ok(count)) = (doc.get_i32("_id"), doc.get_i64("count")) {
                distribution.push((status, count as u64));
            }
        }

        Ok(distribution)
    }

    /// Save a health check result
    pub async fn save_health_check(&self, check: &HealthCheck) -> Result<(), AppError> {
        let collection = self.db.collection::<bson::Document>("health_checks");

        let doc = bson::to_document(check).map_err(|e| AppError::InternalError(e.to_string()))?;

        collection
            .insert_one(doc, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(())
    }

    /// Get recent health checks for a route
    pub async fn get_health_checks_by_route(
        &self,
        route_id: i32,
        limit: i64,
    ) -> Result<Vec<HealthCheck>, AppError> {
        let collection = self.db.collection::<bson::Document>("health_checks");

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(limit)
            .build();

        let mut cursor = collection
            .find(doc! { "route_id": route_id }, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut checks = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(check) = bson::from_document(doc) {
                checks.push(check);
            }
        }

        Ok(checks)
    }

    /// Get latest health status for all routes
    pub async fn get_latest_health_status(&self) -> Result<Vec<HealthCheck>, AppError> {
        let collection = self.db.collection::<bson::Document>("health_checks");

        let pipeline = vec![
            doc! { "$sort": { "timestamp": -1 } },
            doc! { "$group": {
                "_id": "$route_id",
                "doc": { "$first": "$$ROOT" }
            }},
            doc! { "$replaceRoot": { "newRoot": "$doc" } },
        ];

        let mut cursor = collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut checks = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(check) = bson::from_document(doc) {
                checks.push(check);
            }
        }

        Ok(checks)
    }

    /// Count consecutive failures for a route
    pub async fn count_consecutive_failures(&self, route_id: i32) -> Result<u32, AppError> {
        let collection = self.db.collection::<bson::Document>("health_checks");

        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(100)
            .build();

        let mut cursor = collection
            .find(doc! { "route_id": route_id }, options)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut count = 0u32;
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(healthy) = doc.get_bool("healthy") {
                if !healthy {
                    count += 1;
                } else {
                    break;
                }
            }
        }

        Ok(count)
    }
}
