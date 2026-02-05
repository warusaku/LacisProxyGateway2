'use client';

import { useEffect, useState } from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Table } from '@/components/ui/Table';
import { dashboardApi, omadaApi, type NetworkStatus, type SslStatus } from '@/lib/api';
import type { DashboardStats, RouteHealth, AccessLog } from '@/types';

export default function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [health, setHealth] = useState<RouteHealth[]>([]);
  const [logs, setLogs] = useState<AccessLog[]>([]);
  const [network, setNetwork] = useState<NetworkStatus | null>(null);
  const [ssl, setSsl] = useState<SslStatus | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadDashboard();
    // Auto-refresh every 30 seconds
    const interval = setInterval(loadDashboard, 30000);
    return () => clearInterval(interval);
  }, []);

  const loadDashboard = async () => {
    try {
      const [statsData, healthData, logsData, networkData, sslData] = await Promise.all([
        dashboardApi.getStats(),
        dashboardApi.getHealth(),
        dashboardApi.getAccessLog(20),
        omadaApi.getStatus().catch(() => null),
        dashboardApi.getSslStatus().catch(() => null),
      ]);
      setStats(statsData);
      setHealth(healthData);
      setLogs(logsData);
      setNetwork(networkData);
      setSsl(sslData);
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

  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return 'text-green-400';
    if (status >= 300 && status < 400) return 'text-blue-400';
    if (status >= 400 && status < 500) return 'text-yellow-400';
    return 'text-red-400';
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
        <span className="text-sm text-gray-400">
          {new Date(log.timestamp).toLocaleTimeString()}
        </span>
      ),
    },
    {
      key: 'method',
      header: 'Method',
      render: (log: AccessLog) => (
        <span className="font-mono text-sm">{log.method}</span>
      ),
    },
    {
      key: 'path',
      header: 'Path',
      render: (log: AccessLog) => (
        <code className="text-sm truncate max-w-xs block">{log.path}</code>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      render: (log: AccessLog) => (
        <span className={`font-mono ${getStatusColor(log.status)}`}>
          {log.status}
        </span>
      ),
    },
    {
      key: 'response_time_ms',
      header: 'Time',
      render: (log: AccessLog) => (
        <span className="text-sm text-gray-400">{log.response_time_ms}ms</span>
      ),
    },
    {
      key: 'ip',
      header: 'IP',
      render: (log: AccessLog) => (
        <code className="text-sm">{log.ip}</code>
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
          <div className="max-h-[400px] overflow-y-auto">
            <Table
              columns={logColumns}
              data={logs}
              keyExtractor={(log) => `${log.timestamp}-${log.ip}-${log.path}`}
              emptyMessage="No recent requests"
            />
          </div>
        </Card>
      </div>
    </div>
  );
}
