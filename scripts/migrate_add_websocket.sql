-- Migration: Add websocket_support column to proxy_routes
-- Run with: mariadb -u akihabara_admin -p < migrate_add_websocket.sql

USE lacis_proxy;

ALTER TABLE proxy_routes
ADD COLUMN IF NOT EXISTS websocket_support BOOLEAN DEFAULT FALSE
COMMENT 'Enable WebSocket proxy support'
AFTER timeout_ms;
