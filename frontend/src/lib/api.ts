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

// --- Multi-controller types ---

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

export interface OmadaDeviceDoc {
  mac: string;
  controller_id: string;
  site_id: string;
  name: string;
  device_type: string;
  model?: string;
  ip?: string;
  status: number;
  firmware_version?: string;
  lacis_id?: string;
  product_type: string;
  network_device_type: string;
  synced_at: string;
  created_at: string;
  updated_at: string;
}

export interface OmadaClientDoc {
  mac: string;
  controller_id: string;
  site_id: string;
  name?: string;
  host_name?: string;
  ip?: string;
  ipv6_list: string[];
  vendor?: string;
  device_type?: string;
  device_category?: string;
  os_name?: string;
  model?: string;
  connect_type?: number;
  wireless: boolean;
  ssid?: string;
  signal_level?: number;
  rssi?: number;
  ap_mac?: string;
  ap_name?: string;
  wifi_mode?: number;
  channel?: number;
  switch_mac?: string;
  switch_name?: string;
  port?: number;
  vid?: number;
  traffic_down: number;
  traffic_up: number;
  uptime: number;
  active: boolean;
  blocked: boolean;
  guest: boolean;
  lacis_id?: string;
  last_seen_at?: string;
  synced_at: string;
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

export interface OmadaSummary {
  total_controllers: number;
  connected_controllers: number;
  total_devices: number;
  online_devices: number;
  total_clients: number;
  active_clients: number;
  total_wg_peers: number;
  active_wg_peers: number;
}

export interface OmadaTestResult {
  success: boolean;
  controller_ver?: string;
  api_ver?: string;
  omadac_id?: string;
  sites: { site_id: string; name: string }[];
  device_count: number;
  error?: string;
}

export const omadaApi = {
  // Legacy compatibility
  getStatus: () => request<NetworkStatus>('/omada/status'),

  testConnection: () =>
    request<{ success: boolean; message: string; devices?: number }>('/omada/test', {
      method: 'POST',
    }),

  // Controller management
  listControllers: () =>
    request<{ ok: boolean; controllers: OmadaControllerDoc[]; error?: string }>('/omada/controllers'),

  getController: (id: string) =>
    request<{ ok: boolean; controller?: OmadaControllerDoc; error?: string }>(`/omada/controllers/${id}`),

  registerController: (data: {
    display_name: string;
    base_url: string;
    client_id: string;
    client_secret: string;
  }) =>
    request<{ ok: boolean; controller?: OmadaControllerDoc; error?: string }>('/omada/controllers', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  deleteController: (id: string) =>
    request<{ ok: boolean; message?: string; error?: string }>(`/omada/controllers/${id}`, {
      method: 'DELETE',
    }),

  testControllerConnection: (data: {
    base_url: string;
    client_id: string;
    client_secret: string;
  }) =>
    request<OmadaTestResult>('/omada/controllers/test', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  syncController: (id: string) =>
    request<{ ok: boolean; message?: string; error?: string }>(`/omada/controllers/${id}/sync`, {
      method: 'POST',
    }),

  // Data viewing
  getDevices: (controllerId?: string, siteId?: string) => {
    const query = new URLSearchParams();
    if (controllerId) query.set('controller_id', controllerId);
    if (siteId) query.set('site_id', siteId);
    const qs = query.toString();
    return request<{ ok: boolean; devices: OmadaDeviceDoc[]; total: number; error?: string }>(
      `/omada/devices${qs ? `?${qs}` : ''}`
    );
  },

  getClients: (controllerId?: string, siteId?: string, active?: boolean) => {
    const query = new URLSearchParams();
    if (controllerId) query.set('controller_id', controllerId);
    if (siteId) query.set('site_id', siteId);
    if (active !== undefined) query.set('active', active.toString());
    const qs = query.toString();
    return request<{ ok: boolean; clients: OmadaClientDoc[]; total: number; error?: string }>(
      `/omada/clients${qs ? `?${qs}` : ''}`
    );
  },

  getWireguard: (controllerId?: string, siteId?: string) => {
    const query = new URLSearchParams();
    if (controllerId) query.set('controller_id', controllerId);
    if (siteId) query.set('site_id', siteId);
    const qs = query.toString();
    return request<{ ok: boolean; peers: OmadaWgPeerDoc[]; total: number; error?: string }>(
      `/omada/wireguard${qs ? `?${qs}` : ''}`
    );
  },

  getSummary: () =>
    request<{ ok: boolean; summary: OmadaSummary; error?: string }>('/omada/summary'),
};

// ============================================================================
// OpenWrt API
// ============================================================================

export const openwrtApi = {
  listRouters: () =>
    request<{ ok: boolean; routers: import('@/types').OpenWrtRouterDoc[]; total: number; error?: string }>('/openwrt/routers'),

  getRouter: (id: string) =>
    request<{ ok: boolean; router?: import('@/types').OpenWrtRouterDoc; error?: string }>(`/openwrt/routers/${id}`),

  registerRouter: (data: {
    display_name: string;
    mac: string;
    ip: string;
    port?: number;
    username: string;
    password: string;
    firmware: string;
  }) =>
    request<{ ok: boolean; router?: import('@/types').OpenWrtRouterDoc; error?: string }>('/openwrt/routers', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  deleteRouter: (id: string) =>
    request<{ ok: boolean; message?: string; error?: string }>(`/openwrt/routers/${id}`, {
      method: 'DELETE',
    }),

  testConnection: (data: {
    ip: string;
    port?: number;
    username: string;
    password: string;
    firmware: string;
  }) =>
    request<{ success: boolean; status?: unknown; error?: string }>('/openwrt/routers/test', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  pollRouter: (id: string) =>
    request<{ ok: boolean; message?: string; error?: string }>(`/openwrt/routers/${id}/poll`, {
      method: 'POST',
    }),

  getClients: (routerId?: string) => {
    const query = new URLSearchParams();
    if (routerId) query.set('router_id', routerId);
    const qs = query.toString();
    return request<{ ok: boolean; clients: import('@/types').OpenWrtClientDoc[]; total: number; error?: string }>(
      `/openwrt/clients${qs ? `?${qs}` : ''}`
    );
  },

  getSummary: () =>
    request<{ ok: boolean; summary: import('@/types').OpenWrtSummary; error?: string }>('/openwrt/summary'),
};

// ============================================================================
// WireGuard API
// ============================================================================

export const wireguardApi = {
  generateKeypair: () =>
    request<{ ok: boolean; private_key: string; public_key: string }>('/wireguard/keypair', {
      method: 'POST',
    }),

  createPeer: (data: {
    controller_id: string;
    site_id: string;
    name: string;
    interface_id: string;
    public_key: string;
    allow_address: string[];
    keep_alive?: number;
    comment?: string;
  }) =>
    request<{ ok: boolean; peer?: unknown; error?: string }>('/wireguard/peers', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  getPeers: (controllerId?: string, siteId?: string) => {
    const query = new URLSearchParams();
    if (controllerId) query.set('controller_id', controllerId);
    if (siteId) query.set('site_id', siteId);
    const qs = query.toString();
    return request<{ ok: boolean; peers: OmadaWgPeerDoc[]; total: number; error?: string }>(
      `/wireguard/peers${qs ? `?${qs}` : ''}`
    );
  },

  updatePeer: (peerId: string, data: {
    controller_id: string;
    site_id: string;
    name?: string;
    allow_address?: string[];
    keep_alive?: number;
    comment?: string;
  }) =>
    request<{ ok: boolean; message?: string; error?: string }>(`/wireguard/peers/${peerId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    }),

  deletePeer: (peerId: string, controllerId: string, siteId: string) =>
    request<{ ok: boolean; message?: string; error?: string }>(
      `/wireguard/peers/${peerId}?controller_id=${controllerId}&site_id=${siteId}`,
      { method: 'DELETE' }
    ),

  generateConfig: (params: {
    private_key: string;
    address: string;
    dns: string;
    server_public_key: string;
    endpoint: string;
    allowed_ips: string;
    persistent_keepalive?: number;
  }) =>
    request<{ ok: boolean; config: string }>('/wireguard/config', {
      method: 'POST',
      body: JSON.stringify(params),
    }),

  getInterfaces: (controllerId?: string, siteId?: string) => {
    const query = new URLSearchParams();
    if (controllerId) query.set('controller_id', controllerId);
    if (siteId) query.set('site_id', siteId);
    const qs = query.toString();
    return request<{ ok: boolean; interfaces: import('@/types').WgInterface[]; total: number; error?: string }>(
      `/wireguard/interfaces${qs ? `?${qs}` : ''}`
    );
  },
};

// ============================================================================
// External Devices API
// ============================================================================

export const externalApi = {
  listDevices: () =>
    request<{ ok: boolean; devices: import('@/types').ExternalDeviceDoc[]; total: number; error?: string }>('/external/devices'),

  getDevice: (id: string) =>
    request<{ ok: boolean; device?: import('@/types').ExternalDeviceDoc; error?: string }>(`/external/devices/${id}`),

  registerDevice: (data: {
    display_name: string;
    mac: string;
    ip: string;
    protocol: string;
    username?: string;
    password?: string;
  }) =>
    request<{ ok: boolean; device?: import('@/types').ExternalDeviceDoc; error?: string }>('/external/devices', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  deleteDevice: (id: string) =>
    request<{ ok: boolean; message?: string; error?: string }>(`/external/devices/${id}`, {
      method: 'DELETE',
    }),

  testConnection: (data: {
    ip: string;
    protocol: string;
    username?: string;
    password?: string;
  }) =>
    request<{ success: boolean; error?: string; model?: string; firmware?: string }>('/external/devices/test', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  pollDevice: (id: string) =>
    request<{ ok: boolean; message?: string; error?: string }>(`/external/devices/${id}/poll`, {
      method: 'POST',
    }),

  getClients: (deviceId?: string) => {
    const query = new URLSearchParams();
    if (deviceId) query.set('device_id', deviceId);
    const qs = query.toString();
    return request<{ ok: boolean; clients: import('@/types').ExternalClientDoc[]; total: number; error?: string }>(
      `/external/clients${qs ? `?${qs}` : ''}`
    );
  },

  getSummary: () =>
    request<{ ok: boolean; summary: import('@/types').ExternalSummary; error?: string }>('/external/summary'),
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

// ============================================================================
// Topology API (CelestialGlobe)
// ============================================================================

export interface TopologyNode {
  id: string;
  label: string;
  node_type: string;
  mac?: string;
  ip?: string;
  source: string;
  parent_id?: string;
  lacis_id?: string;
  candidate_lacis_id?: string;
  product_type?: string;
  network_device_type?: string;
  status: string;
  metadata: Record<string, unknown>;
}

export interface TopologyEdge {
  from: string;
  to: string;
  edge_type: string;
  label?: string;
}

export interface TopologyMetadata {
  total_devices: number;
  total_clients: number;
  controllers: number;
  routers: number;
  generated_at: string;
}

export interface TopologyResponse {
  nodes: TopologyNode[];
  edges: TopologyEdge[];
  metadata: TopologyMetadata;
}

export const topologyApi = {
  getTopology: () => request<TopologyResponse>('/topology'),
};

// ============================================================================
// Operation Logs API
// ============================================================================

export interface OperationLog {
  operation_id: string;
  operation_type: string;
  initiated_by: string;
  target?: string;
  status: string;
  result?: Record<string, unknown>;
  error?: string;
  duration_ms?: number;
  created_at: string;
}

export interface OperationLogSummary {
  total: number;
  recent_24h: number;
  recent_errors: number;
  recent_success: number;
  generated_at: string;
}

export const operationLogsApi = {
  list: (params?: { operation_type?: string; status?: string; from?: string; to?: string; limit?: number; offset?: number }) => {
    const query = new URLSearchParams();
    if (params?.operation_type) query.set('operation_type', params.operation_type);
    if (params?.status) query.set('status', params.status);
    if (params?.from) query.set('from', params.from);
    if (params?.to) query.set('to', params.to);
    if (params?.limit !== undefined) query.set('limit', params.limit.toString());
    if (params?.offset !== undefined) query.set('offset', params.offset.toString());
    const qs = query.toString();
    return request<OperationLog[]>(`/logs/operations${qs ? `?${qs}` : ''}`);
  },

  get: (id: string) => request<OperationLog>(`/logs/operations/${id}`),

  getSummary: () => request<OperationLogSummary>('/logs/operations/summary'),
};

// ============================================================================
// Tools API
// ============================================================================

export interface ToolResult {
  ok: boolean;
  operation_id?: string;
  message?: string;
  result?: unknown;
  error?: string;
}

// --- Diagnostics types ---

export interface DiagnosticCheck {
  category: string;
  name: string;
  status: 'ok' | 'warning' | 'error';
  message: string;
  details?: Record<string, unknown>;
  duration_ms: number;
}

export interface DiagnosticSummary {
  total: number;
  ok: number;
  warning: number;
  error: number;
}

export interface DiagnosticsResponse {
  checks: DiagnosticCheck[];
  summary: DiagnosticSummary;
  operation_id: string;
  duration_ms: number;
}

export interface DiagnosticsRequest {
  categories?: string[];
  include_device_tests?: boolean;
}

export const toolsApi = {
  syncOmada: () => request<ToolResult>('/tools/sync/omada', { method: 'POST' }),
  syncOpenwrt: () => request<ToolResult>('/tools/sync/openwrt', { method: 'POST' }),
  syncExternal: () => request<ToolResult>('/tools/sync/external', { method: 'POST' }),
  ddnsUpdateAll: () => request<ToolResult>('/tools/ddns/update-all', { method: 'POST' }),
  ping: (host: string) => request<ToolResult>('/tools/network/ping', { method: 'POST', body: JSON.stringify({ host }) }),
  dns: (hostname: string) => request<ToolResult>('/tools/network/dns', { method: 'POST', body: JSON.stringify({ hostname }) }),
  curl: (url: string) => request<ToolResult>('/tools/network/curl', { method: 'POST', body: JSON.stringify({ url }) }),
  omadaApiRef: () => request<{ methods: { name: string; endpoint: string; method: string; description: string }[] }>('/tools/omada/api-ref'),

  diagnostics: (params?: DiagnosticsRequest) =>
    request<DiagnosticsResponse>('/tools/diagnostics', {
      method: 'POST',
      body: JSON.stringify(params ?? {}),
    }),
};

// ============================================================================
// LacisID API
// ============================================================================

export interface LacisIdCandidate {
  device_id: string;
  source: string;
  mac: string;
  display_name: string;
  product_type: string;
  network_device_type: string;
  candidate_lacis_id: string;
  assigned_lacis_id?: string;
  status: string;
}

export const lacisIdApi = {
  candidates: () => request<LacisIdCandidate[]>('/lacis-id/candidates'),
  compute: (mac: string, product_type: string, product_code?: string) =>
    request<{ lacis_id: string }>('/lacis-id/compute', {
      method: 'POST',
      body: JSON.stringify({ mac, product_type, product_code }),
    }),
  assign: (deviceId: string, source: string, lacisId: string) =>
    request<SuccessResponse>(`/lacis-id/assign/${deviceId}`, {
      method: 'POST',
      body: JSON.stringify({ source, lacis_id: lacisId }),
    }),
};

// ============================================================================
// Aranea SDK API
// ============================================================================

export interface AraneaDevice {
  lacis_id: string;
  mac: string;
  product_type: string;
  product_code: string;
  device_type: string;
  health_status?: string;
  mqtt_connected?: boolean;
  last_seen?: string;
}

export interface AraneaSummary {
  total: number;
  online: number;
  offline: number;
  mqtt_connected: number;
}

export const araneaApi = {
  listDevices: () =>
    request<{ ok: boolean; devices: AraneaDevice[]; error?: string }>('/aranea/devices'),
  getDeviceState: (lacisId: string) =>
    request<{ ok: boolean; states: unknown[]; error?: string }>(`/aranea/devices/${lacisId}/state`),
  register: (data: { mac: string; product_type: string; product_code: string; device_type: string }) =>
    request<{ ok: boolean; lacis_id?: string; error?: string }>('/aranea/register', {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  getSummary: () =>
    request<{ ok: boolean; summary: AraneaSummary; error?: string }>('/aranea/summary'),
};

// ============================================================================
// DDNS Integrated API (Phase 2 extensions)
// ============================================================================

export interface DdnsIntegrated {
  config: DdnsConfig;
  omada_wan_ip?: string;
  resolved_ip?: string;
  ip_mismatch: boolean;
  port_forwarding: unknown[];
  linked_controller?: string;
}

export const ddnsIntegratedApi = {
  list: () => request<DdnsIntegrated[]>('/ddns/integrated'),
  linkOmada: (id: number, data: { omada_controller_id: string; omada_site_id: string }) =>
    request<SuccessResponse>(`/ddns/${id}/link-omada`, {
      method: 'PUT',
      body: JSON.stringify(data),
    }),
  getPortForwards: (id: number) =>
    request<{ rules: unknown[] }>(`/ddns/${id}/port-forwards`),
};

// ============================================================================
// Server Routes API (Phase 2 extension)
// ============================================================================

export interface ServerRoute {
  id: number;
  path: string;
  target: string;
  active: boolean;
  strip_prefix: boolean;
  preserve_host: boolean;
  priority: number;
  timeout_ms?: number;
  websocket_support: boolean;
  ddns_config_id?: number;
  subnet?: {
    network: string;
    gateway: string;
    vlan_id?: number;
    controller_id?: string;
    site_name?: string;
  };
  fid?: string;
  tid?: string;
}

export const serverRoutesApi = {
  list: () => request<ServerRoute[]>('/server-routes'),
};
