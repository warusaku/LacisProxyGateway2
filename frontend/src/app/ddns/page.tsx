'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Modal } from '@/components/ui/Modal';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { ddnsApi } from '@/lib/api';
import type { DdnsConfig, CreateDdnsRequest, DdnsProvider, DdnsStatus } from '@/types';

export default function DdnsPage() {
  const [configs, setConfigs] = useState<DdnsConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingConfig, setEditingConfig] = useState<DdnsConfig | null>(null);
  const [formData, setFormData] = useState<CreateDdnsRequest>({
    provider: 'dyndns',
    hostname: '',
    username: '',
    password: '',
    api_token: '',
    zone_id: '',
    update_interval_sec: 300,
  });
  const [error, setError] = useState('');

  useEffect(() => {
    loadConfigs();
  }, []);

  const loadConfigs = async () => {
    try {
      const data = await ddnsApi.list();
      setConfigs(data);
    } catch (err) {
      console.error('Failed to load DDNS configs:', err);
    } finally {
      setLoading(false);
    }
  };

  const openCreateModal = () => {
    setEditingConfig(null);
    setFormData({
      provider: 'dyndns',
      hostname: '',
      username: '',
      password: '',
      api_token: '',
      zone_id: '',
      update_interval_sec: 300,
    });
    setError('');
    setIsModalOpen(true);
  };

  const openEditModal = (config: DdnsConfig) => {
    setEditingConfig(config);
    setFormData({
      provider: config.provider,
      hostname: config.hostname,
      username: config.username || '',
      password: '',
      api_token: '',
      zone_id: config.zone_id || '',
      update_interval_sec: config.update_interval_sec,
    });
    setError('');
    setIsModalOpen(true);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    try {
      if (editingConfig) {
        await ddnsApi.update(editingConfig.id, {
          hostname: formData.hostname,
          username: formData.username || undefined,
          password: formData.password || undefined,
          api_token: formData.api_token || undefined,
          zone_id: formData.zone_id || undefined,
          update_interval_sec: formData.update_interval_sec,
        });
      } else {
        await ddnsApi.create(formData);
      }
      setIsModalOpen(false);
      loadConfigs();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save DDNS config');
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('Are you sure you want to delete this DDNS configuration?')) return;

    try {
      await ddnsApi.delete(id);
      loadConfigs();
    } catch (err) {
      console.error('Failed to delete DDNS config:', err);
    }
  };

  const handleTriggerUpdate = async (id: number) => {
    try {
      await ddnsApi.triggerUpdate(id);
      loadConfigs();
    } catch (err) {
      console.error('Failed to trigger DDNS update:', err);
    }
  };

  const getStatusBadge = (status: DdnsStatus) => {
    switch (status) {
      case 'active':
        return <Badge variant="success">Active</Badge>;
      case 'error':
        return <Badge variant="error">Error</Badge>;
      case 'disabled':
        return <Badge variant="default">Disabled</Badge>;
    }
  };

  const columns = [
    {
      key: 'provider',
      header: 'Provider',
      render: (config: DdnsConfig) => (
        <span className="capitalize">{config.provider}</span>
      ),
    },
    {
      key: 'hostname',
      header: 'Hostname',
      render: (config: DdnsConfig) => (
        <code className="text-blue-400">{config.hostname}</code>
      ),
    },
    {
      key: 'last_ip',
      header: 'Last IP',
      render: (config: DdnsConfig) => (
        <span className="text-sm text-gray-400">{config.last_ip || '-'}</span>
      ),
    },
    {
      key: 'last_update',
      header: 'Last Update',
      render: (config: DdnsConfig) => (
        <span className="text-sm text-gray-400">
          {config.last_update ? new Date(config.last_update).toLocaleString() : '-'}
        </span>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      render: (config: DdnsConfig) => getStatusBadge(config.status),
    },
    {
      key: 'actions',
      header: 'Actions',
      render: (config: DdnsConfig) => (
        <div className="flex gap-2">
          <Button size="sm" variant="ghost" onClick={() => handleTriggerUpdate(config.id)}>
            Update Now
          </Button>
          <Button size="sm" variant="ghost" onClick={() => openEditModal(config)}>
            Edit
          </Button>
          <Button size="sm" variant="danger" onClick={() => handleDelete(config.id)}>
            Delete
          </Button>
        </div>
      ),
    },
  ];

  const providerOptions = [
    { value: 'dyndns', label: 'DynDNS' },
    { value: 'noip', label: 'No-IP' },
    { value: 'cloudflare', label: 'Cloudflare' },
  ];

  const isCloudflare = formData.provider === 'cloudflare';

  if (loading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>;
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">DDNS Configuration</h1>
        <Button onClick={openCreateModal}>Add DDNS</Button>
      </div>

      <div className="bg-card border border-border rounded-lg">
        <Table columns={columns} data={configs} keyExtractor={(c) => c.id} emptyMessage="No DDNS configurations" />
      </div>

      <Modal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        title={editingConfig ? 'Edit DDNS' : 'Add DDNS'}
      >
        <form onSubmit={handleSubmit} className="space-y-4">
          {!editingConfig && (
            <Select
              label="Provider"
              options={providerOptions}
              value={formData.provider}
              onChange={(e) => setFormData({ ...formData, provider: e.target.value as DdnsProvider })}
            />
          )}
          <Input
            label="Hostname"
            placeholder="example.dyndns.org"
            value={formData.hostname}
            onChange={(e) => setFormData({ ...formData, hostname: e.target.value })}
            required
          />
          {isCloudflare ? (
            <>
              <Input
                label="API Token"
                type="password"
                placeholder={editingConfig ? '(unchanged)' : ''}
                value={formData.api_token}
                onChange={(e) => setFormData({ ...formData, api_token: e.target.value })}
                required={!editingConfig}
              />
              <Input
                label="Zone ID"
                value={formData.zone_id}
                onChange={(e) => setFormData({ ...formData, zone_id: e.target.value })}
                required={!editingConfig}
              />
            </>
          ) : (
            <>
              <Input
                label="Username"
                value={formData.username}
                onChange={(e) => setFormData({ ...formData, username: e.target.value })}
                required={!editingConfig}
              />
              <Input
                label="Password"
                type="password"
                placeholder={editingConfig ? '(unchanged)' : ''}
                value={formData.password}
                onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                required={!editingConfig}
              />
            </>
          )}
          <Input
            label="Update Interval (seconds)"
            type="number"
            value={formData.update_interval_sec}
            onChange={(e) => setFormData({ ...formData, update_interval_sec: parseInt(e.target.value) || 300 })}
          />
          {error && <p className="text-red-500 text-sm">{error}</p>}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={() => setIsModalOpen(false)}>
              Cancel
            </Button>
            <Button type="submit">
              {editingConfig ? 'Update' : 'Create'}
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}
