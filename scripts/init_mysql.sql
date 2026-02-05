-- LacisProxyGateway2 MariaDB Schema
-- Run with: mariadb -u akihabara_admin -p < init_mysql.sql

CREATE DATABASE IF NOT EXISTS lacis_proxy CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
USE lacis_proxy;

-- Proxy Routes Table
CREATE TABLE IF NOT EXISTS proxy_routes (
    id INT AUTO_INCREMENT PRIMARY KEY,
    path VARCHAR(255) NOT NULL UNIQUE COMMENT 'URL path (e.g., /eatyui)',
    target VARCHAR(500) NOT NULL COMMENT 'Target URL (e.g., http://192.168.3.242:3000)',
    priority INT DEFAULT 100 COMMENT 'Lower = higher priority',
    active BOOLEAN DEFAULT TRUE,
    strip_prefix BOOLEAN DEFAULT TRUE COMMENT 'Strip matched prefix from forwarded request',
    preserve_host BOOLEAN DEFAULT FALSE COMMENT 'Preserve original Host header',
    timeout_ms INT DEFAULT 30000 COMMENT 'Request timeout in milliseconds',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_path (path),
    INDEX idx_active_priority (active, priority)
) ENGINE=InnoDB;

-- DDNS Configurations Table
CREATE TABLE IF NOT EXISTS ddns_configs (
    id INT AUTO_INCREMENT PRIMARY KEY,
    provider ENUM('dyndns', 'noip', 'cloudflare') NOT NULL,
    hostname VARCHAR(255) NOT NULL COMMENT 'DDNS hostname',
    username VARCHAR(255) COMMENT 'Auth username (encrypted)',
    password VARCHAR(500) COMMENT 'Auth password (encrypted)',
    api_token VARCHAR(500) COMMENT 'API token for Cloudflare (encrypted)',
    zone_id VARCHAR(100) COMMENT 'Cloudflare zone ID',
    update_interval_sec INT DEFAULT 300 COMMENT 'Update interval in seconds',
    last_ip VARCHAR(45) COMMENT 'Last known IP address',
    last_update TIMESTAMP NULL COMMENT 'Last successful update',
    last_error TEXT COMMENT 'Last error message if any',
    status ENUM('active', 'error', 'disabled') DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uk_provider_hostname (provider, hostname)
) ENGINE=InnoDB;

-- Blocked IPs Table
CREATE TABLE IF NOT EXISTS blocked_ips (
    id INT AUTO_INCREMENT PRIMARY KEY,
    ip VARCHAR(45) NOT NULL UNIQUE COMMENT 'IPv4 or IPv6 address',
    reason VARCHAR(500) COMMENT 'Block reason',
    blocked_by VARCHAR(50) DEFAULT 'manual' COMMENT 'manual or auto',
    expires_at TIMESTAMP NULL COMMENT 'NULL = permanent',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_ip (ip),
    INDEX idx_expires (expires_at)
) ENGINE=InnoDB;

-- Settings Table (Key-Value store)
CREATE TABLE IF NOT EXISTS settings (
    id INT AUTO_INCREMENT PRIMARY KEY,
    setting_key VARCHAR(100) NOT NULL UNIQUE,
    setting_value TEXT,
    description VARCHAR(500),
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB;

-- Insert default settings
INSERT INTO settings (setting_key, setting_value, description) VALUES
    ('discord_webhook_url', NULL, 'Discord webhook URL for notifications'),
    ('discord_notify_security', 'true', 'Notify security events to Discord'),
    ('discord_notify_health', 'true', 'Notify health check failures to Discord'),
    ('discord_notify_ddns', 'true', 'Notify DDNS update events to Discord'),
    ('rate_limit_enabled', 'true', 'Enable rate limiting'),
    ('rate_limit_requests_per_minute', '60', 'Max requests per minute per IP'),
    ('health_check_interval_sec', '60', 'Health check interval in seconds'),
    ('health_check_timeout_ms', '5000', 'Health check timeout in milliseconds'),
    ('health_check_failure_threshold', '3', 'Consecutive failures before alert'),
    ('access_log_retention_days', '30', 'Days to retain access logs')
ON DUPLICATE KEY UPDATE setting_key = setting_key;

-- Insert initial proxy routes
INSERT INTO proxy_routes (path, target, priority, active, strip_prefix) VALUES
    ('/eatyui', 'http://192.168.3.242:3000', 10, TRUE, TRUE),
    ('/sorapiapps', 'http://192.168.3.241', 20, TRUE, TRUE)
ON DUPLICATE KEY UPDATE target = VALUES(target);
