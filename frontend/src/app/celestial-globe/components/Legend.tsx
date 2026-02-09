'use client';

import { useState } from 'react';
import {
  Cloud, Globe, GitBranch, Wifi, Monitor, Shield, Box, HardDrive, Server,
  Info, X,
  type LucideIcon,
} from 'lucide-react';
import type { NodeType } from '../types';
import { NODE_COLORS, STATUS_COLORS, EDGE_STYLES } from '../constants';

const DEVICE_TYPES: { type: NodeType; label: string; icon: LucideIcon }[] = [
  { type: 'internet', label: 'Internet', icon: Cloud },
  { type: 'controller', label: 'Controller', icon: Globe },
  { type: 'gateway', label: 'Gateway', icon: Globe },
  { type: 'router', label: 'Router', icon: Globe },
  { type: 'switch', label: 'Switch', icon: GitBranch },
  { type: 'ap', label: 'Access Point', icon: Wifi },
  { type: 'client', label: 'Client', icon: Monitor },
  { type: 'wg_peer', label: 'WireGuard Peer', icon: Shield },
  { type: 'logic_device', label: 'Logic Device', icon: Box },
  { type: 'external', label: 'External', icon: HardDrive },
  { type: 'lpg_server', label: 'LPG Server', icon: Server },
];

const STATUSES: { status: string; label: string }[] = [
  { status: 'online', label: 'Online' },
  { status: 'active', label: 'Active' },
  { status: 'offline', label: 'Offline' },
  { status: 'inactive', label: 'Inactive' },
  { status: 'warning', label: 'Warning' },
  { status: 'unknown', label: 'Unknown' },
];

const EDGE_TYPES: { type: string; label: string }[] = [
  { type: 'wired', label: 'Wired' },
  { type: 'wireless', label: 'Wireless' },
  { type: 'vpn', label: 'VPN' },
  { type: 'logical', label: 'Logical' },
  { type: 'route', label: 'Route' },
];

export function Legend() {
  const [expanded, setExpanded] = useState(false);

  if (!expanded) {
    return (
      <button
        onClick={() => setExpanded(true)}
        style={{
          position: 'absolute',
          bottom: 16,
          left: 16,
          zIndex: 10,
          width: 32,
          height: 32,
          borderRadius: 8,
          background: 'rgba(10,10,10,0.9)',
          backdropFilter: 'blur(12px)',
          border: '1px solid rgba(51,51,51,0.5)',
          color: '#9CA3AF',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
        title="Show Legend"
      >
        <Info size={16} />
      </button>
    );
  }

  return (
    <div
      className="cg-glass-card"
      style={{
        position: 'absolute',
        bottom: 16,
        left: 16,
        zIndex: 10,
        width: 220,
        padding: 0,
        fontSize: 11,
        maxHeight: 400,
        overflow: 'auto',
      }}
    >
      {/* Header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: '8px 12px',
        borderBottom: '1px solid rgba(51,51,51,0.5)',
      }}>
        <span style={{ fontWeight: 600, color: '#E5E7EB', fontSize: 12 }}>Legend</span>
        <button
          onClick={() => setExpanded(false)}
          style={{ border: 'none', background: 'none', color: '#6B7280', cursor: 'pointer', padding: 0 }}
        >
          <X size={14} />
        </button>
      </div>

      {/* Device Types */}
      <div style={{ padding: '8px 12px' }}>
        <div style={{ fontSize: 10, fontWeight: 600, color: '#6B7280', textTransform: 'uppercase', marginBottom: 6 }}>
          Device Types
        </div>
        {DEVICE_TYPES.map(({ type: t, label, icon: Icon }) => {
          const c = NODE_COLORS[t];
          return (
            <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
              <div style={{
                width: 20,
                height: 20,
                borderRadius: '50%',
                background: c.bg,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}>
                <Icon size={10} style={{ color: '#fff' }} />
              </div>
              <span style={{ color: '#D1D5DB' }}>{label}</span>
            </div>
          );
        })}
      </div>

      {/* Status */}
      <div style={{ padding: '4px 12px 8px' }}>
        <div style={{ fontSize: 10, fontWeight: 600, color: '#6B7280', textTransform: 'uppercase', marginBottom: 6 }}>
          Status
        </div>
        {STATUSES.map(({ status, label }) => (
          <div key={status} style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
            <div style={{
              width: 12,
              height: 12,
              borderRadius: '50%',
              background: STATUS_COLORS[status] || '#9CA3AF',
            }} />
            <span style={{ color: '#D1D5DB' }}>{label}</span>
          </div>
        ))}
      </div>

      {/* Edge Types */}
      <div style={{ padding: '4px 12px 10px' }}>
        <div style={{ fontSize: 10, fontWeight: 600, color: '#6B7280', textTransform: 'uppercase', marginBottom: 6 }}>
          Connections
        </div>
        {EDGE_TYPES.map(({ type: t, label }) => {
          const s = EDGE_STYLES[t as keyof typeof EDGE_STYLES];
          if (!s) return null;
          return (
            <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
              <svg width="28" height="12" style={{ flexShrink: 0 }}>
                <line
                  x1="0" y1="6" x2="28" y2="6"
                  stroke={s.color}
                  strokeWidth={s.strokeWidth}
                  strokeDasharray={s.strokeDasharray || 'none'}
                />
              </svg>
              <span style={{ color: '#D1D5DB' }}>{label}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
