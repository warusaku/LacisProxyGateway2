// LacisProxyGateway2 MongoDB Setup
// Run with: mongosh < init_mongo.js

use lacis_proxy;

// Access Logs Collection (TTL: 30 days)
db.createCollection("access_logs", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            required: ["timestamp", "ip", "path", "status"],
            properties: {
                timestamp: { bsonType: "date" },
                ip: { bsonType: "string" },
                method: { bsonType: "string" },
                path: { bsonType: "string" },
                route_id: { bsonType: ["int", "null"] },
                target: { bsonType: "string" },
                status: { bsonType: "int" },
                response_time_ms: { bsonType: "int" },
                request_size: { bsonType: "int" },
                response_size: { bsonType: "int" },
                user_agent: { bsonType: "string" },
                referer: { bsonType: "string" }
            }
        }
    }
});

db.access_logs.createIndex({ "timestamp": 1 }, { expireAfterSeconds: 2592000 }); // 30 days TTL
db.access_logs.createIndex({ "ip": 1, "timestamp": -1 });
db.access_logs.createIndex({ "path": 1, "timestamp": -1 });
db.access_logs.createIndex({ "status": 1, "timestamp": -1 });

// Security Events Collection
db.createCollection("security_events", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            required: ["timestamp", "event_type"],
            properties: {
                timestamp: { bsonType: "date" },
                event_type: {
                    enum: ["ip_blocked", "rate_limit_exceeded", "suspicious_activity", "ddns_failure", "health_check_failure"]
                },
                ip: { bsonType: "string" },
                details: { bsonType: "object" },
                severity: { enum: ["low", "medium", "high", "critical"] },
                notified: { bsonType: "bool" }
            }
        }
    }
});

db.security_events.createIndex({ "timestamp": 1 }, { expireAfterSeconds: 7776000 }); // 90 days TTL
db.security_events.createIndex({ "event_type": 1, "timestamp": -1 });
db.security_events.createIndex({ "severity": 1, "timestamp": -1 });
db.security_events.createIndex({ "ip": 1, "timestamp": -1 });

// Health Check History Collection
db.createCollection("health_checks", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            required: ["timestamp", "route_id", "target", "healthy"],
            properties: {
                timestamp: { bsonType: "date" },
                route_id: { bsonType: "int" },
                target: { bsonType: "string" },
                healthy: { bsonType: "bool" },
                response_time_ms: { bsonType: "int" },
                status_code: { bsonType: ["int", "null"] },
                error: { bsonType: "string" }
            }
        }
    }
});

db.health_checks.createIndex({ "timestamp": 1 }, { expireAfterSeconds: 604800 }); // 7 days TTL
db.health_checks.createIndex({ "route_id": 1, "timestamp": -1 });
db.health_checks.createIndex({ "healthy": 1, "timestamp": -1 });

print("MongoDB collections and indexes created successfully!");
