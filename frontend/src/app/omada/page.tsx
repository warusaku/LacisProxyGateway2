'use client';

import { useCallback, useEffect, useState } from 'react';
import {
  omadaApi,
  type OmadaControllerDoc,
  type OmadaDeviceDoc,
  type OmadaClientDoc,
  type OmadaWgPeerDoc,
  type OmadaSummary,
  type OmadaTestResult,
} from '@/lib/api';

// ============================================================================
// Helper: Format bytes to human-readable
// ============================================================================
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function relativeTime(isoStr?: string): string {
  if (!isoStr) return '-';
  const diff = Date.now() - new Date(isoStr).getTime();
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.floor(hr / 24)}d ago`;
}

// ============================================================================
// Tab type
// ============================================================================
type TabId = 'controllers' | 'devices' | 'clients' | 'wireguard';

// ============================================================================
// Main page component
// ============================================================================
export default function OmadaPage() {
  const [activeTab, setActiveTab] = useState<TabId>('controllers');
  const [summary, setSummary] = useState<OmadaSummary | null>(null);

  const loadSummary = useCallback(async () => {
    try {
      const res = await omadaApi.getSummary();
      if (res.ok) setSummary(res.summary);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadSummary();
  }, [loadSummary]);

  const tabs: { id: TabId; label: string; count?: number }[] = [
    { id: 'controllers', label: 'Controllers', count: summary?.total_controllers },
    { id: 'devices', label: 'Devices', count: summary?.total_devices },
    { id: 'clients', label: 'Clients', count: summary?.active_clients },
    { id: 'wireguard', label: 'WireGuard', count: summary?.total_wg_peers },
  ];

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">OmadaControl</h1>

      {/* Summary badges */}
      {summary && (
        <div className="flex flex-wrap gap-3">
          <Badge label="Controllers" value={`${summary.connected_controllers}/${summary.total_controllers}`} color={summary.connected_controllers === summary.total_controllers ? 'green' : 'yellow'} />
          <Badge label="Devices" value={`${summary.online_devices}/${summary.total_devices}`} color={summary.online_devices === summary.total_devices ? 'green' : 'yellow'} />
          <Badge label="Clients" value={`${summary.active_clients}/${summary.total_clients}`} color="blue" />
          <Badge label="WG Peers" value={`${summary.active_wg_peers}/${summary.total_wg_peers}`} color="purple" />
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-1 border-b border-border">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab.label}
            {tab.count !== undefined && (
              <span className="ml-1.5 px-1.5 py-0.5 text-xs bg-gray-700 rounded-full">
                {tab.count}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {activeTab === 'controllers' && <ControllersTab onSync={loadSummary} />}
      {activeTab === 'devices' && <DevicesTab />}
      {activeTab === 'clients' && <ClientsTab />}
      {activeTab === 'wireguard' && <WireguardTab />}
    </div>
  );
}

// ============================================================================
// Badge component
// ============================================================================
function Badge({ label, value, color }: { label: string; value: string; color: string }) {
  const colors: Record<string, string> = {
    green: 'bg-green-900/50 text-green-300 border-green-700',
    yellow: 'bg-yellow-900/50 text-yellow-300 border-yellow-700',
    blue: 'bg-blue-900/50 text-blue-300 border-blue-700',
    purple: 'bg-purple-900/50 text-purple-300 border-purple-700',
    red: 'bg-red-900/50 text-red-300 border-red-700',
  };
  return (
    <span className={`px-3 py-1 text-sm rounded-md border ${colors[color] || colors.blue}`}>
      {label}: <strong>{value}</strong>
    </span>
  );
}

// ============================================================================
// Controllers Tab
// ============================================================================
function ControllersTab({ onSync }: { onSync: () => void }) {
  const [controllers, setControllers] = useState<OmadaControllerDoc[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [syncing, setSyncing] = useState<string | null>(null);

  // Form state
  const [formData, setFormData] = useState({
    display_name: '',
    base_url: '',
    client_id: '',
    client_secret: '',
  });
  const [testResult, setTestResult] = useState<OmadaTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [registering, setRegistering] = useState(false);
  const [formError, setFormError] = useState('');

  const loadControllers = useCallback(async () => {
    try {
      const res = await omadaApi.listControllers();
      if (res.ok) setControllers(res.controllers);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => {
    loadControllers();
  }, [loadControllers]);

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    setFormError('');
    try {
      const result = await omadaApi.testControllerConnection({
        base_url: formData.base_url,
        client_id: formData.client_id,
        client_secret: formData.client_secret,
      });
      setTestResult(result);
    } catch (e) {
      setFormError(e instanceof Error ? e.message : 'Test failed');
    }
    setTesting(false);
  };

  const handleRegister = async () => {
    setRegistering(true);
    setFormError('');
    try {
      const res = await omadaApi.registerController(formData);
      if (res.ok) {
        setShowForm(false);
        setFormData({ display_name: '', base_url: '', client_id: '', client_secret: '' });
        setTestResult(null);
        loadControllers();
        onSync();
      } else {
        setFormError(res.error || 'Registration failed');
      }
    } catch (e) {
      setFormError(e instanceof Error ? e.message : 'Registration failed');
    }
    setRegistering(false);
  };

  const handleSync = async (id: string) => {
    setSyncing(id);
    try {
      await omadaApi.syncController(id);
      loadControllers();
      onSync();
    } catch { /* ignore */ }
    setSyncing(null);
  };

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`Delete controller "${name}"? All associated data will be removed.`)) return;
    try {
      await omadaApi.deleteController(id);
      loadControllers();
      onSync();
    } catch { /* ignore */ }
  };

  if (loading) return <div className="text-gray-400 py-8">Loading controllers...</div>;

  return (
    <div className="space-y-4">
      {/* Controller cards */}
      {controllers.length === 0 && !showForm && (
        <div className="text-center py-12 text-gray-500">
          No controllers registered. Click &quot;Add Controller&quot; to get started.
        </div>
      )}

      <div className="grid gap-4 sm:grid-cols-1 lg:grid-cols-2">
        {controllers.map((ctrl) => (
          <div key={ctrl.controller_id} className="bg-card border border-border rounded-lg p-4 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className={`w-2.5 h-2.5 rounded-full ${
                  ctrl.status === 'connected' ? 'bg-green-400' :
                  ctrl.status === 'error' ? 'bg-red-400' : 'bg-gray-400'
                }`} />
                <h3 className="font-semibold text-lg">{ctrl.display_name}</h3>
              </div>
              <span className="text-xs text-gray-500">v{ctrl.controller_ver || '?'}</span>
            </div>

            <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm text-gray-400">
              <div>URL: <span className="text-gray-300">{ctrl.base_url}</span></div>
              <div>API: <span className="text-gray-300">v{ctrl.api_ver || '?'}</span></div>
              <div>ID: <span className="text-gray-300 font-mono text-xs">{ctrl.omadac_id}</span></div>
              <div>Synced: <span className="text-gray-300">{relativeTime(ctrl.last_synced_at)}</span></div>
            </div>

            {ctrl.last_error && (
              <div className="text-xs text-red-400 bg-red-900/20 px-2 py-1 rounded">{ctrl.last_error}</div>
            )}

            <div className="text-xs text-gray-500">
              Sites: {ctrl.sites.map(s => s.name).join(', ') || 'None'}
            </div>

            <div className="flex gap-2">
              <button
                onClick={() => handleSync(ctrl.controller_id)}
                disabled={syncing === ctrl.controller_id}
                className="px-3 py-1 text-xs bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded transition-colors"
              >
                {syncing === ctrl.controller_id ? 'Syncing...' : 'Sync Now'}
              </button>
              <button
                onClick={() => handleDelete(ctrl.controller_id, ctrl.display_name)}
                className="px-3 py-1 text-xs bg-red-600/30 hover:bg-red-600/50 text-red-300 rounded transition-colors"
              >
                Delete
              </button>
            </div>
          </div>
        ))}
      </div>

      {/* Add controller button / form */}
      {!showForm ? (
        <button
          onClick={() => setShowForm(true)}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-md transition-colors"
        >
          + Add Controller
        </button>
      ) : (
        <div className="bg-card border border-border rounded-lg p-6 space-y-4 max-w-xl">
          <h3 className="font-semibold text-lg">Register New Controller</h3>

          <div className="space-y-3">
            <div>
              <label className="block text-sm text-gray-400 mb-1">Display Name</label>
              <input
                type="text"
                value={formData.display_name}
                onChange={(e) => setFormData({ ...formData, display_name: e.target.value })}
                placeholder="e.g. Akihabara Office"
                className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-sm"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Controller URL</label>
              <input
                type="text"
                value={formData.base_url}
                onChange={(e) => setFormData({ ...formData, base_url: e.target.value })}
                placeholder="e.g. https://192.168.3.50"
                className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-sm"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Client ID</label>
              <input
                type="text"
                value={formData.client_id}
                onChange={(e) => setFormData({ ...formData, client_id: e.target.value })}
                className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-sm font-mono"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Client Secret</label>
              <input
                type="password"
                value={formData.client_secret}
                onChange={(e) => setFormData({ ...formData, client_secret: e.target.value })}
                className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-sm font-mono"
              />
            </div>
          </div>

          {/* Test result */}
          {testResult && (
            <div className={`p-3 rounded text-sm ${testResult.success ? 'bg-green-900/30 border border-green-700 text-green-300' : 'bg-red-900/30 border border-red-700 text-red-300'}`}>
              {testResult.success ? (
                <div className="space-y-1">
                  <div>Controller v{testResult.controller_ver}, API v{testResult.api_ver}</div>
                  <div>Omadac ID: {testResult.omadac_id}</div>
                  <div>Sites: {testResult.sites.map(s => s.name).join(', ')}</div>
                  <div>Devices: {testResult.device_count}</div>
                </div>
              ) : (
                <div>{testResult.error}</div>
              )}
            </div>
          )}

          {formError && (
            <div className="text-sm text-red-400">{formError}</div>
          )}

          <div className="flex gap-2">
            <button
              onClick={handleTest}
              disabled={testing || !formData.base_url || !formData.client_id || !formData.client_secret}
              className="px-4 py-2 text-sm bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-white rounded transition-colors"
            >
              {testing ? 'Testing...' : 'Test Connection'}
            </button>
            <button
              onClick={handleRegister}
              disabled={registering || !formData.display_name || !formData.base_url || !formData.client_id || !formData.client_secret}
              className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded transition-colors"
            >
              {registering ? 'Registering...' : 'Register'}
            </button>
            <button
              onClick={() => { setShowForm(false); setTestResult(null); setFormError(''); }}
              className="px-4 py-2 text-sm text-gray-400 hover:text-gray-200 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Devices Tab
// ============================================================================
function DevicesTab() {
  const [devices, setDevices] = useState<OmadaDeviceDoc[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState({ controller_id: '', site_id: '' });

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await omadaApi.getDevices(
        filter.controller_id || undefined,
        filter.site_id || undefined
      );
      if (res.ok) setDevices(res.devices);
    } catch { /* ignore */ }
    setLoading(false);
  }, [filter]);

  useEffect(() => { load(); }, [load]);

  const deviceTypeIcon = (type: string) => {
    switch (type.toLowerCase()) {
      case 'gateway': return 'Router';
      case 'switch': return 'Switch';
      case 'ap': return 'AP';
      default: return type;
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex gap-3 items-center">
        <input
          type="text"
          placeholder="Filter by controller ID..."
          value={filter.controller_id}
          onChange={(e) => setFilter({ ...filter, controller_id: e.target.value })}
          className="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded text-sm w-64"
        />
        <span className="text-sm text-gray-400">{devices.length} devices</span>
      </div>

      {loading ? (
        <div className="text-gray-400 py-4">Loading devices...</div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-gray-500 border-b border-border">
                <th className="pb-2 pr-4">Name</th>
                <th className="pb-2 pr-4">Type</th>
                <th className="pb-2 pr-4">Model</th>
                <th className="pb-2 pr-4">IP</th>
                <th className="pb-2 pr-4">MAC</th>
                <th className="pb-2 pr-4">Status</th>
                <th className="pb-2 pr-4">Firmware</th>
                <th className="pb-2">Synced</th>
              </tr>
            </thead>
            <tbody>
              {devices.map((d) => (
                <tr key={`${d.controller_id}-${d.mac}`} className="border-b border-gray-800 hover:bg-gray-800/50">
                  <td className="py-2 pr-4 font-medium">{d.name}</td>
                  <td className="py-2 pr-4">
                    <span className={`px-2 py-0.5 text-xs rounded ${
                      d.device_type === 'gateway' ? 'bg-blue-900/50 text-blue-300' :
                      d.device_type === 'switch' ? 'bg-green-900/50 text-green-300' :
                      'bg-purple-900/50 text-purple-300'
                    }`}>
                      {deviceTypeIcon(d.device_type)}
                    </span>
                  </td>
                  <td className="py-2 pr-4 text-gray-400">{d.model || '-'}</td>
                  <td className="py-2 pr-4 font-mono text-xs">{d.ip || '-'}</td>
                  <td className="py-2 pr-4 font-mono text-xs text-gray-500">{d.mac}</td>
                  <td className="py-2 pr-4">
                    <span className={`inline-block w-2 h-2 rounded-full ${d.status === 1 ? 'bg-green-400' : 'bg-red-400'}`} />
                    <span className="ml-1.5 text-xs">{d.status === 1 ? 'Online' : 'Offline'}</span>
                  </td>
                  <td className="py-2 pr-4 text-xs text-gray-500">{d.firmware_version || '-'}</td>
                  <td className="py-2 text-xs text-gray-500">{relativeTime(d.synced_at)}</td>
                </tr>
              ))}
              {devices.length === 0 && (
                <tr><td colSpan={8} className="text-center py-8 text-gray-500">No devices found</td></tr>
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Clients Tab
// ============================================================================
function ClientsTab() {
  const [clients, setClients] = useState<OmadaClientDoc[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeOnly, setActiveOnly] = useState(true);
  const [connType, setConnType] = useState<'all' | 'wireless' | 'wired'>('all');

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await omadaApi.getClients(undefined, undefined, activeOnly || undefined);
      if (res.ok) {
        let filtered = res.clients;
        if (connType === 'wireless') filtered = filtered.filter(c => c.wireless);
        if (connType === 'wired') filtered = filtered.filter(c => !c.wireless);
        setClients(filtered);
      }
    } catch { /* ignore */ }
    setLoading(false);
  }, [activeOnly, connType]);

  useEffect(() => { load(); }, [load]);

  return (
    <div className="space-y-4">
      <div className="flex gap-3 items-center flex-wrap">
        <label className="flex items-center gap-1.5 text-sm">
          <input
            type="checkbox"
            checked={activeOnly}
            onChange={(e) => setActiveOnly(e.target.checked)}
            className="rounded"
          />
          Active only
        </label>

        <div className="flex gap-1">
          {(['all', 'wireless', 'wired'] as const).map((t) => (
            <button
              key={t}
              onClick={() => setConnType(t)}
              className={`px-3 py-1 text-xs rounded ${
                connType === t ? 'bg-blue-600 text-white' : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
              }`}
            >
              {t.charAt(0).toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>

        <span className="text-sm text-gray-400">{clients.length} clients</span>
      </div>

      {loading ? (
        <div className="text-gray-400 py-4">Loading clients...</div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-gray-500 border-b border-border">
                <th className="pb-2 pr-3">Name</th>
                <th className="pb-2 pr-3">IP</th>
                <th className="pb-2 pr-3">MAC</th>
                <th className="pb-2 pr-3">Connection</th>
                <th className="pb-2 pr-3">Vendor</th>
                <th className="pb-2 pr-3">Traffic</th>
                <th className="pb-2 pr-3">Uptime</th>
                <th className="pb-2">Active</th>
              </tr>
            </thead>
            <tbody>
              {clients.map((c) => (
                <tr key={`${c.controller_id}-${c.mac}`} className="border-b border-gray-800 hover:bg-gray-800/50">
                  <td className="py-2 pr-3 font-medium">{c.name || c.host_name || '-'}</td>
                  <td className="py-2 pr-3 font-mono text-xs">{c.ip || '-'}</td>
                  <td className="py-2 pr-3 font-mono text-xs text-gray-500">{c.mac}</td>
                  <td className="py-2 pr-3 text-xs">
                    {c.wireless ? (
                      <span className="text-purple-300">{c.ssid || 'WiFi'} {c.rssi ? `(${c.rssi}dBm)` : ''}</span>
                    ) : (
                      <span className="text-green-300">{c.switch_name ? `${c.switch_name}:${c.port}` : 'Wired'}</span>
                    )}
                  </td>
                  <td className="py-2 pr-3 text-xs text-gray-400">{c.vendor || '-'}</td>
                  <td className="py-2 pr-3 text-xs text-gray-400">
                    {formatBytes(c.traffic_down)}/{formatBytes(c.traffic_up)}
                  </td>
                  <td className="py-2 pr-3 text-xs text-gray-400">{formatUptime(c.uptime)}</td>
                  <td className="py-2">
                    <span className={`inline-block w-2 h-2 rounded-full ${c.active ? 'bg-green-400' : 'bg-gray-500'}`} />
                  </td>
                </tr>
              ))}
              {clients.length === 0 && (
                <tr><td colSpan={8} className="text-center py-8 text-gray-500">No clients found</td></tr>
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// WireGuard Tab
// ============================================================================
function WireguardTab() {
  const [peers, setPeers] = useState<OmadaWgPeerDoc[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await omadaApi.getWireguard();
      if (res.ok) setPeers(res.peers);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { load(); }, [load]);

  // Group by interface
  const grouped = peers.reduce<Record<string, OmadaWgPeerDoc[]>>((acc, p) => {
    const key = p.interface_name;
    if (!acc[key]) acc[key] = [];
    acc[key].push(p);
    return acc;
  }, {});

  return (
    <div className="space-y-4">
      <div className="text-sm text-gray-400">{peers.length} peers</div>

      {loading ? (
        <div className="text-gray-400 py-4">Loading WireGuard peers...</div>
      ) : Object.keys(grouped).length === 0 ? (
        <div className="text-center py-8 text-gray-500">No WireGuard peers found</div>
      ) : (
        Object.entries(grouped).map(([ifName, ifPeers]) => (
          <div key={ifName} className="bg-card border border-border rounded-lg overflow-hidden">
            <div className="px-4 py-2 bg-gray-800/50 border-b border-border flex items-center gap-2">
              <span className="font-medium text-sm">{ifName}</span>
              <span className="text-xs text-gray-500">{ifPeers.length} peers</span>
            </div>
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-gray-500 border-b border-border">
                  <th className="px-4 pb-2 pt-2">Name</th>
                  <th className="px-4 pb-2 pt-2">Status</th>
                  <th className="px-4 pb-2 pt-2">Allowed IPs</th>
                  <th className="px-4 pb-2 pt-2">Public Key</th>
                  <th className="px-4 pb-2 pt-2">Comment</th>
                </tr>
              </thead>
              <tbody>
                {ifPeers.map((p) => (
                  <tr key={p.peer_id} className="border-b border-gray-800 hover:bg-gray-800/50">
                    <td className="px-4 py-2 font-medium">{p.name}</td>
                    <td className="px-4 py-2">
                      <span className={`inline-block w-2 h-2 rounded-full ${p.status ? 'bg-green-400' : 'bg-gray-500'}`} />
                      <span className="ml-1.5 text-xs">{p.status ? 'Enabled' : 'Disabled'}</span>
                    </td>
                    <td className="px-4 py-2 font-mono text-xs text-gray-400">
                      {p.allow_address.join(', ')}
                    </td>
                    <td className="px-4 py-2 font-mono text-xs text-gray-500" title={p.public_key}>
                      {p.public_key.slice(0, 16)}...
                    </td>
                    <td className="px-4 py-2 text-xs text-gray-500">{p.comment || '-'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ))
      )}
    </div>
  );
}
