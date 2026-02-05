'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Modal } from '@/components/ui/Modal';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { routesApi } from '@/lib/api';
import type { ProxyRoute, CreateRouteRequest } from '@/types';

export default function RoutesPage() {
  const [routes, setRoutes] = useState<ProxyRoute[]>([]);
  const [loading, setLoading] = useState(true);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingRoute, setEditingRoute] = useState<ProxyRoute | null>(null);
  const [formData, setFormData] = useState<CreateRouteRequest>({
    path: '',
    target: '',
    priority: 100,
    active: true,
    strip_prefix: true,
    preserve_host: false,
    timeout_ms: 30000,
  });
  const [error, setError] = useState('');

  useEffect(() => {
    loadRoutes();
  }, []);

  const loadRoutes = async () => {
    try {
      const data = await routesApi.list();
      setRoutes(data);
    } catch (err) {
      console.error('Failed to load routes:', err);
    } finally {
      setLoading(false);
    }
  };

  const openCreateModal = () => {
    setEditingRoute(null);
    setFormData({
      path: '',
      target: '',
      priority: 100,
      active: true,
      strip_prefix: true,
      preserve_host: false,
      timeout_ms: 30000,
    });
    setError('');
    setIsModalOpen(true);
  };

  const openEditModal = (route: ProxyRoute) => {
    setEditingRoute(route);
    setFormData({
      path: route.path,
      target: route.target,
      priority: route.priority,
      active: route.active,
      strip_prefix: route.strip_prefix,
      preserve_host: route.preserve_host,
      timeout_ms: route.timeout_ms,
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
        <code className="text-blue-400">{route.path}</code>
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

  if (loading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>;
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Proxy Routes</h1>
        <Button onClick={openCreateModal}>Add Route</Button>
      </div>

      <div className="bg-card border border-border rounded-lg">
        <Table columns={columns} data={routes} keyExtractor={(r) => r.id} emptyMessage="No routes configured" />
      </div>

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
    </div>
  );
}
