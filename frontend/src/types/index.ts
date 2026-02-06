// LacisProxyGateway2 Type Definitions

// ============================================================================
// Proxy Routes
// ============================================================================

export interface ProxyRoute {
  id: number;
  path: string;
  target: string;
  ddns_config_id?: number | null;
  priority: number;
  active: boolean;
  strip_prefix: boolean;
  preserve_host: boolean;
  timeout_ms: number;
  websocket_support: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateRouteRequest {
  path: string;
  target: string;
  ddns_config_id?: number | null;
  priority?: number;
  active?: boolean;
  strip_prefix?: boolean;
  preserve_host?: boolean;
  timeout_ms?: number;
  websocket_support?: boolean;
}

export interface UpdateRouteRequest {
  path?: string;
  target?: string;
  ddns_config_id?: number | null;
  priority?: number;
  active?: boolean;
  strip_prefix?: boolean;
  preserve_host?: boolean;
  timeout_ms?: number;
  websocket_support?: boolean;
}

// ============================================================================
// DDNS
// ============================================================================

export type DdnsProvider = 'dyndns' | 'noip' | 'cloudflare';
export type DdnsStatus = 'active' | 'error' | 'disabled';

export interface DdnsConfig {
  id: number;
  provider: DdnsProvider;
  hostname: string;
  username?: string;
  password?: string;
  api_token?: string;
  zone_id?: string;
  update_interval_sec: number;
  last_ip?: string;
  last_update?: string;
  last_error?: string;
  status: DdnsStatus;
  created_at: string;
  updated_at: string;
}

export interface CreateDdnsRequest {
  provider: DdnsProvider;
  hostname: string;
  username?: string;
  password?: string;
  api_token?: string;
  zone_id?: string;
  update_interval_sec?: number;
}

export interface UpdateDdnsRequest {
  hostname?: string;
  username?: string;
  password?: string;
  api_token?: string;
  zone_id?: string;
  update_interval_sec?: number;
  status?: DdnsStatus;
}

// ============================================================================
// Security
// ============================================================================

export interface BlockedIp {
  id: number;
  ip: string;
  reason?: string;
  blocked_by: string;
  expires_at?: string;
  created_at: string;
}

export interface BlockIpRequest {
  ip: string;
  reason?: string;
  expires_at?: string;
}

export type SecurityEventType =
  | 'ip_blocked'
  | 'rate_limit_exceeded'
  | 'suspicious_activity'
  | 'ddns_failure'
  | 'health_check_failure';

export type Severity = 'low' | 'medium' | 'high' | 'critical';

export interface SecurityEvent {
  timestamp: string;
  event_type: SecurityEventType;
  ip?: string;
  details: Record<string, unknown>;
  severity: Severity;
  notified: boolean;
}

// ============================================================================
// Settings
// ============================================================================

export interface Setting {
  id: number;
  setting_key: string;
  setting_value?: string;
  description?: string;
  updated_at: string;
}

// ============================================================================
// Dashboard
// ============================================================================

export interface DashboardStats {
  total_requests_today: number;
  active_routes: number;
  active_ddns: number;
  blocked_ips: number;
  server_health: string;
  uptime_seconds: number;
}

export interface RouteHealth {
  route_id: number;
  path: string;
  target: string;
  healthy: boolean;
  last_check?: string;
  consecutive_failures: number;
}

export interface AccessLog {
  timestamp: string;
  ip: string;
  method: string;
  path: string;
  route_id?: number;
  target?: string;
  status: number;
  response_time_ms: number;
  request_size?: number;
  response_size?: number;
  user_agent?: string;
  referer?: string;
}

export interface StatusDistribution {
  status: number;
  count: number;
}

// ============================================================================
// Advanced Search & Analytics
// ============================================================================

export interface HourlyStat {
  hour: string;
  total_requests: number;
  error_count: number;
  avg_response_time_ms: number;
}

export interface TopEntry {
  key: string;
  count: number;
  error_count: number;
}

export interface ErrorSummary {
  status: number;
  count: number;
  paths: string[];
}

export interface AccessLogSearchResult {
  logs: AccessLog[];
  total: number;
}

export interface AccessLogSearchParams {
  from?: string;
  to?: string;
  method?: string;
  status_min?: number;
  status_max?: number;
  ip?: string;
  path?: string;
  limit?: number;
  offset?: number;
}

export interface SecurityEventSearchParams {
  from?: string;
  to?: string;
  severity?: string;
  event_type?: string;
  ip?: string;
  limit?: number;
  offset?: number;
}

// ============================================================================
// API Response
// ============================================================================

export interface SuccessResponse {
  message: string;
  id?: number;
}

export interface ErrorResponse {
  error: string;
  status: number;
}
