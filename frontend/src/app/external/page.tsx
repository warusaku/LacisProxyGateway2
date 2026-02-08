'use client';

import { useCallback, useEffect, useState } from 'react';
import { Badge } from '@/components/ui/Badge';
import { Card } from '@/components/ui/Card';
import { externalApi } from '@/lib/api';
import type { ExternalDeviceDoc, ExternalClientDoc, ExternalSummary } from '@/types';

function relativeTime(iso?: string): string {
  if (!iso) return '-';
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

const PROTOCOL_LABELS: Record<string, string> = {
  mercury_ac: 'Mercury AC',
  deco: 'TP-Link DECO',
  generic: 'Generic',
};

export default function ExternalPage() {
  const [tab, setTab] = useState<'devices' | 'clients'>('devices');
  const [devices, setDevices] = useState<ExternalDeviceDoc[]>([]);
  const [clients, setClients] = useState<ExternalClientDoc[]>([]);
  const [summary, setSummary] = useState<ExternalSummary | null>(null);
  const [loading, setLoading] = useState(true);

  // Registration form
  const [showForm, setShowForm] = useState(false);
  const [formData, setFormData] = useState({
    display_name: '', mac: '', ip: '', protocol: 'mercury_ac', username: 'admin', password: '',
  });
  const [formLoading, setFormLoading] = useState(false);
  const [formError, setFormError] = useState('');
  const [testResult, setTestResult] = useState<string>('');

  // Client filter
  const [filterDevice, setFilterDevice] = useState('');

  const loadData = useCallback(async () => {
    try {
      const [dRes, sRes] = await Promise.all([
        externalApi.listDevices(),
        externalApi.getSummary(),
      ]);
      if (dRes.ok) setDevices(dRes.devices);
      if (sRes.ok) setSummary(sRes.summary);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  const loadClients = useCallback(async () => {
    try {
      const res = await externalApi.getClients(filterDevice || undefined);
      if (res.ok) setClients(res.clients);
    } catch { /* ignore */ }
  }, [filterDevice]);

  useEffect(() => { loadData(); }, [loadData]);
  useEffect(() => {
    if (tab === 'clients') loadClients();
  }, [tab, loadClients]);

  useEffect(() => {
    const interval = setInterval(() => {
      loadData();
      if (tab === 'clients') loadClients();
    }, 60000);
    return () => clearInterval(interval);
  }, [loadData, loadClients, tab]);

  const handleTest = async () => {
    setTestResult('Testing...');
    try {
      const res = await externalApi.testConnection({
        ip: formData.ip,
        protocol: formData.protocol,
        username: formData.username || undefined,
        password: formData.password || undefined,
      });
      if (res.success) {
        setTestResult(`Connection successful${res.model ? ` (Model: ${res.model})` : ''}`);
      } else {
        setTestResult(`Failed: ${res.error}`);
      }
    } catch (e) {
      setTestResult(`Error: ${e instanceof Error ? e.message : 'Unknown'}`);
    }
  };

  const handleRegister = async () => {
    setFormLoading(true);
    setFormError('');
    try {
      const res = await externalApi.registerDevice({
        display_name: formData.display_name,
        mac: formData.mac,
        ip: formData.ip,
        protocol: formData.protocol,
        username: formData.username || undefined,
        password: formData.password || undefined,
      });
      if (res.ok) {
        setShowForm(false);
        setFormData({ display_name: '', mac: '', ip: '', protocol: 'mercury_ac', username: 'admin', password: '' });
        setTestResult('');
        loadData();
      } else {
        setFormError(res.error || 'Registration failed');
      }
    } catch (e) {
      setFormError(e instanceof Error ? e.message : 'Error');
    } finally {
      setFormLoading(false);
    }
  };

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`Delete device "${name}"?`)) return;
    try {
      await externalApi.deleteDevice(id);
      loadData();
    } catch { /* ignore */ }
  };

  const handlePoll = async (id: string) => {
    try {
      await externalApi.pollDevice(id);
      setTimeout(loadData, 3000);
    } catch { /* ignore */ }
  };

  if (loading) return <div className="text-gray-400">Loading...</div>;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">External Devices</h1>
        <div className="flex gap-2">
          {summary && (
            <>
              <Badge variant={summary.online_devices > 0 ? 'success' : 'default'}>
                {summary.online_devices}/{summary.total_devices} Online
              </Badge>
              <Badge variant="info">{summary.active_clients} Clients</Badge>
            </>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 border-b border-border pb-2">
        {(['devices', 'clients'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`px-4 py-2 rounded-t text-sm font-medium transition-colors ${
              tab === t ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'
            }`}
          >
            {t === 'devices' ? `Devices (${devices.length})` : `Clients (${clients.length})`}
          </button>
        ))}
      </div>

      {/* Devices Tab */}
      {tab === 'devices' && (
        <div className="space-y-4">
          <button
            onClick={() => setShowForm(!showForm)}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm"
          >
            {showForm ? 'Cancel' : '+ Register Device'}
          </button>

          {showForm && (
            <Card title="Register Device">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Display Name</label>
                  <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={formData.display_name} onChange={(e) => setFormData({ ...formData, display_name: e.target.value })} />
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">MAC Address</label>
                  <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" placeholder="AA:BB:CC:DD:EE:FF" value={formData.mac} onChange={(e) => setFormData({ ...formData, mac: e.target.value })} />
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">IP Address</label>
                  <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" placeholder="192.168.1.1" value={formData.ip} onChange={(e) => setFormData({ ...formData, ip: e.target.value })} />
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Protocol</label>
                  <select className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={formData.protocol} onChange={(e) => setFormData({ ...formData, protocol: e.target.value })}>
                    <option value="mercury_ac">Mercury AC (HTTP)</option>
                    <option value="deco">TP-Link DECO (future)</option>
                    <option value="generic">Generic (manual only)</option>
                  </select>
                </div>
                {formData.protocol !== 'generic' && (
                  <>
                    <div>
                      <label className="block text-xs text-gray-400 mb-1">Username</label>
                      <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={formData.username} onChange={(e) => setFormData({ ...formData, username: e.target.value })} />
                    </div>
                    <div>
                      <label className="block text-xs text-gray-400 mb-1">Password</label>
                      <input type="password" className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={formData.password} onChange={(e) => setFormData({ ...formData, password: e.target.value })} />
                    </div>
                  </>
                )}
              </div>
              <div className="flex gap-2 mt-4">
                <button onClick={handleTest} className="px-4 py-2 bg-gray-600 text-white rounded text-sm hover:bg-gray-500" disabled={!formData.ip}>
                  Test Connection
                </button>
                <button onClick={handleRegister} disabled={formLoading || !formData.display_name || !formData.mac || !formData.ip} className="px-4 py-2 bg-blue-600 text-white rounded text-sm hover:bg-blue-700 disabled:opacity-50">
                  {formLoading ? 'Registering...' : 'Register'}
                </button>
              </div>
              {testResult && <p className={`mt-2 text-sm ${testResult.includes('successful') ? 'text-green-400' : 'text-yellow-400'}`}>{testResult}</p>}
              {formError && <p className="mt-2 text-sm text-red-400">{formError}</p>}
            </Card>
          )}

          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {devices.map((d) => (
              <Card key={d.device_id}>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <h3 className="font-medium">{d.display_name}</h3>
                    <Badge variant={d.status === 'online' ? 'success' : d.status === 'error' ? 'error' : 'default'}>
                      {d.status}
                    </Badge>
                  </div>
                  <div className="text-xs text-gray-400 space-y-1">
                    <div className="flex justify-between"><span>Protocol</span><span>{PROTOCOL_LABELS[d.protocol] || d.protocol}</span></div>
                    <div className="flex justify-between"><span>IP</span><span className="font-mono">{d.ip}</span></div>
                    <div className="flex justify-between"><span>MAC</span><span className="font-mono text-xs">{d.mac}</span></div>
                    {d.device_model && <div className="flex justify-between"><span>Model</span><span>{d.device_model}</span></div>}
                    <div className="flex justify-between"><span>Clients</span><span>{d.client_count}</span></div>
                    <div className="flex justify-between"><span>Last Poll</span><span>{relativeTime(d.last_polled_at)}</span></div>
                    {d.last_error && <div className="text-red-400 text-xs mt-1">Error: {d.last_error}</div>}
                  </div>
                  <div className="flex gap-2 pt-2 border-t border-border">
                    {d.protocol !== 'generic' && (
                      <button onClick={() => handlePoll(d.device_id)} className="px-3 py-1 bg-gray-700 text-xs text-white rounded hover:bg-gray-600">Poll</button>
                    )}
                    <button onClick={() => handleDelete(d.device_id, d.display_name)} className="px-3 py-1 bg-red-700 text-xs text-white rounded hover:bg-red-600">Delete</button>
                  </div>
                </div>
              </Card>
            ))}
          </div>
          {devices.length === 0 && <p className="text-gray-500 text-center py-8">No devices registered</p>}
        </div>
      )}

      {/* Clients Tab */}
      {tab === 'clients' && (
        <div className="space-y-4">
          <div className="flex gap-2 items-center">
            <select className="px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={filterDevice} onChange={(e) => setFilterDevice(e.target.value)}>
              <option value="">All Devices</option>
              {devices.map((d) => (
                <option key={d.device_id} value={d.device_id}>{d.display_name}</option>
              ))}
            </select>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-gray-400 border-b border-border">
                  <th className="p-2">MAC</th>
                  <th className="p-2">IP</th>
                  <th className="p-2">Hostname</th>
                  <th className="p-2">Device</th>
                  <th className="p-2">Active</th>
                  <th className="p-2">Last Seen</th>
                </tr>
              </thead>
              <tbody>
                {clients.map((c) => (
                  <tr key={`${c.mac}-${c.device_id}`} className="border-b border-border/50 hover:bg-gray-800/50">
                    <td className="p-2 font-mono text-xs">{c.mac}</td>
                    <td className="p-2 font-mono">{c.ip || '-'}</td>
                    <td className="p-2">{c.hostname || '-'}</td>
                    <td className="p-2 text-xs">{devices.find((d) => d.device_id === c.device_id)?.display_name || c.device_id}</td>
                    <td className="p-2"><Badge variant={c.active ? 'success' : 'default'}>{c.active ? 'Yes' : 'No'}</Badge></td>
                    <td className="p-2 text-xs text-gray-400">{relativeTime(c.last_seen_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {clients.length === 0 && <p className="text-gray-500 text-center py-8">No clients found</p>}
        </div>
      )}
    </div>
  );
}
