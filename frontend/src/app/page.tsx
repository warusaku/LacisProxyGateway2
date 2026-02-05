'use client';

import { useEffect, useState } from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Table } from '@/components/ui/Table';
import { dashboardApi } from '@/lib/api';
import type { DashboardStats, RouteHealth, AccessLog } from '@/types';

export default function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [health, setHealth] = useState<RouteHealth[]>([]);
  const [logs, setLogs] = useState<AccessLog[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadDashboard();
    // Auto-refresh every 30 seconds
    const interval = setInterval(loadDashboard, 30000);
    return () => clearInterval(interval);
  }, []);

  const loadDashboard = async () => {
    try {
      const [statsData, healthData, logsData] = await Promise.all([
        dashboardApi.getStats(),
        dashboardApi.getHealth(),
        dashboardApi.getAccessLog(20),
      ]);
      setStats(statsData);
      setHealth(healthData);
      setLogs(logsData);
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

  return (
    <div>
      <h1 className="text-2xl font-bold mb-8">Dashboard</h1>

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
