'use client';

import { useCallback, useEffect, useState } from 'react';
import { Badge } from '@/components/ui/Badge';
import { Card } from '@/components/ui/Card';
import { openwrtApi } from '@/lib/api';
import type { OpenWrtRouterDoc, OpenWrtClientDoc, OpenWrtSummary } from '@/types';

function formatUptime(seconds?: number): string {
  if (!seconds) return '-';
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

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

export default function OpenWrtPage() {
  const [tab, setTab] = useState<'routers' | 'clients'>('routers');
  const [routers, setRouters] = useState<OpenWrtRouterDoc[]>([]);
  const [clients, setClients] = useState<OpenWrtClientDoc[]>([]);
  const [summary, setSummary] = useState<OpenWrtSummary | null>(null);
  const [loading, setLoading] = useState(true);

  // Registration form
  const [showForm, setShowForm] = useState(false);
  const [formData, setFormData] = useState({
    display_name: '', mac: '', ip: '', port: 22, username: 'root', password: '', firmware: 'openwrt',
  });
  const [formLoading, setFormLoading] = useState(false);
  const [formError, setFormError] = useState('');
  const [testResult, setTestResult] = useState<string>('');

  // Client filter
  const [filterRouter, setFilterRouter] = useState('');

  const loadData = useCallback(async () => {
    try {
      const [rRes, sRes] = await Promise.all([
        openwrtApi.listRouters(),
        openwrtApi.getSummary(),
      ]);
      if (rRes.ok) setRouters(rRes.routers);
      if (sRes.ok) setSummary(sRes.summary);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  const loadClients = useCallback(async () => {
    try {
      const res = await openwrtApi.getClients(filterRouter || undefined);
      if (res.ok) setClients(res.clients);
    } catch { /* ignore */ }
  }, [filterRouter]);

  useEffect(() => { loadData(); }, [loadData]);
  useEffect(() => {
    if (tab === 'clients') loadClients();
  }, [tab, loadClients]);

  useEffect(() => {
    const interval = setInterval(() => {
      loadData();
      if (tab === 'clients') loadClients();
    }, 30000);
    return () => clearInterval(interval);
  }, [loadData, loadClients, tab]);

  const handleTest = async () => {
    setTestResult('Testing...');
    try {
      const res = await openwrtApi.testConnection({
        ip: formData.ip,
        port: formData.port,
        username: formData.username,
        password: formData.password,
        firmware: formData.firmware,
      });
      if (res.success) {
        setTestResult('Connection successful');
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
      const res = await openwrtApi.registerRouter({
        ...formData,
        port: formData.port || 22,
      });
      if (res.ok) {
        setShowForm(false);
        setFormData({ display_name: '', mac: '', ip: '', port: 22, username: 'root', password: '', firmware: 'openwrt' });
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
    if (!confirm(`Delete router "${name}"?`)) return;
    try {
      await openwrtApi.deleteRouter(id);
      loadData();
    } catch { /* ignore */ }
  };

  const handlePoll = async (id: string) => {
    try {
      await openwrtApi.pollRouter(id);
      setTimeout(loadData, 2000);
    } catch { /* ignore */ }
  };

  if (loading) return <div className="text-gray-400">Loading...</div>;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">OpenWrt / AsusWrt</h1>
        <div className="flex gap-2">
          {summary && (
            <>
              <Badge variant={summary.online_routers > 0 ? 'success' : 'default'}>
                {summary.online_routers}/{summary.total_routers} Online
              </Badge>
              <Badge variant="info">{summary.active_clients} Clients</Badge>
            </>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 border-b border-border pb-2">
        {(['routers', 'clients'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`px-4 py-2 rounded-t text-sm font-medium transition-colors ${
              tab === t ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'
            }`}
          >
            {t === 'routers' ? `Routers (${routers.length})` : `Clients (${clients.length})`}
          </button>
        ))}
      </div>

      {/* Routers Tab */}
      {tab === 'routers' && (
        <div className="space-y-4">
          <button
            onClick={() => setShowForm(!showForm)}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm"
          >
            {showForm ? 'Cancel' : '+ Register Router'}
          </button>

          {showForm && (
            <Card title="Register Router">
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
                  <label className="block text-xs text-gray-400 mb-1">SSH Port</label>
                  <input type="number" className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={formData.port} onChange={(e) => setFormData({ ...formData, port: parseInt(e.target.value) || 22 })} />
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Username</label>
                  <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={formData.username} onChange={(e) => setFormData({ ...formData, username: e.target.value })} />
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Password</label>
                  <input type="password" className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={formData.password} onChange={(e) => setFormData({ ...formData, password: e.target.value })} />
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Firmware</label>
                  <select className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={formData.firmware} onChange={(e) => setFormData({ ...formData, firmware: e.target.value })}>
                    <option value="openwrt">OpenWrt</option>
                    <option value="asuswrt">AsusWrt</option>
                  </select>
                </div>
              </div>
              <div className="flex gap-2 mt-4">
                <button onClick={handleTest} className="px-4 py-2 bg-gray-600 text-white rounded text-sm hover:bg-gray-500" disabled={!formData.ip || !formData.username}>
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
            {routers.map((r) => (
              <Card key={r.router_id}>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <h3 className="font-medium">{r.display_name}</h3>
                    <Badge variant={r.status === 'online' ? 'success' : r.status === 'error' ? 'error' : 'default'}>
                      {r.status}
                    </Badge>
                  </div>
                  <div className="text-xs text-gray-400 space-y-1">
                    <div className="flex justify-between"><span>IP</span><span className="font-mono">{r.ip}:{r.port}</span></div>
                    <div className="flex justify-between"><span>Firmware</span><span>{r.firmware} {r.firmware_version || ''}</span></div>
                    <div className="flex justify-between"><span>WAN IP</span><span className="font-mono">{r.wan_ip || '-'}</span></div>
                    <div className="flex justify-between"><span>LAN IP</span><span className="font-mono">{r.lan_ip || '-'}</span></div>
                    <div className="flex justify-between"><span>SSID 2.4G</span><span>{r.ssid_24g || '-'}</span></div>
                    <div className="flex justify-between"><span>SSID 5G</span><span>{r.ssid_5g || '-'}</span></div>
                    <div className="flex justify-between"><span>Clients</span><span>{r.client_count}</span></div>
                    <div className="flex justify-between"><span>Uptime</span><span>{formatUptime(r.uptime_seconds)}</span></div>
                    <div className="flex justify-between"><span>Last Poll</span><span>{relativeTime(r.last_polled_at)}</span></div>
                    {r.last_error && <div className="text-red-400 text-xs mt-1">Error: {r.last_error}</div>}
                  </div>
                  <div className="flex gap-2 pt-2 border-t border-border">
                    <button onClick={() => handlePoll(r.router_id)} className="px-3 py-1 bg-gray-700 text-xs text-white rounded hover:bg-gray-600">Poll</button>
                    <button onClick={() => handleDelete(r.router_id, r.display_name)} className="px-3 py-1 bg-red-700 text-xs text-white rounded hover:bg-red-600">Delete</button>
                  </div>
                </div>
              </Card>
            ))}
          </div>
          {routers.length === 0 && <p className="text-gray-500 text-center py-8">No routers registered</p>}
        </div>
      )}

      {/* Clients Tab */}
      {tab === 'clients' && (
        <div className="space-y-4">
          <div className="flex gap-2 items-center">
            <select className="px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={filterRouter} onChange={(e) => setFilterRouter(e.target.value)}>
              <option value="">All Routers</option>
              {routers.map((r) => (
                <option key={r.router_id} value={r.router_id}>{r.display_name}</option>
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
                  <th className="p-2">Router</th>
                  <th className="p-2">Active</th>
                  <th className="p-2">Last Seen</th>
                </tr>
              </thead>
              <tbody>
                {clients.map((c) => (
                  <tr key={`${c.mac}-${c.router_id}`} className="border-b border-border/50 hover:bg-gray-800/50">
                    <td className="p-2 font-mono text-xs">{c.mac}</td>
                    <td className="p-2 font-mono">{c.ip}</td>
                    <td className="p-2">{c.hostname || '-'}</td>
                    <td className="p-2 text-xs">{routers.find((r) => r.router_id === c.router_id)?.display_name || c.router_id}</td>
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
