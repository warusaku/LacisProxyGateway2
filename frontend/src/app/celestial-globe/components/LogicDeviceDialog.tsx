'use client';

import { useState, useCallback } from 'react';
import { X } from 'lucide-react';
import { useTopologyStore } from '../stores/useTopologyStore';
import { LOGIC_DEVICE_TYPE_LABELS } from '../constants';
import type { LogicDeviceType } from '../types';

interface LogicDeviceDialogProps {
  open: boolean;
  onClose: () => void;
}

const DEVICE_TYPES: LogicDeviceType[] = ['switch', 'hub', 'converter', 'ups', 'other'];

export function LogicDeviceDialog({ open, onClose }: LogicDeviceDialogProps) {
  const nodes = useTopologyStore(s => s.nodes);
  const createLogicDevice = useTopologyStore(s => s.createLogicDevice);
  const loading = useTopologyStore(s => s.loading);

  const [label, setLabel] = useState('');
  const [deviceType, setDeviceType] = useState<LogicDeviceType>('switch');
  const [parentId, setParentId] = useState('');
  const [ip, setIp] = useState('');
  const [location, setLocation] = useState('');
  const [note, setNote] = useState('');

  const handleSubmit = useCallback(async () => {
    if (!label.trim()) return;
    await createLogicDevice({
      label: label.trim(),
      device_type: deviceType,
      parent_id: parentId || undefined,
      ip: ip || undefined,
      location: location || undefined,
      note: note || undefined,
    });
    // Reset form
    setLabel('');
    setDeviceType('switch');
    setParentId('');
    setIp('');
    setLocation('');
    setNote('');
    onClose();
  }, [label, deviceType, parentId, ip, location, note, createLogicDevice, onClose]);

  if (!open) return null;

  // Filter potential parent nodes (devices, not clients)
  const parentCandidates = nodes.filter(
    n => n.node_type !== 'client' && n.node_type !== 'wg_peer'
  );

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 100,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.6)',
        backdropFilter: 'blur(4px)',
      }}
      onClick={onClose}
    >
      <div
        className="cg-glass-card"
        style={{ width: 400, padding: 20 }}
        onClick={e => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
          <h3 style={{ fontSize: 15, fontWeight: 700, color: '#E5E7EB', margin: 0 }}>
            Add Logic Device
          </h3>
          <button onClick={onClose} style={{ border: 'none', background: 'none', color: '#6B7280', cursor: 'pointer' }}>
            <X size={16} />
          </button>
        </div>

        {/* Form */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <Field label="Label *">
            <input
              type="text"
              value={label}
              onChange={e => setLabel(e.target.value)}
              placeholder="e.g. UPS-01"
              style={inputStyle}
            />
          </Field>

          <Field label="Device Type">
            <select value={deviceType} onChange={e => setDeviceType(e.target.value as LogicDeviceType)} style={inputStyle}>
              {DEVICE_TYPES.map(dt => (
                <option key={dt} value={dt}>{LOGIC_DEVICE_TYPE_LABELS[dt]}</option>
              ))}
            </select>
          </Field>

          <Field label="Parent Node">
            <select value={parentId} onChange={e => setParentId(e.target.value)} style={inputStyle}>
              <option value="">None (root)</option>
              {parentCandidates.map(n => (
                <option key={n.id} value={n.id}>{n.label} ({n.node_type})</option>
              ))}
            </select>
          </Field>

          <Field label="IP Address">
            <input
              type="text"
              value={ip}
              onChange={e => setIp(e.target.value)}
              placeholder="e.g. 192.168.1.100"
              style={inputStyle}
            />
          </Field>

          <Field label="Location">
            <input
              type="text"
              value={location}
              onChange={e => setLocation(e.target.value)}
              placeholder="e.g. Server Room A"
              style={inputStyle}
            />
          </Field>

          <Field label="Note">
            <textarea
              value={note}
              onChange={e => setNote(e.target.value)}
              placeholder="Additional notes..."
              rows={2}
              style={{ ...inputStyle, resize: 'vertical' }}
            />
          </Field>

          <button
            onClick={handleSubmit}
            disabled={!label.trim() || loading}
            style={{
              width: '100%',
              padding: '8px 0',
              fontSize: 13,
              fontWeight: 600,
              borderRadius: 6,
              border: '1px solid rgba(59,130,246,0.5)',
              background: label.trim() ? 'rgba(59,130,246,0.2)' : 'rgba(51,51,51,0.3)',
              color: label.trim() ? '#60A5FA' : '#6B7280',
              cursor: label.trim() && !loading ? 'pointer' : 'not-allowed',
            }}
          >
            {loading ? 'Creating...' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label style={{ fontSize: 11, color: '#9CA3AF', display: 'block', marginBottom: 4 }}>{label}</label>
      {children}
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  width: '100%',
  padding: '6px 10px',
  fontSize: 13,
  background: 'rgba(255,255,255,0.05)',
  border: '1px solid rgba(51,51,51,0.5)',
  borderRadius: 6,
  color: '#E5E7EB',
  outline: 'none',
};
