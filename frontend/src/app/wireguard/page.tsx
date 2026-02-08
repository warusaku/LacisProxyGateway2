'use client';

import { useCallback, useEffect, useState } from 'react';
import { Badge } from '@/components/ui/Badge';
import { Card } from '@/components/ui/Card';
import { wireguardApi, omadaApi } from '@/lib/api';
import type { OmadaWgPeerDoc, OmadaControllerDoc, OmadaSiteMapping, WgInterface } from '@/types';

export default function WireGuardPage() {
  const [tab, setTab] = useState<'peers' | 'create' | 'config'>('peers');
  const [peers, setPeers] = useState<OmadaWgPeerDoc[]>([]);
  const [interfaces, setInterfaces] = useState<WgInterface[]>([]);
  const [controllers, setControllers] = useState<OmadaControllerDoc[]>([]);
  const [loading, setLoading] = useState(true);

  // Create form
  const [createForm, setCreateForm] = useState({
    controller_id: '', site_id: '', interface_id: '', name: '',
    public_key: '', allow_address: '10.0.0.2/32', keep_alive: 25, comment: '',
  });
  const [generatedKeys, setGeneratedKeys] = useState<{ private_key: string; public_key: string } | null>(null);
  const [createLoading, setCreateLoading] = useState(false);
  const [createError, setCreateError] = useState('');
  const [createSuccess, setCreateSuccess] = useState('');

  // Config generator
  const [configForm, setConfigForm] = useState({
    private_key: '', address: '10.0.0.2/32', dns: '8.8.8.8',
    server_public_key: '', endpoint: '', allowed_ips: '0.0.0.0/0', persistent_keepalive: 25,
  });
  const [generatedConfig, setGeneratedConfig] = useState('');

  const loadData = useCallback(async () => {
    try {
      const [pRes, iRes, cRes] = await Promise.all([
        wireguardApi.getPeers(),
        wireguardApi.getInterfaces(),
        omadaApi.listControllers(),
      ]);
      if (pRes.ok) setPeers(pRes.peers);
      if (iRes.ok) setInterfaces(iRes.interfaces);
      if (cRes.ok) setControllers(cRes.controllers);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  useEffect(() => {
    const interval = setInterval(loadData, 60000);
    return () => clearInterval(interval);
  }, [loadData]);

  // Auto-populate site dropdown when controller changes
  const selectedController = controllers.find((c) => c.controller_id === createForm.controller_id);
  const sites: OmadaSiteMapping[] = selectedController?.sites || [];

  const handleGenerateKeys = async () => {
    try {
      const res = await wireguardApi.generateKeypair();
      if (res.ok) {
        setGeneratedKeys({ private_key: res.private_key, public_key: res.public_key });
        setCreateForm({ ...createForm, public_key: res.public_key });
      }
    } catch { /* ignore */ }
  };

  const handleCreate = async () => {
    setCreateLoading(true);
    setCreateError('');
    setCreateSuccess('');
    try {
      const res = await wireguardApi.createPeer({
        controller_id: createForm.controller_id,
        site_id: createForm.site_id,
        name: createForm.name,
        interface_id: createForm.interface_id,
        public_key: createForm.public_key,
        allow_address: createForm.allow_address.split(',').map((s) => s.trim()),
        keep_alive: createForm.keep_alive || undefined,
        comment: createForm.comment || undefined,
      });
      if (res.ok) {
        setCreateSuccess('Peer created successfully');
        loadData();
      } else {
        setCreateError(res.error || 'Creation failed');
      }
    } catch (e) {
      setCreateError(e instanceof Error ? e.message : 'Error');
    } finally {
      setCreateLoading(false);
    }
  };

  const handleDeletePeer = async (peer: OmadaWgPeerDoc) => {
    if (!confirm(`Delete peer "${peer.name}"?`)) return;
    try {
      await wireguardApi.deletePeer(peer.peer_id, peer.controller_id, peer.site_id);
      loadData();
    } catch { /* ignore */ }
  };

  const handleGenerateConfig = async () => {
    try {
      const res = await wireguardApi.generateConfig({
        ...configForm,
        persistent_keepalive: configForm.persistent_keepalive || undefined,
      });
      if (res.ok) setGeneratedConfig(res.config);
    } catch { /* ignore */ }
  };

  const downloadConfig = () => {
    const blob = new Blob([generatedConfig], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'wg0.conf';
    a.click();
    URL.revokeObjectURL(url);
  };

  // Group peers by interface
  const peersByInterface: Record<string, OmadaWgPeerDoc[]> = {};
  for (const p of peers) {
    const key = p.interface_name || p.interface_id;
    if (!peersByInterface[key]) peersByInterface[key] = [];
    peersByInterface[key].push(p);
  }

  if (loading) return <div className="text-gray-400">Loading...</div>;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">WireGuard</h1>
        <div className="flex gap-2">
          <Badge variant="info">{peers.length} Peers</Badge>
          <Badge variant="success">{peers.filter((p) => p.status).length} Active</Badge>
          <Badge variant="default">{interfaces.length} Interfaces</Badge>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 border-b border-border pb-2">
        {(['peers', 'create', 'config'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`px-4 py-2 rounded-t text-sm font-medium transition-colors ${
              tab === t ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'
            }`}
          >
            {t === 'peers' ? 'Peers' : t === 'create' ? 'Create Peer' : 'Config Generator'}
          </button>
        ))}
      </div>

      {/* Peers Tab */}
      {tab === 'peers' && (
        <div className="space-y-6">
          {Object.entries(peersByInterface).map(([ifName, ifPeers]) => (
            <Card key={ifName} title={`Interface: ${ifName}`}>
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-left text-gray-400 border-b border-border">
                    <th className="p-2">Name</th>
                    <th className="p-2">Status</th>
                    <th className="p-2">Public Key</th>
                    <th className="p-2">Allowed IPs</th>
                    <th className="p-2">KeepAlive</th>
                    <th className="p-2">Comment</th>
                    <th className="p-2">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {ifPeers.map((p) => (
                    <tr key={p.peer_id} className="border-b border-border/50 hover:bg-gray-800/50">
                      <td className="p-2 font-medium">{p.name}</td>
                      <td className="p-2"><Badge variant={p.status ? 'success' : 'default'}>{p.status ? 'Active' : 'Inactive'}</Badge></td>
                      <td className="p-2 font-mono text-xs truncate max-w-[180px]" title={p.public_key}>{p.public_key.substring(0, 20)}...</td>
                      <td className="p-2 text-xs">{p.allow_address.join(', ')}</td>
                      <td className="p-2">{p.keep_alive || '-'}</td>
                      <td className="p-2 text-xs text-gray-400">{p.comment || '-'}</td>
                      <td className="p-2">
                        <button onClick={() => handleDeletePeer(p)} className="px-2 py-1 bg-red-700 text-xs text-white rounded hover:bg-red-600">Delete</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Card>
          ))}
          {peers.length === 0 && <p className="text-gray-500 text-center py-8">No WireGuard peers found</p>}
        </div>
      )}

      {/* Create Peer Tab */}
      {tab === 'create' && (
        <Card title="Create WireGuard Peer">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-xs text-gray-400 mb-1">Controller</label>
              <select className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={createForm.controller_id} onChange={(e) => setCreateForm({ ...createForm, controller_id: e.target.value, site_id: '' })}>
                <option value="">Select Controller</option>
                {controllers.map((c) => (
                  <option key={c.controller_id} value={c.controller_id}>{c.display_name}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Site</label>
              <select className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={createForm.site_id} onChange={(e) => setCreateForm({ ...createForm, site_id: e.target.value })}>
                <option value="">Select Site</option>
                {sites.map((s) => (
                  <option key={s.site_id} value={s.site_id}>{s.name}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Interface</label>
              <select className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={createForm.interface_id} onChange={(e) => setCreateForm({ ...createForm, interface_id: e.target.value })}>
                <option value="">Select Interface</option>
                {interfaces
                  .filter((i) => !createForm.controller_id || i.controller_id === createForm.controller_id)
                  .map((i) => (
                    <option key={i.interface_id} value={i.interface_id}>{i.interface_name}</option>
                  ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Peer Name</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={createForm.name} onChange={(e) => setCreateForm({ ...createForm, name: e.target.value })} />
            </div>
            <div className="col-span-2">
              <label className="block text-xs text-gray-400 mb-1">Public Key</label>
              <div className="flex gap-2">
                <input className="flex-1 px-3 py-2 bg-gray-800 border border-border rounded text-sm font-mono" value={createForm.public_key} onChange={(e) => setCreateForm({ ...createForm, public_key: e.target.value })} />
                <button onClick={handleGenerateKeys} className="px-4 py-2 bg-gray-600 text-white rounded text-sm hover:bg-gray-500 whitespace-nowrap">Generate Keys</button>
              </div>
              {generatedKeys && (
                <div className="mt-2 p-2 bg-gray-800 rounded text-xs">
                  <div className="text-yellow-400 mb-1">Private Key (save this - shown only once):</div>
                  <div className="font-mono break-all select-all">{generatedKeys.private_key}</div>
                </div>
              )}
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Allowed IPs</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={createForm.allow_address} onChange={(e) => setCreateForm({ ...createForm, allow_address: e.target.value })} placeholder="10.0.0.2/32" />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Keep Alive (sec)</label>
              <input type="number" className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={createForm.keep_alive} onChange={(e) => setCreateForm({ ...createForm, keep_alive: parseInt(e.target.value) || 0 })} />
            </div>
            <div className="col-span-2">
              <label className="block text-xs text-gray-400 mb-1">Comment</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={createForm.comment} onChange={(e) => setCreateForm({ ...createForm, comment: e.target.value })} />
            </div>
          </div>
          <div className="mt-4">
            <button
              onClick={handleCreate}
              disabled={createLoading || !createForm.controller_id || !createForm.site_id || !createForm.name || !createForm.public_key || !createForm.interface_id}
              className="px-4 py-2 bg-blue-600 text-white rounded text-sm hover:bg-blue-700 disabled:opacity-50"
            >
              {createLoading ? 'Creating...' : 'Create Peer'}
            </button>
          </div>
          {createError && <p className="mt-2 text-sm text-red-400">{createError}</p>}
          {createSuccess && <p className="mt-2 text-sm text-green-400">{createSuccess}</p>}
        </Card>
      )}

      {/* Config Generator Tab */}
      {tab === 'config' && (
        <Card title="WireGuard Config Generator">
          <div className="grid grid-cols-2 gap-4">
            <div className="col-span-2">
              <label className="block text-xs text-gray-400 mb-1">Client Private Key</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm font-mono" value={configForm.private_key} onChange={(e) => setConfigForm({ ...configForm, private_key: e.target.value })} />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Client Address</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={configForm.address} onChange={(e) => setConfigForm({ ...configForm, address: e.target.value })} />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">DNS</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={configForm.dns} onChange={(e) => setConfigForm({ ...configForm, dns: e.target.value })} />
            </div>
            <div className="col-span-2">
              <label className="block text-xs text-gray-400 mb-1">Server Public Key</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm font-mono" value={configForm.server_public_key} onChange={(e) => setConfigForm({ ...configForm, server_public_key: e.target.value })} />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Server Endpoint</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" placeholder="vpn.example.com:51820" value={configForm.endpoint} onChange={(e) => setConfigForm({ ...configForm, endpoint: e.target.value })} />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Allowed IPs</label>
              <input className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={configForm.allowed_ips} onChange={(e) => setConfigForm({ ...configForm, allowed_ips: e.target.value })} />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Persistent Keepalive (sec)</label>
              <input type="number" className="w-full px-3 py-2 bg-gray-800 border border-border rounded text-sm" value={configForm.persistent_keepalive} onChange={(e) => setConfigForm({ ...configForm, persistent_keepalive: parseInt(e.target.value) || 0 })} />
            </div>
          </div>
          <div className="flex gap-2 mt-4">
            <button
              onClick={handleGenerateConfig}
              disabled={!configForm.private_key || !configForm.server_public_key || !configForm.endpoint}
              className="px-4 py-2 bg-blue-600 text-white rounded text-sm hover:bg-blue-700 disabled:opacity-50"
            >
              Generate Config
            </button>
            {generatedConfig && (
              <button onClick={downloadConfig} className="px-4 py-2 bg-green-600 text-white rounded text-sm hover:bg-green-700">
                Download .conf
              </button>
            )}
          </div>
          {generatedConfig && (
            <div className="mt-4">
              <pre className="p-4 bg-gray-900 rounded text-xs font-mono text-green-400 overflow-x-auto select-all">{generatedConfig}</pre>
            </div>
          )}
        </Card>
      )}
    </div>
  );
}
