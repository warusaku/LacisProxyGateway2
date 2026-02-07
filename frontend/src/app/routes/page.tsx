'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Modal } from '@/components/ui/Modal';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { Card } from '@/components/ui/Card';
import { routesApi, ddnsApi, type RouteDetailedStatus } from '@/lib/api';
import { getStatusColor } from '@/lib/format';
import type { ProxyRoute, CreateRouteRequest, DdnsConfig, AccessLog } from '@/types';

type ViewMode = 'list' | 'status';

export default function RoutesPage() {
  const [routes, setRoutes] = useState<ProxyRoute[]>([]);
  const [routeStatus, setRouteStatus] = useState<RouteDetailedStatus[]>([]);
  const [ddnsConfigs, setDdnsConfigs] = useState<DdnsConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isLogsModalOpen, setIsLogsModalOpen] = useState(false);
  const [selectedRouteLogs, setSelectedRouteLogs] = useState<AccessLog[]>([]);
  const [selectedRouteName, setSelectedRouteName] = useState('');
  const [editingRoute, setEditingRoute] = useState<ProxyRoute | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [formData, setFormData] = useState<CreateRouteRequest>({
    path: '',
    target: '',
    ddns_config_id: null,
    priority: 100,
    active: true,
    strip_prefix: true,
    preserve_host: false,
    timeout_ms: 30000,
    websocket_support: false,
  });
  const [error, setError] = useState('');

  useEffect(() => {
    loadData();
    // Auto-refresh status every 30 seconds
    const interval = setInterval(() => {
      if (viewMode === 'status') {
        loadRouteStatus();
      }
    }, 30000);
    return () => clearInterval(interval);
  }, [viewMode]);

  const loadData = async () => {
    try {
      const [routesData, ddnsData, statusData] = await Promise.all([
        routesApi.list(),
        ddnsApi.list(),
        routesApi.getAllStatus().catch(() => []),
      ]);
      setRoutes(routesData);
      setDdnsConfigs(ddnsData);
      setRouteStatus(statusData);
    } catch (err) {
      console.error('Failed to load data:', err);
    } finally {
      setLoading(false);
    }
  };

  const loadRoutes = async () => {
    try {
      const data = await routesApi.list();
      setRoutes(data);
    } catch (err) {
      console.error('Failed to load routes:', err);
    }
  };

  const loadRouteStatus = async () => {
    try {
      const statusData = await routesApi.getAllStatus();
      setRouteStatus(statusData);
    } catch (err) {
      console.error('Failed to load route status:', err);
    }
  };

  const loadRouteLogs = async (routeId: number, routePath: string) => {
    try {
      const logs = await routesApi.getLogs(routeId, 50);
      setSelectedRouteLogs(logs);
      setSelectedRouteName(routePath);
      setIsLogsModalOpen(true);
    } catch (err) {
      console.error('Failed to load route logs:', err);
    }
  };

  const getDdnsHostname = (ddnsId: number | null | undefined) => {
    if (!ddnsId) return null;
    const ddns = ddnsConfigs.find((d) => d.id === ddnsId);
    return ddns?.hostname;
  };

  const openCreateModal = () => {
    setEditingRoute(null);
    setFormData({
      path: '',
      target: '',
      ddns_config_id: null,
      priority: 100,
      active: true,
      strip_prefix: true,
      preserve_host: false,
      timeout_ms: 30000,
      websocket_support: false,
    });
    setError('');
    setIsModalOpen(true);
  };

  const openEditModal = (route: ProxyRoute) => {
    setEditingRoute(route);
    setFormData({
      path: route.path,
      target: route.target,
      ddns_config_id: route.ddns_config_id,
      priority: route.priority,
      active: route.active,
      strip_prefix: route.strip_prefix,
      preserve_host: route.preserve_host,
      timeout_ms: route.timeout_ms,
      websocket_support: route.websocket_support,
    });
    setError('');
    setIsModalOpen(true);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    try {
      if (editingRoute) {
        await routesApi.update(editingRoute.id, formData);
      } else {
        await routesApi.create(formData);
      }
      setIsModalOpen(false);
      loadRoutes();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save route');
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('Are you sure you want to delete this route?')) return;

    try {
      await routesApi.delete(id);
      loadRoutes();
    } catch (err) {
      console.error('Failed to delete route:', err);
    }
  };

  const handleToggleActive = async (route: ProxyRoute) => {
    try {
      await routesApi.update(route.id, { active: !route.active });
      loadRoutes();
    } catch (err) {
      console.error('Failed to toggle route:', err);
    }
  };

  const columns = [
    {
      key: 'path',
      header: 'Path',
      render: (route: ProxyRoute) => (
        <div className="flex items-center gap-2">
          <code className="text-blue-400">{route.path}</code>
          {route.websocket_support && <Badge variant="info">WS</Badge>}
        </div>
      ),
    },
    {
      key: 'target',
      header: 'Target',
      render: (route: ProxyRoute) => (
        <span className="text-sm text-gray-400">{route.target}</span>
      ),
    },
    {
      key: 'ddns',
      header: 'DDNS',
      render: (route: ProxyRoute) => {
        const hostname = getDdnsHostname(route.ddns_config_id);
        return hostname ? (
          <Badge variant="info">{hostname}</Badge>
        ) : (
          <span className="text-gray-500 text-sm">All</span>
        );
      },
    },
    {
      key: 'priority',
      header: 'Priority',
    },
    {
      key: 'active',
      header: 'Status',
      render: (route: ProxyRoute) => (
        <Badge variant={route.active ? 'success' : 'default'}>
          {route.active ? 'Active' : 'Inactive'}
        </Badge>
      ),
    },
    {
      key: 'actions',
      header: 'Actions',
      render: (route: ProxyRoute) => (
        <div className="flex gap-2">
          <Button size="sm" variant="ghost" onClick={() => handleToggleActive(route)}>
            {route.active ? 'Disable' : 'Enable'}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => openEditModal(route)}>
            Edit
          </Button>
          <Button size="sm" variant="danger" onClick={() => handleDelete(route.id)}>
            Delete
          </Button>
        </div>
      ),
    },
  ];

  const logColumns = [
    {
      key: 'timestamp',
      header: 'Time',
      render: (log: AccessLog) => (
        <span className="text-sm text-gray-400">
          {new Date(log.timestamp).toLocaleString()}
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
    return <div className="flex items-center justify-center h-64">Loading...</div>;
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Proxy Routes</h1>
        <div className="flex gap-2">
          <div className="flex rounded-lg border border-border overflow-hidden">
            <button
              className={`px-4 py-2 text-sm ${viewMode === 'list' ? 'bg-primary text-primary-foreground' : 'bg-card hover:bg-muted'}`}
              onClick={() => setViewMode('list')}
            >
              List
            </button>
            <button
              className={`px-4 py-2 text-sm ${viewMode === 'status' ? 'bg-primary text-primary-foreground' : 'bg-card hover:bg-muted'}`}
              onClick={() => { setViewMode('status'); loadRouteStatus(); }}
            >
              Status
            </button>
          </div>
          <Button onClick={openCreateModal}>Add Route</Button>
        </div>
      </div>

      {viewMode === 'list' ? (
        <div className="bg-card border border-border rounded-lg">
          <Table columns={columns} data={routes} keyExtractor={(r) => r.id} emptyMessage="No routes configured" />
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {routeStatus.map((status) => (
            <Card key={status.route_id} className="relative">
              <div className="flex justify-between items-start mb-4">
                <div>
                  <code className="text-lg text-blue-400">{status.path}</code>
                  <div className="text-sm text-gray-400 mt-1">{status.target}</div>
                </div>
                <div className="flex gap-2">
                  <Badge variant={status.active ? 'success' : 'default'}>
                    {status.active ? 'Active' : 'Inactive'}
                  </Badge>
                  <Badge variant={status.healthy ? 'success' : 'error'}>
                    {status.healthy ? 'Healthy' : 'Unhealthy'}
                  </Badge>
                </div>
              </div>

              <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                <div>
                  <div className="text-gray-400">Today</div>
                  <div className="text-xl font-bold">{status.requests_today.toLocaleString()}</div>
                </div>
                <div>
                  <div className="text-gray-400">Last Hour</div>
                  <div className="text-xl font-bold">{status.requests_last_hour.toLocaleString()}</div>
                </div>
                <div>
                  <div className="text-gray-400">Error Rate</div>
                  <div className={`text-xl font-bold ${status.error_rate_percent > 10 ? 'text-red-400' : status.error_rate_percent > 5 ? 'text-yellow-400' : 'text-green-400'}`}>
                    {status.error_rate_percent.toFixed(1)}%
                  </div>
                </div>
                <div>
                  <div className="text-gray-400">Avg Response</div>
                  <div className={`text-xl font-bold ${status.avg_response_time_ms > 1000 ? 'text-red-400' : status.avg_response_time_ms > 500 ? 'text-yellow-400' : 'text-green-400'}`}>
                    {status.avg_response_time_ms.toFixed(0)}ms
                  </div>
                </div>
              </div>

              <div className="mt-4 pt-4 border-t border-border flex justify-between items-center text-sm">
                <div className="text-gray-400">
                  {status.consecutive_failures > 0 && (
                    <span className="text-red-400">
                      {status.consecutive_failures} consecutive failures
                    </span>
                  )}
                  {status.last_check && (
                    <span className="ml-2">
                      Last check: {new Date(status.last_check).toLocaleTimeString()}
                    </span>
                  )}
                  {status.last_status_code && (
                    <span className={`ml-2 font-mono ${getStatusColor(status.last_status_code)}`}>
                      ({status.last_status_code})
                    </span>
                  )}
                </div>
                <Button size="sm" variant="ghost" onClick={() => loadRouteLogs(status.route_id, status.path)}>
                  View Logs
                </Button>
              </div>
            </Card>
          ))}
        </div>
      )}

      <Modal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        title={editingRoute ? 'Edit Route' : 'Add Route'}
      >
        <form onSubmit={handleSubmit} className="space-y-4">
          <Input
            label="Path"
            placeholder="/example"
            value={formData.path}
            onChange={(e) => setFormData({ ...formData, path: e.target.value })}
            required
          />
          <Input
            label="Target URL"
            placeholder="http://localhost:3000"
            value={formData.target}
            onChange={(e) => setFormData({ ...formData, target: e.target.value })}
            required
          />
          <Select
            label="DDNS Hostname (Optional)"
            value={formData.ddns_config_id?.toString() || ''}
            onChange={(e) => setFormData({
              ...formData,
              ddns_config_id: e.target.value ? parseInt(e.target.value) : null
            })}
            options={[
              { value: '', label: 'All hosts (no restriction)' },
              ...ddnsConfigs.map((d) => ({ value: d.id.toString(), label: d.hostname })),
            ]}
          />
          <Input
            label="Priority"
            type="number"
            value={formData.priority}
            onChange={(e) => setFormData({ ...formData, priority: parseInt(e.target.value) || 100 })}
          />
          <Input
            label="Timeout (ms)"
            type="number"
            value={formData.timeout_ms}
            onChange={(e) => setFormData({ ...formData, timeout_ms: parseInt(e.target.value) || 30000 })}
          />
          <div className="flex gap-4">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={formData.active}
                onChange={(e) => setFormData({ ...formData, active: e.target.checked })}
                className="rounded"
              />
              <span className="text-sm">Active</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={formData.strip_prefix}
                onChange={(e) => setFormData({ ...formData, strip_prefix: e.target.checked })}
                className="rounded"
              />
              <span className="text-sm">Strip Prefix</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={formData.preserve_host}
                onChange={(e) => setFormData({ ...formData, preserve_host: e.target.checked })}
                className="rounded"
              />
              <span className="text-sm">Preserve Host</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={formData.websocket_support}
                onChange={(e) => setFormData({ ...formData, websocket_support: e.target.checked })}
                className="rounded"
              />
              <span className="text-sm">WebSocket</span>
            </label>
          </div>
          {error && <p className="text-red-500 text-sm">{error}</p>}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={() => setIsModalOpen(false)}>
              Cancel
            </Button>
            <Button type="submit">
              {editingRoute ? 'Update' : 'Create'}
            </Button>
          </div>
        </form>
      </Modal>

      {/* Logs Modal */}
      <Modal
        isOpen={isLogsModalOpen}
        onClose={() => setIsLogsModalOpen(false)}
        title={`Access Logs: ${selectedRouteName}`}
      >
        <div className="max-h-[500px] overflow-y-auto">
          <Table
            columns={logColumns}
            data={selectedRouteLogs}
            keyExtractor={(log) => `${log.timestamp}-${log.ip}-${log.path}`}
            emptyMessage="No logs found"
          />
        </div>
        <div className="flex justify-end mt-4">
          <Button variant="ghost" onClick={() => setIsLogsModalOpen(false)}>
            Close
          </Button>
        </div>
      </Modal>
    </div>
  );
}
