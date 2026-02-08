'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Modal } from '@/components/ui/Modal';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { Card } from '@/components/ui/Card';
import { araneaApi, type AraneaDevice } from '@/lib/api';

interface AraneaConfig {
  configured: boolean;
  tid?: string;
  tenant_user_id?: string;
  device_gate_url: string;
  device_state_url: string;
}

export default function AraneaSdkPage() {
  const [devices, setDevices] = useState<AraneaDevice[]>([]);
  const [config, setConfig] = useState<AraneaConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [isRegisterOpen, setIsRegisterOpen] = useState(false);
  const [isStateOpen, setIsStateOpen] = useState(false);
  const [selectedDevice, setSelectedDevice] = useState<AraneaDevice | null>(null);
  const [deviceStates, setDeviceStates] = useState<unknown[]>([]);
  const [registerForm, setRegisterForm] = useState({
    mac: '', product_type: '', product_code: '0000', device_type: 'araneaDevice',
  });
  const [error, setError] = useState('');

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      setLoading(true);
      const [summaryResp, devicesResp] = await Promise.all([
        araneaApi.getSummary().catch(() => null),
        araneaApi.listDevices().catch(() => null),
      ]);

      // Summary response is the config summary from backend
      if (summaryResp) {
        setConfig(summaryResp as unknown as AraneaConfig);
      }

      // Devices response - the Cloud Function returns varying shapes
      if (devicesResp && Array.isArray((devicesResp as Record<string, unknown>).devices)) {
        setDevices((devicesResp as { devices: AraneaDevice[] }).devices);
      } else if (devicesResp && typeof devicesResp === 'object') {
        // Cloud function might return different structure
        const resp = devicesResp as Record<string, unknown>;
        if (Array.isArray(resp.data)) {
          setDevices(resp.data as AraneaDevice[]);
        }
      }
    } catch (err) {
      console.error('Failed to load aranea data:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleRegister = async () => {
    setError('');
    try {
      await araneaApi.register(registerForm);
      setIsRegisterOpen(false);
      setRegisterForm({ mac: '', product_type: '', product_code: '0000', device_type: 'araneaDevice' });
      loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Registration failed');
    }
  };

  const handleViewState = async (device: AraneaDevice) => {
    setSelectedDevice(device);
    try {
      const resp = await araneaApi.getDeviceState(device.lacis_id);
      if (resp && Array.isArray((resp as Record<string, unknown>).states)) {
        setDeviceStates((resp as { states: unknown[] }).states);
      } else if (resp && typeof resp === 'object') {
        setDeviceStates([resp]);
      }
      setIsStateOpen(true);
    } catch (err) {
      console.error('Failed to get device state:', err);
    }
  };

  const columns = [
    { key: 'lacis_id', header: 'LacisID', render: (d: AraneaDevice) => <code className="text-xs text-blue-400">{d.lacis_id}</code> },
    { key: 'mac', header: 'MAC', render: (d: AraneaDevice) => <code className="text-xs">{d.mac}</code> },
    { key: 'product_type', header: 'Type', render: (d: AraneaDevice) => <span className="text-sm">{d.product_type}</span> },
    { key: 'device_type', header: 'Device', render: (d: AraneaDevice) => <span className="text-sm">{d.device_type}</span> },
    { key: 'mqtt_connected', header: 'MQTT', render: (d: AraneaDevice) => (
      d.mqtt_connected !== undefined
        ? <Badge variant={d.mqtt_connected ? 'success' : 'error'}>{d.mqtt_connected ? 'Connected' : 'Off'}</Badge>
        : <span className="text-gray-500">-</span>
    )},
    { key: 'last_seen', header: 'Last Seen', render: (d: AraneaDevice) => (
      <span className="text-xs text-gray-400">{d.last_seen ? new Date(d.last_seen).toLocaleString() : '-'}</span>
    )},
    { key: 'actions', header: 'Actions', render: (d: AraneaDevice) => (
      <Button size="sm" variant="ghost" onClick={() => handleViewState(d)}>State</Button>
    )},
  ];

  if (loading) {
    return <div className="flex items-center justify-center h-64"><div className="text-gray-400">Loading...</div></div>;
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">araneaSDK</h1>
        <Button onClick={() => setIsRegisterOpen(true)} disabled={!config?.configured}>
          Register Device
        </Button>
      </div>

      {/* Config Status */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <Card>
          <div className="p-4">
            <div className="text-sm text-gray-400">Status</div>
            <div className="mt-1">
              {config?.configured
                ? <Badge variant="success">Configured</Badge>
                : <Badge variant="error">Not Configured</Badge>
              }
            </div>
          </div>
        </Card>
        <Card>
          <div className="p-4">
            <div className="text-sm text-gray-400">Tenant</div>
            <div className="mt-1 text-sm">{config?.tenant_user_id || '-'}</div>
            <div className="text-xs text-gray-500">{config?.tid || 'No TID'}</div>
          </div>
        </Card>
        <Card>
          <div className="p-4">
            <div className="text-sm text-gray-400">Devices</div>
            <div className="mt-1 text-2xl font-bold">{devices.length}</div>
          </div>
        </Card>
      </div>

      {!config?.configured && (
        <div className="mb-4 p-3 bg-yellow-900/30 border border-yellow-700 rounded text-yellow-400 text-sm">
          araneaSDK is not configured. Set aranea.tid, aranea.tenant_lacis_id, aranea.tenant_user_id, and aranea.tenant_cic in config or environment variables (LACISPROXY__ARANEA__TID, etc.)
        </div>
      )}

      {/* Devices Table */}
      <Card>
        <Table columns={columns} data={devices} keyExtractor={(d) => d.lacis_id || d.mac} emptyMessage="No aranea devices registered" />
      </Card>

      {/* Register Modal */}
      <Modal isOpen={isRegisterOpen} onClose={() => setIsRegisterOpen(false)} title="Register araneaDevice">
        <div className="space-y-4">
          <Input label="MAC Address" placeholder="AA:BB:CC:DD:EE:FF" value={registerForm.mac}
            onChange={(e) => setRegisterForm({ ...registerForm, mac: e.target.value })} />
          <Input label="Product Type (3 digits)" placeholder="101" value={registerForm.product_type}
            onChange={(e) => setRegisterForm({ ...registerForm, product_type: e.target.value })} />
          <Input label="Product Code (4 digits)" placeholder="0000" value={registerForm.product_code}
            onChange={(e) => setRegisterForm({ ...registerForm, product_code: e.target.value })} />
          <Input label="Device Type" placeholder="araneaDevice" value={registerForm.device_type}
            onChange={(e) => setRegisterForm({ ...registerForm, device_type: e.target.value })} />
          {error && <p className="text-red-500 text-sm">{error}</p>}
          <div className="flex justify-end gap-2 pt-4">
            <Button variant="secondary" onClick={() => setIsRegisterOpen(false)}>Cancel</Button>
            <Button onClick={handleRegister}>Register</Button>
          </div>
        </div>
      </Modal>

      {/* State History Modal */}
      <Modal isOpen={isStateOpen} onClose={() => setIsStateOpen(false)} title={`State: ${selectedDevice?.lacis_id || ''}`}>
        <div className="max-h-[500px] overflow-auto">
          {deviceStates.length > 0 ? (
            <pre className="text-xs bg-gray-900 p-3 rounded overflow-auto">
              {JSON.stringify(deviceStates, null, 2)}
            </pre>
          ) : (
            <p className="text-gray-500 text-center py-4">No state data available</p>
          )}
        </div>
      </Modal>
    </div>
  );
}
