'use client';

import { useEffect, useState } from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Table } from '@/components/ui/Table';
import { LogDetailModal } from '@/components/LogDetailModal';
import { dashboardApi, omadaApi, type NetworkStatus, type SslStatus, type ServerHealth } from '@/lib/api';
import type { DashboardStats, RouteHealth, AccessLog, HourlyStat, TopEntry, StatusDistribution, IpExclusionParams } from '@/types';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, PieChart, Pie, Cell } from 'recharts';
import { getStatusColor } from '@/lib/format';
import { countryCodeToFlag } from '@/lib/geo';

export default function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [health, setHealth] = useState<RouteHealth[]>([]);
  const [logs, setLogs] = useState<AccessLog[]>([]);
  const [network, setNetwork] = useState<NetworkStatus | null>(null);
  const [ssl, setSsl] = useState<SslStatus | null>(null);
  const [serverHealth, setServerHealth] = useState<ServerHealth | null>(null);
  const [hourlyStats, setHourlyStats] = useState<HourlyStat[]>([]);
  const [statusDist, setStatusDist] = useState<StatusDistribution[]>([]);
  const [topIps, setTopIps] = useState<TopEntry[]>([]);
  const [topPaths, setTopPaths] = useState<TopEntry[]>([]);
  const [selectedLog, setSelectedLog] = useState<AccessLog | null>(null);
  const [loading, setLoading] = useState(true);

  // IP exclusion filter state
  const [myIp, setMyIp] = useState<string>('');
  const [excludeMyIp, setExcludeMyIp] = useState<boolean>(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('lpg_exclude_my_ip') !== 'false'; // デフォルトON
    }
    return true;
  });
  const [excludeLan, setExcludeLan] = useState<boolean>(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('lpg_exclude_lan') === 'true'; // デフォルトOFF
    }
    return false;
  });

  // 自分のIPを取得
  useEffect(() => {
    dashboardApi.getMyIp().then(r => setMyIp(r.ip)).catch(() => {});
  }, []);

  // localStorage 永続化
  useEffect(() => {
    localStorage.setItem('lpg_exclude_my_ip', String(excludeMyIp));
  }, [excludeMyIp]);
  useEffect(() => {
    localStorage.setItem('lpg_exclude_lan', String(excludeLan));
  }, [excludeLan]);

  // 除外パラメータ構築ヘルパー
  const buildExclusionParams = (): IpExclusionParams => {
    const params: IpExclusionParams = {};
    if (excludeMyIp && myIp) params.exclude_ips = myIp;
    if (excludeLan) params.exclude_lan = true;
    return params;
  };

  useEffect(() => {
    loadDashboard();
    // Auto-refresh every 30 seconds
    const interval = setInterval(loadDashboard, 30000);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [excludeMyIp, excludeLan, myIp]);

  const loadDashboard = async () => {
    try {
      const exclusion = buildExclusionParams();
      const [statsData, healthData, logsData, networkData, sslData, serverHealthData] = await Promise.all([
        dashboardApi.getStats(exclusion),
        dashboardApi.getHealth(),
        dashboardApi.getAccessLog(20, 0, exclusion),
        omadaApi.getStatus().catch(() => null),
        dashboardApi.getSslStatus().catch(() => null),
        dashboardApi.getServerHealth().catch(() => null),
      ]);
      setStats(statsData);
      setHealth(healthData);
      setLogs(logsData);
      setNetwork(networkData);
      setSsl(sslData);
      setServerHealth(serverHealthData);

      // Load analytics data (non-blocking)
      Promise.all([
        dashboardApi.getHourlyStats(undefined, undefined, exclusion).catch(() => []),
        dashboardApi.getStatusDistribution(exclusion).catch(() => []),
        dashboardApi.getTopIps(undefined, undefined, 10, exclusion).catch(() => []),
        dashboardApi.getTopPaths(undefined, undefined, 10, exclusion).catch(() => []),
      ]).then(([hourly, dist, ips, paths]) => {
        setHourlyStats(hourly);
        setStatusDist(dist);
        setTopIps(ips);
        setTopPaths(paths);
      });
    } catch (err) {
      console.error('Failed to load dashboard:', err);
    } finally {
      setLoading(false);
    }
  };

  const formatUptime = (seconds: number) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);

    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
  };

  const healthColumns = [
    {
      key: 'path',
      header: 'Path',
      render: (h: RouteHealth) => <code className="text-blue-400">{h.path}</code>,
    },
    {
      key: 'target',
      header: 'Target',
      render: (h: RouteHealth) => (
        <span className="text-sm text-gray-400">{h.target}</span>
      ),
    },
    {
      key: 'healthy',
      header: 'Status',
      render: (h: RouteHealth) => (
        <Badge variant={h.healthy ? 'success' : 'error'}>
          {h.healthy ? 'Healthy' : 'Unhealthy'}
        </Badge>
      ),
    },
    {
      key: 'failures',
      header: 'Failures',
      render: (h: RouteHealth) => (
        <span className={h.consecutive_failures > 0 ? 'text-red-400' : ''}>
          {h.consecutive_failures}
        </span>
      ),
    },
  ];

  const logColumns = [
    {
      key: 'timestamp',
      header: 'Time',
      render: (log: AccessLog) => (
        <span className="text-sm text-gray-400 whitespace-nowrap">
          {new Date(log.timestamp).toLocaleTimeString()}
        </span>
      ),
    },
    {
      key: 'request',
      header: 'Request',
      render: (log: AccessLog) => (
        <div className="flex items-center gap-2">
          <span className={`font-mono text-xs ${getStatusColor(log.status)}`}>{log.status}</span>
          <span className="font-mono text-xs text-gray-400">{log.method}</span>
          <code className="text-sm truncate max-w-[200px] block">{log.path}</code>
        </div>
      ),
    },
    {
      key: 'ip',
      header: 'IP',
      render: (log: AccessLog) => (
        <code className="text-sm">{log.ip}</code>
      ),
    },
    {
      key: 'location',
      header: 'Location',
      render: (log: AccessLog) => (
        <span className="text-sm whitespace-nowrap">
          {log.country_code
            ? `${countryCodeToFlag(log.country_code)} ${log.city || log.country || log.country_code}`
            : '-'}
        </span>
      ),
    },
  ];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted">Loading...</div>
      </div>
    );
  }

  const getDeviceIcon = (type: string) => {
    switch (type) {
      case 'gateway': return '🌐';
      case 'switch': return '🔀';
      case 'ap': return '📡';
      default: return '📦';
    }
  };

  return (
    <div>
      <h1 className="text-2xl font-bold mb-8">Dashboard</h1>

      {/* Network & SSL Status */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
        {/* Network Status */}
        {network?.configured && (
          <Card title="Network Status">
            <div className="flex flex-wrap items-center gap-6 mb-4">
              {/* Gateway Status */}
              <div className="flex items-center gap-2">
                <span className={`w-3 h-3 rounded-full ${network.gateway_online ? 'bg-green-500' : 'bg-red-500'}`}></span>
                <span className="text-sm">Gateway: {network.gateway_ip || 'N/A'}</span>
              </div>
              {/* WAN IP */}
              {network.wan_ip && (
                <div className="text-sm text-gray-400">
                  WAN: {network.wan_ip}
                </div>
              )}
            </div>
            {/* Devices */}
            <div className="flex flex-wrap gap-3">
              {network.devices.map((device) => (
                <div
                  key={device.mac}
                  className={`flex items-center gap-2 px-3 py-2 rounded-lg border ${
                    device.status === 1
                      ? 'border-green-500/30 bg-green-500/10'
                      : 'border-red-500/30 bg-red-500/10'
                  }`}
                >
                  <span>{getDeviceIcon(device.type)}</span>
                  <div>
                    <div className="text-sm font-medium">{device.name}</div>
                    <div className="text-xs text-gray-400">{device.ip || device.mac}</div>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        )}

        {/* SSL Certificate Status */}
        {ssl && (
          <Card title="SSL Certificate">
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <span className={`w-3 h-3 rounded-full ${ssl.enabled ? 'bg-green-500' : 'bg-gray-500'}`}></span>
                <span className="text-sm font-medium">
                  {ssl.enabled ? 'HTTPS Enabled' : 'HTTPS Not Configured'}
                </span>
              </div>
              {ssl.enabled && (
                <>
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400">Domain</div>
                      <div className="font-mono">{ssl.domain}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Issuer</div>
                      <div>{ssl.issuer || 'N/A'}</div>
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400">Valid Until</div>
                      <div>{ssl.valid_until || 'N/A'}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Days Remaining</div>
                      <div className={`font-bold ${
                        ssl.days_remaining && ssl.days_remaining < 14
                          ? 'text-red-400'
                          : ssl.days_remaining && ssl.days_remaining < 30
                            ? 'text-yellow-400'
                            : 'text-green-400'
                      }`}>
                        {ssl.days_remaining ?? 'N/A'}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-4 text-sm pt-2 border-t border-gray-700">
                    <div className="flex items-center gap-2">
                      <span className={`w-2 h-2 rounded-full ${ssl.auto_renew ? 'bg-green-500' : 'bg-yellow-500'}`}></span>
                      <span>{ssl.auto_renew ? 'Auto-renew Active' : 'Auto-renew Inactive'}</span>
                    </div>
                    {ssl.next_renewal_attempt && (
                      <div className="text-gray-400">{ssl.next_renewal_attempt}</div>
                    )}
                  </div>
                </>
              )}
            </div>
          </Card>
        )}
      </div>

      {/* Server Health */}
      {serverHealth && (
        <Card title="Server Health" className="mb-8">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            {/* System Info */}
            <div className="space-y-2">
              <h4 className="text-sm font-semibold text-gray-400 uppercase">System</h4>
              <div className="text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Hostname</span>
                  <span className="font-mono">{serverHealth.hostname}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">OS</span>
                  <span className="text-xs">{serverHealth.os}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Kernel</span>
                  <span className="font-mono text-xs">{serverHealth.kernel}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Uptime</span>
                  <span className="text-green-400 font-bold">{serverHealth.uptime}</span>
                </div>
              </div>
            </div>

            {/* CPU */}
            <div className="space-y-2">
              <h4 className="text-sm font-semibold text-gray-400 uppercase">CPU</h4>
              <div className="text-sm">
                <div className="text-xs text-gray-400 truncate mb-1">{serverHealth.cpu.model}</div>
                <div className="flex justify-between mb-1">
                  <span className="text-gray-400">Cores</span>
                  <span>{serverHealth.cpu.cores}</span>
                </div>
                <div className="flex justify-between mb-1">
                  <span className="text-gray-400">Usage</span>
                  <span className={`font-bold ${
                    serverHealth.cpu.usage_percent > 80 ? 'text-red-400' :
                    serverHealth.cpu.usage_percent > 50 ? 'text-yellow-400' : 'text-green-400'
                  }`}>{serverHealth.cpu.usage_percent.toFixed(1)}%</span>
                </div>
                <div className="w-full bg-gray-700 rounded-full h-2">
                  <div
                    className={`h-2 rounded-full ${
                      serverHealth.cpu.usage_percent > 80 ? 'bg-red-500' :
                      serverHealth.cpu.usage_percent > 50 ? 'bg-yellow-500' : 'bg-green-500'
                    }`}
                    style={{ width: `${Math.min(serverHealth.cpu.usage_percent, 100)}%` }}
                  ></div>
                </div>
                <div className="text-xs text-gray-500 mt-1">
                  Load: {serverHealth.load_average.one_min.toFixed(2)} / {serverHealth.load_average.five_min.toFixed(2)} / {serverHealth.load_average.fifteen_min.toFixed(2)}
                </div>
              </div>
            </div>

            {/* Memory */}
            <div className="space-y-2">
              <h4 className="text-sm font-semibold text-gray-400 uppercase">Memory</h4>
              <div className="text-sm">
                <div className="flex justify-between mb-1">
                  <span className="text-gray-400">Used / Total</span>
                  <span>{(serverHealth.memory.used_mb / 1024).toFixed(1)}GB / {(serverHealth.memory.total_mb / 1024).toFixed(1)}GB</span>
                </div>
                <div className="flex justify-between mb-1">
                  <span className="text-gray-400">Usage</span>
                  <span className={`font-bold ${
                    serverHealth.memory.usage_percent > 80 ? 'text-red-400' :
                    serverHealth.memory.usage_percent > 60 ? 'text-yellow-400' : 'text-green-400'
                  }`}>{serverHealth.memory.usage_percent.toFixed(1)}%</span>
                </div>
                <div className="w-full bg-gray-700 rounded-full h-2">
                  <div
                    className={`h-2 rounded-full ${
                      serverHealth.memory.usage_percent > 80 ? 'bg-red-500' :
                      serverHealth.memory.usage_percent > 60 ? 'bg-yellow-500' : 'bg-green-500'
                    }`}
                    style={{ width: `${Math.min(serverHealth.memory.usage_percent, 100)}%` }}
                  ></div>
                </div>
                <div className="text-xs text-gray-500 mt-1">
                  Available: {(serverHealth.memory.available_mb / 1024).toFixed(1)}GB
                </div>
                {serverHealth.swap.total_mb > 0 && (
                  <div className="text-xs text-gray-500">
                    Swap: {(serverHealth.swap.used_mb / 1024).toFixed(1)}GB / {(serverHealth.swap.total_mb / 1024).toFixed(1)}GB ({serverHealth.swap.usage_percent.toFixed(0)}%)
                  </div>
                )}
              </div>
            </div>

            {/* Disk */}
            <div className="space-y-2">
              <h4 className="text-sm font-semibold text-gray-400 uppercase">Storage</h4>
              <div className="text-sm space-y-2">
                {serverHealth.disk.map((disk, i) => (
                  <div key={i}>
                    <div className="flex justify-between text-xs mb-1">
                      <span className="text-gray-400 font-mono">{disk.mount_point}</span>
                      <span className={`font-bold ${
                        disk.usage_percent > 90 ? 'text-red-400' :
                        disk.usage_percent > 70 ? 'text-yellow-400' : 'text-green-400'
                      }`}>{disk.usage_percent.toFixed(0)}%</span>
                    </div>
                    <div className="w-full bg-gray-700 rounded-full h-1.5">
                      <div
                        className={`h-1.5 rounded-full ${
                          disk.usage_percent > 90 ? 'bg-red-500' :
                          disk.usage_percent > 70 ? 'bg-yellow-500' : 'bg-green-500'
                        }`}
                        style={{ width: `${Math.min(disk.usage_percent, 100)}%` }}
                      ></div>
                    </div>
                    <div className="text-xs text-gray-500">
                      {disk.used_gb.toFixed(0)}GB / {disk.total_gb.toFixed(0)}GB
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Network & Processes Row */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mt-6 pt-4 border-t border-gray-700">
            {/* Network */}
            <div className="space-y-2">
              <h4 className="text-sm font-semibold text-gray-400 uppercase">Network</h4>
              <div className="text-sm">
                <div className="flex justify-between mb-2">
                  <span className="text-gray-400">Active Connections</span>
                  <span className="font-bold text-blue-400">{serverHealth.network.connections}</span>
                </div>
                <div className="space-y-1">
                  {serverHealth.network.interfaces.map((iface, i) => (
                    <div key={i} className="flex justify-between text-xs">
                      <span className="text-gray-400 font-mono">{iface.name}</span>
                      <span>{iface.ip || 'N/A'}</span>
                      <span className="text-gray-500">
                        RX: {(iface.rx_bytes / 1024 / 1024 / 1024).toFixed(1)}GB / TX: {(iface.tx_bytes / 1024 / 1024 / 1024).toFixed(1)}GB
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Processes */}
            <div className="space-y-2">
              <h4 className="text-sm font-semibold text-gray-400 uppercase">Processes</h4>
              <div className="text-sm flex gap-6">
                <div>
                  <div className="text-2xl font-bold">{serverHealth.processes.total}</div>
                  <div className="text-xs text-gray-400">Total</div>
                </div>
                <div>
                  <div className="text-2xl font-bold text-green-400">{serverHealth.processes.running}</div>
                  <div className="text-xs text-gray-400">Running</div>
                </div>
                <div>
                  <div className="text-2xl font-bold text-blue-400">{serverHealth.processes.sleeping}</div>
                  <div className="text-xs text-gray-400">Sleeping</div>
                </div>
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* IP Exclusion Filter */}
      <Card className="mb-6">
        <div className="flex flex-wrap items-center gap-6">
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={excludeMyIp}
              onChange={(e) => setExcludeMyIp(e.target.checked)}
              className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-500 focus:ring-blue-500 focus:ring-offset-0"
            />
            <span className="text-sm">
              自分のIPを除外
              {myIp && <code className="ml-1 text-xs text-gray-400">({myIp})</code>}
            </span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={excludeLan}
              onChange={(e) => setExcludeLan(e.target.checked)}
              className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-500 focus:ring-blue-500 focus:ring-offset-0"
            />
            <span className="text-sm">LANアクセスを除外</span>
          </label>
        </div>
      </Card>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4 mb-8">
        <Card className="text-center">
          <div className="text-3xl font-bold">{stats?.total_requests_today.toLocaleString() ?? 0}</div>
          <div className="text-sm text-gray-400">Requests Today</div>
        </Card>
        <Card className="text-center">
          <div className="text-3xl font-bold text-blue-400">{stats?.active_routes ?? 0}</div>
          <div className="text-sm text-gray-400">Active Routes</div>
        </Card>
        <Card className="text-center">
          <div className="text-3xl font-bold text-purple-400">{stats?.active_ddns ?? 0}</div>
          <div className="text-sm text-gray-400">Active DDNS</div>
        </Card>
        <Card className="text-center">
          <div className="text-3xl font-bold text-red-400">{stats?.blocked_ips ?? 0}</div>
          <div className="text-sm text-gray-400">Blocked IPs</div>
        </Card>
        <Card className="text-center">
          <div className="text-3xl font-bold text-green-400">{stats ? formatUptime(stats.uptime_seconds) : '-'}</div>
          <div className="text-sm text-gray-400">Uptime</div>
        </Card>
      </div>

      {/* Charts: Request Timeline & Status Distribution */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
        {/* Request Timeline Chart */}
        <Card title="Request Timeline (24h)" className="lg:col-span-2">
          {hourlyStats.length > 0 ? (
            <ResponsiveContainer width="100%" height={250}>
              <LineChart data={hourlyStats}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis
                  dataKey="hour"
                  stroke="#666"
                  tick={{ fontSize: 11 }}
                  tickFormatter={(v: string) => v ? new Date(v).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : ''}
                />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                <Tooltip
                  contentStyle={{ backgroundColor: '#1a1a1a', border: '1px solid #333', borderRadius: '8px' }}
                  labelFormatter={(v) => typeof v === 'string' && v ? new Date(v).toLocaleString() : ''}
                />
                <Line type="monotone" dataKey="total_requests" stroke="#3b82f6" name="Total" strokeWidth={2} dot={false} />
                <Line type="monotone" dataKey="error_count" stroke="#ef4444" name="Errors" strokeWidth={2} dot={false} />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-[250px] text-gray-500">No data</div>
          )}
        </Card>

        {/* Status Code Distribution Pie Chart */}
        <Card title="Status Distribution">
          {statusDist.length > 0 ? (
            <ResponsiveContainer width="100%" height={250}>
              <PieChart>
                <Pie
                  data={statusDist.map(d => ({ name: `${d.status}`, value: d.count }))}
                  cx="50%"
                  cy="50%"
                  innerRadius={50}
                  outerRadius={80}
                  paddingAngle={2}
                  dataKey="value"
                  label={({ name, percent }: { name?: string; percent?: number }) => `${name || ''} (${((percent || 0) * 100).toFixed(0)}%)`}
                >
                  {statusDist.map((d, i) => {
                    const color = d.status >= 500 ? '#ef4444' : d.status >= 400 ? '#eab308' : d.status >= 300 ? '#3b82f6' : '#10b981';
                    return <Cell key={i} fill={color} />;
                  })}
                </Pie>
                <Tooltip contentStyle={{ backgroundColor: '#1a1a1a', border: '1px solid #333', borderRadius: '8px' }} />
              </PieChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-[250px] text-gray-500">No data</div>
          )}
        </Card>
      </div>

      {/* Top IPs & Top Paths */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
        <Card title="Top IPs (24h)">
          <div className="space-y-2">
            {topIps.length > 0 ? topIps.map((entry, i) => (
              <div key={entry.key} className="flex items-center justify-between text-sm py-1 border-b border-gray-800 last:border-0">
                <div className="flex items-center gap-2">
                  <span className="text-gray-500 w-6 text-right">{i + 1}.</span>
                  <code className="text-blue-400">{entry.key}</code>
                </div>
                <div className="flex items-center gap-4">
                  <span>{entry.count.toLocaleString()} req</span>
                  {entry.error_count > 0 && (
                    <span className="text-red-400">{entry.error_count} err</span>
                  )}
                </div>
              </div>
            )) : (
              <div className="text-gray-500 text-sm">No data</div>
            )}
          </div>
        </Card>

        <Card title="Top Paths (24h)">
          <div className="space-y-2">
            {topPaths.length > 0 ? topPaths.map((entry, i) => (
              <div key={entry.key} className="flex items-center justify-between text-sm py-1 border-b border-gray-800 last:border-0">
                <div className="flex items-center gap-2 min-w-0">
                  <span className="text-gray-500 w-6 text-right flex-shrink-0">{i + 1}.</span>
                  <code className="text-blue-400 truncate">{entry.key}</code>
                </div>
                <div className="flex items-center gap-4 flex-shrink-0">
                  <span>{entry.count.toLocaleString()} req</span>
                  {entry.error_count > 0 && (
                    <span className="text-red-400">{entry.error_count} err</span>
                  )}
                </div>
              </div>
            )) : (
              <div className="text-gray-500 text-sm">No data</div>
            )}
          </div>
        </Card>
      </div>

      {/* Server Health */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
        <Card title="Server Health">
          <div className="flex items-center gap-3 mb-4">
            <span
              className={`w-4 h-4 rounded-full ${
                stats?.server_health === 'healthy'
                  ? 'bg-green-500'
                  : stats?.server_health === 'degraded'
                  ? 'bg-yellow-500'
                  : 'bg-red-500'
              }`}
            ></span>
            <span className="text-lg font-medium capitalize">{stats?.server_health ?? 'unknown'}</span>
          </div>
          <Table
            columns={healthColumns}
            data={health}
            keyExtractor={(h) => h.route_id}
            emptyMessage="No routes configured"
          />
        </Card>

        <Card title="Recent Access Log">
          <div className="max-h-[400px] overflow-auto">
            <Table
              columns={logColumns}
              data={logs}
              keyExtractor={(log) => `${log.timestamp}-${log.ip}-${log.path}`}
              emptyMessage="No recent requests"
              onRowClick={(log) => setSelectedLog(log)}
            />
          </div>
        </Card>
      </div>

      <LogDetailModal log={selectedLog} onClose={() => setSelectedLog(null)} />
    </div>
  );
}
