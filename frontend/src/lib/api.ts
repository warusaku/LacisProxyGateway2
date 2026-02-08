// LacisProxyGateway2 API Client

import type {
  ProxyRoute,
  CreateRouteRequest,
  UpdateRouteRequest,
  DdnsConfig,
  CreateDdnsRequest,
  UpdateDdnsRequest,
  BlockedIp,
  BlockIpRequest,
  SecurityEvent,
  Setting,
  DashboardStats,
  RouteHealth,
  AccessLog,
  StatusDistribution,
  SuccessResponse,
  HourlyStat,
  TopEntry,
  ErrorSummary,
  AccessLogSearchResult,
  AccessLogSearchParams,
  SecurityEventSearchParams,
  IpExclusionParams,
  AuthResponse,
  LacisOathConfig,
} from '@/types';

const API_BASE = '/LacisProxyGateway2/api';

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...options,
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(error.error || `HTTP ${response.status}`);
  }

  return response.json();
}

// ============================================================================
// Routes API
// ============================================================================

export interface RouteDetailedStatus {
  route_id: number;
  path: string;
  target: string;
  active: boolean;
  healthy: boolean;
  last_check: string | null;
  consecutive_failures: number;
  response_time_ms: number | null;
  last_status_code: number | null;
  requests_today: number;
  requests_last_hour: number;
  error_rate_percent: number;
  avg_response_time_ms: number;
}

export const routesApi = {
  list: () => request<ProxyRoute[]>('/routes'),

  get: (id: number) => request<ProxyRoute>(`/routes/${id}`),

  create: (data: CreateRouteRequest) =>
    request<SuccessResponse>('/routes', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  update: (id: number, data: UpdateRouteRequest) =>
    request<SuccessResponse>(`/routes/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    }),

  delete: (id: number) =>
    request<SuccessResponse>(`/routes/${id}`, {
      method: 'DELETE',
    }),

  // Status and health APIs
  getAllStatus: () => request<RouteDetailedStatus[]>('/routes/status'),

  getStatus: (id: number) => request<RouteDetailedStatus>(`/routes/${id}/status`),

  getLogs: (id: number, limit: number = 50) =>
    request<AccessLog[]>(`/routes/${id}/logs?limit=${limit}`),
};

// ============================================================================
// DDNS API
// ============================================================================

export const ddnsApi = {
  list: () => request<DdnsConfig[]>('/ddns'),

  get: (id: number) => request<DdnsConfig>(`/ddns/${id}`),

  create: (data: CreateDdnsRequest) =>
    request<SuccessResponse>('/ddns', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  update: (id: number, data: UpdateDdnsRequest) =>
    request<SuccessResponse>(`/ddns/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    }),

  delete: (id: number) =>
    request<SuccessResponse>(`/ddns/${id}`, {
      method: 'DELETE',
    }),

  triggerUpdate: (id: number) =>
    request<SuccessResponse>(`/ddns/${id}/update`, {
      method: 'POST',
    }),
};

// ============================================================================
// Security API
// ============================================================================

