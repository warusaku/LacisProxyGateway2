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
} from '@/types';

const API_BASE = '/LacisProxyGateway2/api';

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    ...options,
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

export const dashboardApi = {
  getStats: () => request<DashboardStats>('/dashboard/stats'),

  getAccessLog: (limit = 50, offset = 0) =>
    request<AccessLog[]>(`/dashboard/access-log?limit=${limit}&offset=${offset}`),

  getFilteredAccessLog: (params: { limit?: number; path?: string; ip?: string }) => {
    const query = new URLSearchParams();
    if (params.limit) query.set('limit', params.limit.toString());
    if (params.path) query.set('path', params.path);
    if (params.ip) query.set('ip', params.ip);
    return request<AccessLog[]>(`/dashboard/access-log/filter?${query}`);
  },

  getHealth: () => request<RouteHealth[]>('/dashboard/health'),

  getStatusDistribution: () => request<StatusDistribution[]>('/dashboard/status-distribution'),

  getSslStatus: () => request<SslStatus>('/dashboard/ssl-status'),

  getServerHealth: () => request<ServerHealth>('/dashboard/server-health'),
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
};
