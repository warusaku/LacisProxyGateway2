'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Modal } from '@/components/ui/Modal';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { Card } from '@/components/ui/Card';
import { routesApi, ddnsApi, serverRoutesApi, type RouteDetailedStatus, type ServerRoute } from '@/lib/api';
import { getStatusColor } from '@/lib/format';
import type { ProxyRoute, CreateRouteRequest, DdnsConfig, AccessLog } from '@/types';

type ViewMode = 'list' | 'status' | 'subnet';

export default function ServerRoutesPage() {
  const [routes, setRoutes] = useState<ProxyRoute[]>([]);
  const [serverRoutes, setServerRoutes] = useState<ServerRoute[]>([]);
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
    const interval = setInterval(() => {
      if (viewMode === 'status') loadRouteStatus();
    }, 30000);
    return () => clearInterval(interval);
  }, [viewMode]);

  const loadData = async () => {
    try {
      setLoading(true);
      const [routesList, ddnsList, statusList, srList] = await Promise.all([
        routesApi.list(),
        ddnsApi.list(),
        routesApi.getAllStatus(),
        serverRoutesApi.list(),
      ]);
      setRoutes(routesList);
      setDdnsConfigs(ddnsList);
      setRouteStatus(statusList);
      setServerRoutes(srList);
    } catch {
      setError('Failed to load data');
    } finally {
      setLoading(false);
    }
  };

  const loadRouteStatus = async () => {
    try {
      const status = await routesApi.getAllStatus();
      setRouteStatus(status);
    } catch {
      /* ignore refresh errors */
    }
  };

  const handleSubmit = async () => {
    try {
      setError('');
      if (editingRoute) {
        await routesApi.update(editingRoute.id, formData);
      } else {
        await routesApi.create(formData);
      }
      setIsModalOpen(false);
      setEditingRoute(null);
      resetForm();
      loadData();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Operation failed');
    }
  };

  const handleDelete = async (id: number) => {
    if (!window.confirm('Delete this route?')) return;
    try {
      await routesApi.delete(id);
      loadData();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    }
  };

  const handleViewLogs = async (route: ProxyRoute) => {
    try {
      const logs = await routesApi.getLogs(route.id, 50);
      setSelectedRouteLogs(logs);
      setSelectedRouteName(route.path);
      setIsLogsModalOpen(true);
    } catch {
      setError('Failed to load logs');
    }
  };

  const openEdit = (route: ProxyRoute) => {
    setEditingRoute(route);
    setFormData({
      path: route.path,
      target: route.target,
      ddns_config_id: route.ddns_config_id ?? null,
      priority: route.priority,
      active: route.active,
      strip_prefix: route.strip_prefix,
      preserve_host: route.preserve_host,
      timeout_ms: route.timeout_ms ?? 30000,
      websocket_support: route.websocket_support,
    });
    setIsModalOpen(true);
  };

  const resetForm = () => {
    setFormData({
      path: '', target: '', ddns_config_id: null, priority: 100,
      active: true, strip_prefix: true, preserve_host: false,
      timeout_ms: 30000, websocket_support: false,
    });
  };

  const routeColumns = [
    { key: 'path' as const, header: 'Path', render: (r: ProxyRoute) => <code className="text-blue-400">{r.path}</code> },
    { key: 'target' as const, header: 'Target', render: (r: ProxyRoute) => <code className="text-xs truncate max-w-[200px] block">{r.target}</code> },
    { key: 'priority' as const, header: 'Priority' },
    { key: 'active' as const, header: 'Status', render: (r: ProxyRoute) => <Badge variant={r.active ? 'success' : 'error'}>{r.active ? 'Active' : 'Inactive'}</Badge> },
    { key: 'websocket_support' as const, header: 'WS', render: (r: ProxyRoute) => r.websocket_support ? <Badge variant="info">WS</Badge> : null },
    { key: 'id' as const, header: 'Actions', render: (r: ProxyRoute) => (
      <div className="flex gap-1">
        <Button size="sm" variant="secondary" onClick={() => openEdit(r)}>Edit</Button>
        <Button size="sm" variant="secondary" onClick={() => handleViewLogs(r)}>Logs</Button>
        <Button size="sm" variant="danger" onClick={() => handleDelete(r.id)}>Del</Button>
      </div>
    )},
  ];

  const statusColumns = [
    { key: 'path' as const, header: 'Path', render: (s: RouteDetailedStatus) => <code className="text-blue-400">{s.path}</code> },
    { key: 'healthy' as const, header: 'Health', render: (s: RouteDetailedStatus) => <Badge variant={s.healthy ? 'success' : 'error'}>{s.healthy ? 'Healthy' : 'Down'}</Badge> },
    { key: 'requests_today' as const, header: 'Today', render: (s: RouteDetailedStatus) => s.requests_today.toLocaleString() },
    { key: 'requests_last_hour' as const, header: 'Last Hour', render: (s: RouteDetailedStatus) => s.requests_last_hour.toLocaleString() },
    { key: 'error_rate_percent' as const, header: 'Error%', render: (s: RouteDetailedStatus) => <span className={s.error_rate_percent > 5 ? 'text-red-400' : ''}>{s.error_rate_percent.toFixed(1)}%</span> },
    { key: 'avg_response_time_ms' as const, header: 'Avg ms', render: (s: RouteDetailedStatus) => `${s.avg_response_time_ms.toFixed(0)}ms` },
  ];

  const subnetColumns = [
    { key: 'path' as const, header: 'Path', render: (r: ServerRoute) => <code className="text-blue-400">{r.path}</code> },
    { key: 'target' as const, header: 'Target', render: (r: ServerRoute) => <code className="text-xs truncate max-w-[180px] block">{r.target}</code> },
    { key: 'active' as const, header: 'Status', render: (r: ServerRoute) => <Badge variant={r.active ? 'success' : 'error'}>{r.active ? 'Active' : 'Off'}</Badge> },
    { key: 'subnet' as const, header: 'Subnet', render: (r: ServerRoute) => r.subnet ? <code className="text-xs text-green-400">{r.subnet.network}</code> : <span className="text-gray-500">-</span> },
    { key: 'fid' as const, header: 'FID', render: (r: ServerRoute) => r.fid ? <code className="text-xs">{r.fid}</code> : <span className="text-gray-500">-</span> },
    { key: 'tid' as const, header: 'TID', render: (r: ServerRoute) => r.tid ? <code className="text-xs">{r.tid}</code> : <span className="text-gray-500">-</span> },
  ];

  const logColumns = [
    { key: 'timestamp' as const, header: 'Time', render: (l: AccessLog) => new Date(l.timestamp).toLocaleString() },
    { key: 'ip' as const, header: 'IP', render: (l: AccessLog) => <code className="text-xs">{l.ip}</code> },
    { key: 'method' as const, header: 'Method' },
    { key: 'path' as const, header: 'Path', render: (l: AccessLog) => <code className="text-xs truncate max-w-[200px] block">{l.path}</code> },
    { key: 'status' as const, header: 'Status', render: (l: AccessLog) => <span className={getStatusColor(l.status)}>{l.status}</span> },
    { key: 'response_time_ms' as const, header: 'Time(ms)' },
  ];

  if (loading) {
    return <div className="flex items-center justify-center h-64"><div className="text-gray-400">Loading...</div></div>;
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold">ServerRoutes</h2>
        <div className="flex gap-2">
          <div className="flex bg-gray-800 rounded overflow-hidden">
            {(['list', 'status', 'subnet'] as ViewMode[]).map(mode => (
              <button
                key={mode}
                onClick={() => setViewMode(mode)}
                className={`px-3 py-1.5 text-sm capitalize ${viewMode === mode ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'}`}
              >
                {mode}
              </button>
            ))}
          </div>
          <Button onClick={() => { resetForm(); setEditingRoute(null); setIsModalOpen(true); }}>
            Add Route
          </Button>
        </div>
      </div>

      {error && <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded text-red-400 text-sm">{error}</div>}

      {viewMode === 'list' && (
        <Card>
          <Table columns={routeColumns} data={routes} keyExtractor={(r) => r.id} emptyMessage="No routes configured" />
        </Card>
      )}

      {viewMode === 'status' && (
        <Card>
          <Table columns={statusColumns} data={routeStatus} keyExtractor={(s) => s.route_id} emptyMessage="No status data" />
        </Card>
      )}

      {viewMode === 'subnet' && (
        <Card>
          <Table columns={subnetColumns} data={serverRoutes} keyExtractor={(r) => r.id} emptyMessage="No routes" />
        </Card>
      )}

      {/* Create/Edit Modal */}
      <Modal isOpen={isModalOpen} onClose={() => { setIsModalOpen(false); setEditingRoute(null); }} title={editingRoute ? 'Edit Route' : 'Add Route'}>
        <div className="space-y-4">
          <Input label="Path" value={formData.path} onChange={(e) => setFormData(prev => ({ ...prev, path: e.target.value }))} placeholder="/service" />
          <Input label="Target URL" value={formData.target} onChange={(e) => setFormData(prev => ({ ...prev, target: e.target.value }))} placeholder="http://192.168.1.100:8080" />
          <Select label="DDNS Config" value={formData.ddns_config_id?.toString() ?? ''} onChange={(e) => setFormData(prev => ({ ...prev, ddns_config_id: e.target.value ? parseInt(e.target.value) : null }))}
            options={[{ value: '', label: 'None' }, ...ddnsConfigs.map(d => ({ value: d.id.toString(), label: d.hostname }))]} />
          <Input label="Priority" type="number" value={formData.priority} onChange={(e) => setFormData(prev => ({ ...prev, priority: parseInt(e.target.value) || 100 }))} />
          <Input label="Timeout (ms)" type="number" value={formData.timeout_ms} onChange={(e) => setFormData(prev => ({ ...prev, timeout_ms: parseInt(e.target.value) || 30000 }))} />
          <div className="flex gap-4">
            <label className="flex items-center gap-2 cursor-pointer">
              <input type="checkbox" checked={formData.active} onChange={(e) => setFormData(prev => ({ ...prev, active: e.target.checked }))} className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-500" />
              <span className="text-sm">Active</span>
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input type="checkbox" checked={formData.strip_prefix} onChange={(e) => setFormData(prev => ({ ...prev, strip_prefix: e.target.checked }))} className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-500" />
              <span className="text-sm">Strip Prefix</span>
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input type="checkbox" checked={formData.websocket_support} onChange={(e) => setFormData(prev => ({ ...prev, websocket_support: e.target.checked }))} className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-500" />
              <span className="text-sm">WebSocket</span>
            </label>
          </div>
          <div className="flex justify-end gap-2 pt-4">
            <Button variant="secondary" onClick={() => { setIsModalOpen(false); setEditingRoute(null); }}>Cancel</Button>
            <Button onClick={handleSubmit}>{editingRoute ? 'Update' : 'Create'}</Button>
          </div>
        </div>
      </Modal>

      {/* Logs Modal */}
      <Modal isOpen={isLogsModalOpen} onClose={() => setIsLogsModalOpen(false)} title={`Logs: ${selectedRouteName}`}>
        <div className="max-h-[500px] overflow-auto">
          <Table columns={logColumns} data={selectedRouteLogs} keyExtractor={(l) => `${l.timestamp}-${l.ip}`} emptyMessage="No recent logs" />
        </div>
      </Modal>
    </div>
  );
}
