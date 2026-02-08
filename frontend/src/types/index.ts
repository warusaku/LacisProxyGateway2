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
  omada_controller_id?: string;
  omada_site_id?: string;
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
  // GeoIP fields (optional, populated when GeoIP DB is available)
  country_code?: string;
  country?: string;
  city?: string;
  latitude?: number;
  longitude?: number;
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
  // GeoIP fields (populated for IP-based entries)
  country_code?: string;
  country?: string;
  city?: string;
  latitude?: number;
  longitude?: number;
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
  exclude_ips?: string;
  exclude_lan?: boolean;
}

export interface IpExclusionParams {
  exclude_ips?: string;
  exclude_lan?: boolean;
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
// Authentication
// ============================================================================

export interface AuthUser {
  sub: string;
  lacis_id?: string;
  permission: number;
  auth_method: 'local' | 'lacisoath';
}

export interface AuthResponse {
  ok: boolean;
  user: AuthUser;
}

export interface LocalLoginRequest {
  email: string;
  password: string;
}

export interface LacisOathConfig {
  enabled: boolean;
  client_id: string;
  auth_url: string;
  redirect_uri: string;
}

// ============================================================================
// OpenWrt
// ============================================================================

export interface OpenWrtRouterDoc {
  router_id: string;
  display_name: string;
  mac: string;
  ip: string;
  port: number;
  username: string;
  password: string;
  firmware: string;
  status: string;
  wan_ip?: string;
  lan_ip?: string;
  ssid_24g?: string;
  ssid_5g?: string;
  uptime_seconds?: number;
  client_count: number;
  firmware_version?: string;
  last_error?: string;
  omada_controller_id?: string;
  omada_site_id?: string;
  lacis_id?: string;
  product_type: string;
  network_device_type: string;
  last_polled_at?: string;
  created_at: string;
  updated_at: string;
}

export interface OpenWrtClientDoc {
  mac: string;
  router_id: string;
  ip: string;
  hostname?: string;
  lacis_id?: string;
  active: boolean;
  last_seen_at: string;
  synced_at: string;
  created_at: string;
  updated_at: string;
}

export interface OpenWrtSummary {
  total_routers: number;
  online_routers: number;
  total_clients: number;
  active_clients: number;
}

// ============================================================================
// External Devices
// ============================================================================

export interface ExternalDeviceDoc {
  device_id: string;
  display_name: string;
  mac: string;
  ip: string;
  protocol: string;
  username?: string;
  password?: string;
  status: string;
  device_model?: string;
  client_count: number;
  last_error?: string;
  omada_controller_id?: string;
  omada_site_id?: string;
  lacis_id?: string;
  product_type: string;
  network_device_type: string;
  last_polled_at?: string;
  created_at: string;
  updated_at: string;
}

export interface ExternalClientDoc {
  mac: string;
  device_id: string;
  ip?: string;
  hostname?: string;
  lacis_id?: string;
  active: boolean;
  last_seen_at: string;
  synced_at: string;
  created_at: string;
  updated_at: string;
}

export interface ExternalSummary {
  total_devices: number;
  online_devices: number;
  total_clients: number;
  active_clients: number;
}

// ============================================================================
// Omada (multi-controller types used by WireGuard page)
// ============================================================================

export interface OmadaSiteMapping {
  site_id: string;
  name: string;
  region?: string;
  fid?: string;
  tid?: string;
  fid_display_name?: string;
}

export interface OmadaControllerDoc {
  controller_id: string;
  display_name: string;
  base_url: string;
  client_id: string;
  client_secret: string;
  omadac_id: string;
  controller_ver: string;
  api_ver: string;
  status: string;
  last_error?: string;
  sites: OmadaSiteMapping[];
  last_synced_at?: string;
  created_at: string;
  updated_at: string;
}

export interface OmadaWgPeerDoc {
  peer_id: string;
  controller_id: string;
  site_id: string;
  name: string;
  status: boolean;
  interface_id: string;
  interface_name: string;
  public_key: string;
  allow_address: string[];
  keep_alive: number;
  comment?: string;
  synced_at: string;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// WireGuard
// ============================================================================

export interface WgKeyPair {
  private_key: string;
  public_key: string;
}

export interface WgInterface {
  interface_id: string;
  interface_name: string;
  controller_id: string;
  site_id: string;
  peer_count: number;
  active_peers: number;
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
