//! Access log operations (MongoDB)

use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::{self, doc};
use mongodb::options::FindOptions;

use crate::error::AppError;
use crate::models::{
    AccessLog, AccessLogSearchQuery, AccessLogSearchResult, ErrorSummary, HealthCheck, HourlyStat,
    TopEntry,
};

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
                "timestamp": { "$gte": today_start.to_rfc3339() }
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
            doc! { "$match": { "timestamp": { "$gte": today_start.to_rfc3339() } } },
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

    /// Get statistics for a specific route path
    pub async fn get_route_stats(&self, path: &str) -> Result<crate::models::RouteStats, AppError> {
        use chrono::Duration;

        let collection = self.db.collection::<bson::Document>("access_logs");

        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let hour_ago = Utc::now() - Duration::hours(1);

        // Escape regex special characters in path
        let escaped_path = regex::escape(path);

        // Count requests today
        let requests_today = collection
            .count_documents(
                doc! {
                    "path": { "$regex": format!("^{}", escaped_path) },
                    "timestamp": { "$gte": today_start.to_rfc3339() }
                },
                None,
            )
            .await
            .unwrap_or(0);

        // Count requests last hour
        let requests_last_hour = collection
            .count_documents(
                doc! {
                    "path": { "$regex": format!("^{}", escaped_path) },
                    "timestamp": { "$gte": hour_ago.to_rfc3339() }
                },
                None,
            )
            .await
            .unwrap_or(0);

        // Calculate error rate (4xx and 5xx) for today
        let error_count = collection
            .count_documents(
                doc! {
                    "path": { "$regex": format!("^{}", escaped_path) },
                    "timestamp": { "$gte": today_start.to_rfc3339() },
                    "status": { "$gte": 400 }
                },
                None,
            )
            .await
            .unwrap_or(0);

        let error_rate_percent = if requests_today > 0 {
            (error_count as f64 / requests_today as f64) * 100.0
        } else {
            0.0
        };

        // Calculate average response time for today
        let pipeline = vec![
            doc! {
                "$match": {
                    "path": { "$regex": format!("^{}", escaped_path) },
                    "timestamp": { "$gte": today_start.to_rfc3339() },
                    "response_time_ms": { "$exists": true }
                }
            },
            doc! {
                "$group": {
                    "_id": null,
                    "avg_response_time": { "$avg": "$response_time_ms" }
                }
            },
        ];

        let mut cursor = collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut avg_response_time_ms = 0.0;
        if let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            if let Ok(avg) = doc.get_f64("avg_response_time") {
                avg_response_time_ms = avg;
            }
        }

        Ok(crate::models::RouteStats {
            requests_today,
            requests_last_hour,
            error_rate_percent,
            avg_response_time_ms,
        })
    }

    /// Advanced search: time range + method + status range + IP + path with pagination
    pub async fn search_access_logs(
        &self,
        query: &AccessLogSearchQuery,
    ) -> Result<AccessLogSearchResult, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let filter = Self::build_access_log_filter(query);

        // Get total count
        let total = collection
            .count_documents(filter.clone(), None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        // Get paginated results
        let options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .skip(query.offset as u64)
            .limit(query.limit)
            .build();

        let mut cursor = collection
            .find(filter, options)
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

        Ok(AccessLogSearchResult { logs, total })
    }

    /// Hourly aggregation: aggregate by hour within specified period
    pub async fn get_hourly_stats(
        &self,
        from: chrono::DateTime<Utc>,
        to: chrono::DateTime<Utc>,
    ) -> Result<Vec<HourlyStat>, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        // Timestamp is stored as ISO 8601 string, so use string comparison
        // and $substrBytes to extract hour portion (first 13 chars = "2026-02-06T22")
        let pipeline = vec![
            doc! {
                "$match": {
                    "timestamp": {
                        "$gte": from.to_rfc3339(),
                        "$lte": to.to_rfc3339(),
                    }
                }
            },
            doc! {
                "$group": {
                    "_id": { "$substrBytes": ["$timestamp", 0, 13] },
                    "total_requests": { "$sum": 1 },
                    "error_count": {
                        "$sum": {
                            "$cond": [{ "$gte": ["$status", 400] }, 1, 0]
                        }
                    },
                    "avg_response_time_ms": { "$avg": "$response_time_ms" }
                }
            },
            doc! { "$sort": { "_id": 1 } },
        ];

        let mut cursor = collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut stats = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            // _id is "2026-02-06T22" (first 13 chars), append ":00:00Z" for full ISO format
            let hour_prefix = doc.get_str("_id").unwrap_or("");
            let hour = format!("{}:00:00Z", hour_prefix);
            let total_requests = doc.get_i64("total_requests").unwrap_or(0) as u64;
            let error_count = doc.get_i64("error_count").unwrap_or(0) as u64;
            let avg_response_time_ms = doc.get_f64("avg_response_time_ms").unwrap_or(0.0);

            stats.push(HourlyStat {
                hour,
                total_requests,
                error_count,
                avg_response_time_ms,
            });
        }

        Ok(stats)
    }

    /// Top N IPs by request count
    pub async fn get_top_ips(
        &self,
        from: chrono::DateTime<Utc>,
        to: chrono::DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<TopEntry>, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let pipeline = vec![
            doc! {
                "$match": {
                    "timestamp": {
                        "$gte": from.to_rfc3339(),
                        "$lte": to.to_rfc3339(),
                    }
                }
            },
            doc! {
                "$group": {
                    "_id": "$ip",
                    "count": { "$sum": 1 },
                    "error_count": {
                        "$sum": {
                            "$cond": [{ "$gte": ["$status", 400] }, 1, 0]
                        }
                    }
                }
            },
            doc! { "$sort": { "count": -1 } },
            doc! { "$limit": limit },
        ];

        let mut cursor = collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut entries = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            let key = doc.get_str("_id").unwrap_or("").to_string();
            let count = doc.get_i64("count").unwrap_or(0) as u64;
            let error_count = doc.get_i64("error_count").unwrap_or(0) as u64;
            entries.push(TopEntry {
                key,
                count,
                error_count,
            });
        }

        Ok(entries)
    }

    /// Top N paths by request count
    pub async fn get_top_paths(
        &self,
        from: chrono::DateTime<Utc>,
        to: chrono::DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<TopEntry>, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let pipeline = vec![
            doc! {
                "$match": {
                    "timestamp": {
                        "$gte": from.to_rfc3339(),
                        "$lte": to.to_rfc3339(),
                    }
                }
            },
            doc! {
                "$group": {
                    "_id": "$path",
                    "count": { "$sum": 1 },
                    "error_count": {
                        "$sum": {
                            "$cond": [{ "$gte": ["$status", 400] }, 1, 0]
                        }
                    }
                }
            },
            doc! { "$sort": { "count": -1 } },
            doc! { "$limit": limit },
        ];

        let mut cursor = collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut entries = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            let key = doc.get_str("_id").unwrap_or("").to_string();
            let count = doc.get_i64("count").unwrap_or(0) as u64;
            let error_count = doc.get_i64("error_count").unwrap_or(0) as u64;
            entries.push(TopEntry {
                key,
                count,
                error_count,
            });
        }

        Ok(entries)
    }

    /// Error (4xx/5xx) grouping summary
    pub async fn get_error_summary(
        &self,
        from: chrono::DateTime<Utc>,
        to: chrono::DateTime<Utc>,
    ) -> Result<Vec<ErrorSummary>, AppError> {
        let collection = self.db.collection::<bson::Document>("access_logs");

        let pipeline = vec![
            doc! {
                "$match": {
                    "timestamp": {
                        "$gte": from.to_rfc3339(),
                        "$lte": to.to_rfc3339(),
                    },
                    "status": { "$gte": 400 }
                }
            },
            doc! {
                "$group": {
                    "_id": {
                        "status": "$status",
                        "path": "$path"
                    },
                    "count": { "$sum": 1 }
                }
            },
            doc! { "$sort": { "count": -1 } },
            doc! {
                "$group": {
                    "_id": "$_id.status",
                    "count": { "$sum": "$count" },
                    "paths": {
                        "$push": {
                            "path": "$_id.path",
                            "count": "$count"
                        }
                    }
                }
            },
            doc! { "$sort": { "count": -1 } },
        ];

        let mut cursor = collection
            .aggregate(pipeline, None)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut summaries = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            let status = doc.get_i32("_id").unwrap_or(0);
            let count = doc.get_i64("count").unwrap_or(0) as u64;

            // Extract top 3 paths
            let mut paths = Vec::new();
            if let Ok(paths_arr) = doc.get_array("paths") {
                for (i, p) in paths_arr.iter().enumerate() {
                    if i >= 3 {
                        break;
                    }
                    if let Some(p_doc) = p.as_document() {
                        if let Ok(path) = p_doc.get_str("path") {
                            paths.push(path.to_string());
                        }
                    }
                }
            }

            summaries.push(ErrorSummary {
                status,
                count,
                paths,
            });
        }

        Ok(summaries)
    }

    /// Build MongoDB filter document from search query
    fn build_access_log_filter(query: &AccessLogSearchQuery) -> bson::Document {
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

        // Method
        if let Some(ref method) = query.method {
            if !method.is_empty() {
                filter.insert("method", method.to_uppercase());
            }
        }

        // Status range
        let mut status_filter = doc! {};
        if let Some(min) = query.status_min {
            status_filter.insert("$gte", min);
        }
        if let Some(max) = query.status_max {
            status_filter.insert("$lte", max);
        }
        if !status_filter.is_empty() {
            filter.insert("status", status_filter);
        }

        // IP
        if let Some(ref ip) = query.ip {
            if !ip.is_empty() {
                filter.insert("ip", ip.as_str());
            }
        }

        // Path (regex)
        if let Some(ref path) = query.path {
            if !path.is_empty() {
                filter.insert("path", doc! { "$regex": path.as_str() });
            }
        }

        filter
    }
}
