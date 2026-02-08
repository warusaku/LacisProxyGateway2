'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Modal } from '@/components/ui/Modal';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { Card } from '@/components/ui/Card';
import { ddnsApi, ddnsIntegratedApi, omadaApi, type DdnsIntegrated, type OmadaControllerDoc } from '@/lib/api';
import type { DdnsConfig, CreateDdnsRequest, DdnsProvider, DdnsStatus } from '@/types';

type ViewTab = 'standard' | 'integrated';

export default function DdnsPage() {
  const [configs, setConfigs] = useState<DdnsConfig[]>([]);
  const [integrated, setIntegrated] = useState<DdnsIntegrated[]>([]);
  const [controllers, setControllers] = useState<OmadaControllerDoc[]>([]);
  const [loading, setLoading] = useState(true);
  const [viewTab, setViewTab] = useState<ViewTab>('integrated');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isLinkModalOpen, setIsLinkModalOpen] = useState(false);
  const [isPortForwardOpen, setIsPortForwardOpen] = useState(false);
  const [editingConfig, setEditingConfig] = useState<DdnsConfig | null>(null);
  const [linkingConfigId, setLinkingConfigId] = useState<number | null>(null);
  const [linkForm, setLinkForm] = useState({ omada_controller_id: '', omada_site_id: '' });
  const [portForwards, setPortForwards] = useState<DdnsIntegrated | null>(null);
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
    loadData();
  }, []);

  const loadData = async () => {
    try {
      setLoading(true);
      const [configList, integratedList, ctrlResult] = await Promise.all([
        ddnsApi.list(),
        ddnsIntegratedApi.list().catch(() => [] as DdnsIntegrated[]),
        omadaApi.listControllers().catch(() => ({ ok: false, controllers: [] as OmadaControllerDoc[] })),
      ]);
      setConfigs(configList);
      setIntegrated(integratedList);
      setControllers(ctrlResult.controllers || []);
    } catch (err) {
      console.error('Failed to load data:', err);
    } finally {
      setLoading(false);
    }
  };

  const openCreateModal = () => {
    setEditingConfig(null);
    setFormData({
      provider: 'dyndns', hostname: '', username: '', password: '',
      api_token: '', zone_id: '', update_interval_sec: 300,
    });
    setError('');
    setIsModalOpen(true);
  };

  const openEditModal = (config: DdnsConfig) => {
    setEditingConfig(config);
    setFormData({
      provider: config.provider, hostname: config.hostname,
      username: config.username || '', password: '',
      api_token: '', zone_id: config.zone_id || '',
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
      loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save DDNS config');
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('Are you sure you want to delete this DDNS configuration?')) return;
    try {
      await ddnsApi.delete(id);
      loadData();
    } catch (err) {
      console.error('Failed to delete DDNS config:', err);
    }
  };

  const handleTriggerUpdate = async (id: number) => {
    try {
      await ddnsApi.triggerUpdate(id);
      loadData();
    } catch (err) {
      console.error('Failed to trigger DDNS update:', err);
    }
  };

  const openLinkModal = (configId: number, current?: DdnsIntegrated) => {
    setLinkingConfigId(configId);
    setLinkForm({
      omada_controller_id: current?.config.omada_controller_id || '',
      omada_site_id: current?.config.omada_site_id || '',
    });
    setIsLinkModalOpen(true);
  };

  const handleLinkSubmit = async () => {
    if (linkingConfigId === null) return;
    try {
      await ddnsIntegratedApi.linkOmada(linkingConfigId, {
        omada_controller_id: linkForm.omada_controller_id || '',
        omada_site_id: linkForm.omada_site_id || '',
      });
      setIsLinkModalOpen(false);
      loadData();
    } catch (err) {
      console.error('Failed to link Omada:', err);
    }
  };

  const getStatusBadge = (status: DdnsStatus) => {
    switch (status) {
      case 'active': return <Badge variant="success">Active</Badge>;
      case 'error': return <Badge variant="error">Error</Badge>;
      case 'disabled': return <Badge variant="default">Disabled</Badge>;
    }
  };

  // Get available sites for selected controller
  const selectedController = controllers.find(c => c.controller_id === linkForm.omada_controller_id);
  const siteOptions = selectedController?.sites?.map(s => ({
    value: s.site_id, label: `${s.name}${s.fid ? ` (FID:${s.fid})` : ''}`,
  })) || [];

  // Standard columns
  const standardColumns = [
    { key: 'provider', header: 'Provider', render: (c: DdnsConfig) => <span className="capitalize">{c.provider}</span> },
    { key: 'hostname', header: 'Hostname', render: (c: DdnsConfig) => <code className="text-blue-400">{c.hostname}</code> },
    { key: 'last_ip', header: 'Last IP', render: (c: DdnsConfig) => <span className="text-sm text-gray-400">{c.last_ip || '-'}</span> },
    { key: 'last_update', header: 'Last Update', render: (c: DdnsConfig) => <span className="text-sm text-gray-400">{c.last_update ? new Date(c.last_update).toLocaleString() : '-'}</span> },
    { key: 'status', header: 'Status', render: (c: DdnsConfig) => getStatusBadge(c.status) },
    { key: 'actions', header: 'Actions', render: (c: DdnsConfig) => (
      <div className="flex gap-2">
        <Button size="sm" variant="ghost" onClick={() => handleTriggerUpdate(c.id)}>Update Now</Button>
        <Button size="sm" variant="ghost" onClick={() => openEditModal(c)}>Edit</Button>
        <Button size="sm" variant="danger" onClick={() => handleDelete(c.id)}>Delete</Button>
      </div>
    )},
  ];

  // Integrated columns
  const integratedColumns = [
    { key: 'hostname', header: 'Hostname', render: (d: DdnsIntegrated) => <code className="text-blue-400">{d.config.hostname}</code> },
    { key: 'last_ip', header: 'DDNS IP', render: (d: DdnsIntegrated) => <span className="text-sm">{d.config.last_ip || '-'}</span> },
    { key: 'omada_wan_ip', header: 'Omada WAN IP', render: (d: DdnsIntegrated) => (
      <span className="text-sm">{d.omada_wan_ip || <span className="text-gray-500">-</span>}</span>
    )},
    { key: 'resolved_ip', header: 'DNS Resolved', render: (d: DdnsIntegrated) => (
      <span className="text-sm">{d.resolved_ip || <span className="text-gray-500">-</span>}</span>
    )},
    { key: 'ip_mismatch', header: 'Match', render: (d: DdnsIntegrated) => (
      d.omada_wan_ip
        ? d.ip_mismatch
          ? <Badge variant="error">Mismatch</Badge>
          : <Badge variant="success">OK</Badge>
        : <span className="text-gray-500">-</span>
    )},
    { key: 'linked', header: 'Linked', render: (d: DdnsIntegrated) => (
      d.linked_controller
        ? <Badge variant="info">{d.linked_controller}</Badge>
        : <span className="text-gray-500">Unlinked</span>
    )},
    { key: 'status', header: 'Status', render: (d: DdnsIntegrated) => getStatusBadge(d.config.status) },
    { key: 'actions', header: 'Actions', render: (d: DdnsIntegrated) => (
      <div className="flex gap-1">
        <Button size="sm" variant="ghost" onClick={() => openLinkModal(d.config.id, d)}>
          {d.linked_controller ? 'Relink' : 'Link'}
        </Button>
        {d.port_forwarding.length > 0 && (
          <Button size="sm" variant="ghost" onClick={() => setPortForwards(d)}>
            PF({d.port_forwarding.length})
          </Button>
        )}
        <Button size="sm" variant="ghost" onClick={() => handleTriggerUpdate(d.config.id)}>Update</Button>
        <Button size="sm" variant="ghost" onClick={() => openEditModal(d.config)}>Edit</Button>
        <Button size="sm" variant="danger" onClick={() => handleDelete(d.config.id)}>Del</Button>
      </div>
    )},
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
        <div className="flex gap-2">
          <div className="flex bg-gray-800 rounded overflow-hidden">
            {(['integrated', 'standard'] as ViewTab[]).map(tab => (
              <button
                key={tab}
                onClick={() => setViewTab(tab)}
                className={`px-3 py-1.5 text-sm capitalize ${viewTab === tab ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'}`}
              >
                {tab}
              </button>
            ))}
          </div>
          <Button onClick={openCreateModal}>Add DDNS</Button>
        </div>
      </div>

      {/* IP Mismatch Warnings */}
      {viewTab === 'integrated' && integrated.some(d => d.ip_mismatch) && (
        <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded text-red-400 text-sm">
          IP mismatch detected: DDNS resolved IP differs from Omada WAN IP. The DNS may not yet have propagated.
        </div>
      )}

      {viewTab === 'standard' && (
        <Card>
          <Table columns={standardColumns} data={configs} keyExtractor={(c) => c.id} emptyMessage="No DDNS configurations" />
        </Card>
      )}

      {viewTab === 'integrated' && (
        <Card>
          <Table columns={integratedColumns} data={integrated} keyExtractor={(d) => d.config.id} emptyMessage="No DDNS configurations" />
        </Card>
      )}

      {/* Create/Edit Modal */}
      <Modal isOpen={isModalOpen} onClose={() => setIsModalOpen(false)} title={editingConfig ? 'Edit DDNS' : 'Add DDNS'}>
        <form onSubmit={handleSubmit} className="space-y-4">
          {!editingConfig && (
            <Select
              label="Provider"
              options={providerOptions}
              value={formData.provider}
              onChange={(e) => setFormData({ ...formData, provider: e.target.value as DdnsProvider })}
            />
          )}
          <Input label="Hostname" placeholder="example.dyndns.org" value={formData.hostname}
            onChange={(e) => setFormData({ ...formData, hostname: e.target.value })} required />
          {isCloudflare ? (
            <>
              <Input label="API Token" type="password" placeholder={editingConfig ? '(unchanged)' : ''}
                value={formData.api_token} onChange={(e) => setFormData({ ...formData, api_token: e.target.value })}
                required={!editingConfig} />
              <Input label="Zone ID" value={formData.zone_id}
                onChange={(e) => setFormData({ ...formData, zone_id: e.target.value })} required={!editingConfig} />
            </>
          ) : (
            <>
              <Input label="Username" value={formData.username}
                onChange={(e) => setFormData({ ...formData, username: e.target.value })} required={!editingConfig} />
              <Input label="Password" type="password" placeholder={editingConfig ? '(unchanged)' : ''}
                value={formData.password} onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                required={!editingConfig} />
            </>
          )}
          <Input label="Update Interval (seconds)" type="number" value={formData.update_interval_sec}
            onChange={(e) => setFormData({ ...formData, update_interval_sec: parseInt(e.target.value) || 300 })} />
          {error && <p className="text-red-500 text-sm">{error}</p>}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={() => setIsModalOpen(false)}>Cancel</Button>
            <Button type="submit">{editingConfig ? 'Update' : 'Create'}</Button>
          </div>
        </form>
      </Modal>

      {/* Link to Omada Modal */}
      <Modal isOpen={isLinkModalOpen} onClose={() => setIsLinkModalOpen(false)} title="Link to Omada Controller">
        <div className="space-y-4">
          <Select
            label="Controller"
            value={linkForm.omada_controller_id}
            onChange={(e) => setLinkForm({ omada_controller_id: e.target.value, omada_site_id: '' })}
            options={[
              { value: '', label: 'None (Unlink)' },
              ...controllers.map(c => ({ value: c.controller_id, label: c.display_name })),
            ]}
          />
          {linkForm.omada_controller_id && (
            <Select
              label="Site"
              value={linkForm.omada_site_id}
              onChange={(e) => setLinkForm({ ...linkForm, omada_site_id: e.target.value })}
              options={[
                { value: '', label: 'Default' },
                ...siteOptions,
              ]}
            />
          )}
          <div className="flex justify-end gap-2 pt-4">
            <Button variant="secondary" onClick={() => setIsLinkModalOpen(false)}>Cancel</Button>
            <Button onClick={handleLinkSubmit}>Save Link</Button>
          </div>
        </div>
      </Modal>

      {/* Port Forwarding Modal */}
      <Modal
        isOpen={isPortForwardOpen || portForwards !== null}
        onClose={() => { setIsPortForwardOpen(false); setPortForwards(null); }}
        title={`Port Forwarding: ${portForwards?.config.hostname || ''}`}
      >
        <div className="max-h-[400px] overflow-auto">
          {portForwards && portForwards.port_forwarding.length > 0 ? (
            <table className="min-w-full text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left px-3 py-2 text-gray-400">Name</th>
                  <th className="text-left px-3 py-2 text-gray-400">External</th>
                  <th className="text-left px-3 py-2 text-gray-400">Internal</th>
                  <th className="text-left px-3 py-2 text-gray-400">Protocol</th>
                </tr>
              </thead>
              <tbody>
                {(portForwards.port_forwarding as Record<string, unknown>[]).map((pf, idx) => (
                  <tr key={idx} className="border-b border-border">
                    <td className="px-3 py-2">{String(pf.name || '-')}</td>
                    <td className="px-3 py-2"><code className="text-xs">{String(pf.external_port || '-')}</code></td>
                    <td className="px-3 py-2"><code className="text-xs">{String(pf.internal_ip || '')}:{String(pf.internal_port || '-')}</code></td>
                    <td className="px-3 py-2">{String(pf.protocol || '-')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <p className="text-gray-500 text-center py-4">No port forwarding rules</p>
          )}
        </div>
      </Modal>
    </div>
  );
}