export const securityApi = {
  listBlockedIps: () => request<BlockedIp[]>('/security/blocked-ips'),

  blockIp: (data: BlockIpRequest) =>
    request<SuccessResponse>('/security/blocked-ips', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  unblockIp: (id: number) =>
    request<SuccessResponse>(`/security/blocked-ips/${id}`, {
      method: 'DELETE',
    }),

  listEvents: (limit = 50, offset = 0) =>
    request<SecurityEvent[]>(`/security/events?limit=${limit}&offset=${offset}`),

  getEventsByIp: (ip: string) => request<SecurityEvent[]>(`/security/events/ip/${ip}`),

  searchEvents: (params: SecurityEventSearchParams) => {
    const query = new URLSearchParams();
    if (params.from) query.set('from', params.from);
    if (params.to) query.set('to', params.to);
    if (params.severity) query.set('severity', params.severity);
    if (params.event_type) query.set('event_type', params.event_type);
    if (params.ip) query.set('ip', params.ip);
    if (params.limit !== undefined) query.set('limit', params.limit.toString());
    if (params.offset !== undefined) query.set('offset', params.offset.toString());
    return request<SecurityEvent[]>(`/security/events/search?${query}`);
  },
};

// ============================================================================
// Settings API
// ============================================================================

export interface RestartSettings {
  scheduled_enabled: boolean;
  scheduled_time: string;
  auto_restart_enabled: boolean;
  cpu_threshold: number;
  ram_threshold: number;
}

export interface UpdateRestartSettingsRequest {
  scheduled_enabled?: boolean;
  scheduled_time?: string;
  auto_restart_enabled?: boolean;
  cpu_threshold?: number;
  ram_threshold?: number;
}

export const settingsApi = {
  list: () => request<Setting[]>('/settings'),

  update: (key: string, value: string | null) =>
    request<SuccessResponse>(`/settings/${key}`, {
      method: 'PUT',
      body: JSON.stringify({ value }),
    }),

  testDiscord: () =>
    request<SuccessResponse>('/settings/test-discord', {
      method: 'POST',
    }),

  getRestartSettings: () => request<RestartSettings>('/settings/restart'),

  updateRestartSettings: (data: UpdateRestartSettingsRequest) =>
    request<SuccessResponse>('/settings/restart', {
      method: 'PUT',
      body: JSON.stringify(data),
    }),

  triggerRestart: () =>
    request<SuccessResponse>('/settings/restart/trigger', {
      method: 'POST',
    }),
};

// ============================================================================
// Dashboard API
// ============================================================================

export interface SslStatus {
  enabled: boolean;
  domain?: string;
  issuer?: string;
  valid_from?: string;
  valid_until?: string;
  days_remaining?: number;
  auto_renew: boolean;
  last_renewal?: string;
  next_renewal_attempt?: string;
}

export interface ServerHealth {
  hostname: string;
  os: string;
  kernel: string;
  uptime: string;
  uptime_seconds: number;
  load_average: {
    one_min: number;
    five_min: number;
    fifteen_min: number;
  };
  cpu: {
    model: string;
    cores: number;
    usage_percent: number;
  };
  memory: {
    total_mb: number;
    used_mb: number;
    free_mb: number;
    available_mb: number;
    usage_percent: number;
  };
  swap: {
    total_mb: number;
    used_mb: number;
    free_mb: number;
    usage_percent: number;
  };
  disk: Array<{
    mount_point: string;
    filesystem: string;
    total_gb: number;
    used_gb: number;
    free_gb: number;
    usage_percent: number;
  }>;
  network: {
    interfaces: Array<{
      name: string;
      ip?: string;
      rx_bytes: number;
      tx_bytes: number;
    }>;
    connections: number;
  };
  processes: {
    total: number;
    running: number;
    sleeping: number;
  };
}

/** Append IP exclusion params to a URLSearchParams instance */
function appendExclusionParams(query: URLSearchParams, params?: IpExclusionParams) {
  if (params?.exclude_ips) query.set('exclude_ips', params.exclude_ips);
  if (params?.exclude_lan) query.set('exclude_lan', 'true');
}

export const dashboardApi = {
  getMyIp: () => request<{
    ip: string;
    server_ip?: string;
    server_ip_history: string[];
    admin_ip_history: string[];
  }>('/my-ip'),

  getStats: (exclusion?: IpExclusionParams) => {
    const query = new URLSearchParams();
    appendExclusionParams(query, exclusion);
    const qs = query.toString();
    return request<DashboardStats>(`/dashboard/stats${qs ? `?${qs}` : ''}`);
  },

  getAccessLog: (limit = 50, offset = 0, exclusion?: IpExclusionParams) => {
    const query = new URLSearchParams();
    query.set('limit', limit.toString());
    query.set('offset', offset.toString());
    appendExclusionParams(query, exclusion);
    return request<AccessLog[]>(`/dashboard/access-log?${query}`);
  },

  getFilteredAccessLog: (params: { limit?: number; path?: string; ip?: string }) => {
    const query = new URLSearchParams();
    if (params.limit) query.set('limit', params.limit.toString());
    if (params.path) query.set('path', params.path);
    if (params.ip) query.set('ip', params.ip);
    return request<AccessLog[]>(`/dashboard/access-log/filter?${query}`);
  },

  getHealth: () => request<RouteHealth[]>('/dashboard/health'),

  getStatusDistribution: (exclusion?: IpExclusionParams) => {
    const query = new URLSearchParams();
    appendExclusionParams(query, exclusion);
    const qs = query.toString();
    return request<StatusDistribution[]>(`/dashboard/status-distribution${qs ? `?${qs}` : ''}`);
  },

  getSslStatus: () => request<SslStatus>('/dashboard/ssl-status'),

  getServerHealth: () => request<ServerHealth>('/dashboard/server-health'),

  searchAccessLogs: (params: AccessLogSearchParams) => {
    const query = new URLSearchParams();
    if (params.from) query.set('from', params.from);
    if (params.to) query.set('to', params.to);
    if (params.method) query.set('method', params.method);
    if (params.status_min !== undefined) query.set('status_min', params.status_min.toString());
    if (params.status_max !== undefined) query.set('status_max', params.status_max.toString());
    if (params.ip) query.set('ip', params.ip);
    if (params.path) query.set('path', params.path);
    if (params.limit !== undefined) query.set('limit', params.limit.toString());
    if (params.offset !== undefined) query.set('offset', params.offset.toString());
    if (params.exclude_ips) query.set('exclude_ips', params.exclude_ips);
    if (params.exclude_lan) query.set('exclude_lan', 'true');
    return request<AccessLogSearchResult>(`/dashboard/access-log/search?${query}`);
  },

  getHourlyStats: (from?: string, to?: string, exclusion?: IpExclusionParams) => {
    const query = new URLSearchParams();
    if (from) query.set('from', from);
    if (to) query.set('to', to);
    appendExclusionParams(query, exclusion);
    return request<HourlyStat[]>(`/dashboard/hourly-stats?${query}`);
  },

  getTopIps: (from?: string, to?: string, limit?: number, exclusion?: IpExclusionParams) => {
    const query = new URLSearchParams();
    if (from) query.set('from', from);
    if (to) query.set('to', to);
    if (limit) query.set('limit', limit.toString());
    appendExclusionParams(query, exclusion);
    return request<TopEntry[]>(`/dashboard/top-ips?${query}`);
  },

  getTopPaths: (from?: string, to?: string, limit?: number, exclusion?: IpExclusionParams) => {
    const query = new URLSearchParams();
    if (from) query.set('from', from);
    if (to) query.set('to', to);
    if (limit) query.set('limit', limit.toString());
    appendExclusionParams(query, exclusion);
    return request<TopEntry[]>(`/dashboard/top-paths?${query}`);
  },

  getErrorSummary: (from?: string, to?: string, exclusion?: IpExclusionParams) => {
    const query = new URLSearchParams();
    if (from) query.set('from', from);
    if (to) query.set('to', to);
    appendExclusionParams(query, exclusion);
    return request<ErrorSummary[]>(`/dashboard/error-summary?${query}`);
  },

  exportCsv: async (params: AccessLogSearchParams) => {
    const query = new URLSearchParams();
    if (params.from) query.set('from', params.from);
    if (params.to) query.set('to', params.to);
    if (params.method) query.set('method', params.method);
    if (params.status_min !== undefined) query.set('status_min', params.status_min.toString());
    if (params.status_max !== undefined) query.set('status_max', params.status_max.toString());
    if (params.ip) query.set('ip', params.ip);
    if (params.path) query.set('path', params.path);
    if (params.limit !== undefined) query.set('limit', params.limit.toString());
    if (params.exclude_ips) query.set('exclude_ips', params.exclude_ips);
    if (params.exclude_lan) query.set('exclude_lan', 'true');
    const response = await fetch(`${API_BASE}/dashboard/access-log/export?${query}`, {
      credentials: 'include',
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const blob = await response.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'access_logs.csv';
    a.click();
    URL.revokeObjectURL(url);
  },
};

// ============================================================================
// Omada API
// ============================================================================

export interface OmadaDevice {
  mac: string;
  name: string;
  type: string;
  model?: string;
  ip?: string;
  status: number; // 0=offline, 1=online
  firmwareVersion?: string;
}

export interface NetworkStatus {
  gateway_online: boolean;
  gateway_ip?: string;
  wan_ip?: string;
  devices: OmadaDevice[];
  port_forwarding: unknown[];
  configured: boolean;
  error?: string;
}

export const omadaApi = {
  getStatus: () => request<NetworkStatus>('/omada/status'),

  testConnection: () =>
    request<{ success: boolean; message: string; devices?: number }>('/omada/test', {
      method: 'POST',
    }),
};

// ============================================================================
// Audit API
// ============================================================================

export interface AuditLog {
  id: number;
  entity_type: string;
  entity_id: number | null;
  action: string;
  field_name: string | null;
  old_value: string | null;
  new_value: string | null;
  changed_by: string;
  ip_address: string | null;
  created_at: string | null;
}

export const auditApi = {
  getLogs: (limit = 50, offset = 0) =>
    request<AuditLog[]>(`/audit?limit=${limit}&offset=${offset}`),
};

// ============================================================================
// Nginx API
// ============================================================================

export interface NginxStatus {
  running: boolean;
  config_valid: boolean;
  proxy_mode: string; // "selective" or "full_proxy"
  config_path: string | null;
  last_reload: string | null;
  error: string | null;
  client_max_body_size: string | null;
}

export interface NginxConfig {
  path: string;
  content: string;
}

export interface EnableFullProxyRequest {
  enable_full_proxy: boolean;
  backend_port?: number;
  server_name?: string;
}

export interface NginxTemplateSettings {
  server_name: string;
  backend_port: number;
  gzip_enabled: boolean;
  gzip_comp_level: number;
  gzip_min_length: number;
  proxy_connect_timeout: number;
  proxy_send_timeout: number;
  proxy_read_timeout: number;
  header_x_frame_options: string;
  header_x_content_type: string;
  header_xss_protection: string;
  header_hsts: string;
  header_referrer_policy: string;
  header_permissions_policy: string;
  header_csp: string;
}

// ============================================================================
// Auth API
// ============================================================================

export const authApi = {
  getLacisOathConfig: () => request<LacisOathConfig>('/auth/lacisoath-config'),

  loginLocal: (email: string, password: string) =>
    request<AuthResponse>('/auth/login/local', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    }),

  loginLacisOath: (code: string, redirectUri: string) =>
    request<AuthResponse>('/auth/login/lacisoath', {
      method: 'POST',
      body: JSON.stringify({ code, redirect_uri: redirectUri }),
    }),

  me: () => request<AuthResponse>('/auth/me'),

  logout: () =>
    request<{ ok: boolean }>('/auth/logout', {
      method: 'POST',
    }),
};

// ============================================================================
// Nginx API
// ============================================================================

export const nginxApi = {
  getStatus: () => request<NginxStatus>('/nginx/status'),

  getConfig: () => request<NginxConfig>('/nginx/config'),

  enableFullProxy: (data: EnableFullProxyRequest) =>
    request<SuccessResponse>('/nginx/enable-full-proxy', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  reload: () =>
    request<SuccessResponse>('/nginx/reload', {
      method: 'POST',
    }),

  test: () =>
    request<{ valid: boolean; error: string | null }>('/nginx/test', {
      method: 'POST',
    }),

  updateBodySize: (size: string) =>
    request<SuccessResponse>('/nginx/body-size', {
      method: 'PUT',
      body: JSON.stringify({ size }),
    }),

  getTemplateSettings: () =>
    request<NginxTemplateSettings>('/nginx/template-settings'),

  updateTemplateSettings: (data: Partial<NginxTemplateSettings>) =>
    request<SuccessResponse>('/nginx/template-settings', {
      method: 'PUT',
      body: JSON.stringify(data),
    }),

  regenerateConfig: () =>
    request<SuccessResponse>('/nginx/regenerate', {
      method: 'POST',
    }),
};
