'use client';

import { useMemo, useCallback, useState } from 'react';
import { X, Trash2, Edit3 } from 'lucide-react';
import { useTopologyStore } from '../stores/useTopologyStore';
import { lacisIdApi } from '@/lib/api';
import { NODE_COLORS, STATUS_COLORS } from '../constants';
import type { TopologyNodeV2, NodeType } from '../types';

export function PropertyPanel() {
  const nodes = useTopologyStore(s => s.nodes);
  const selectedNodeId = useTopologyStore(s => s.selectedNodeId);
  const setSelectedNodeId = useTopologyStore(s => s.setSelectedNodeId);
  const deleteLogicDevice = useTopologyStore(s => s.deleteLogicDevice);
  const fetchTopology = useTopologyStore(s => s.fetchTopology);
  const [assigning, setAssigning] = useState(false);

  const node = useMemo(
    () => nodes.find(n => n.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId]
  );

  const handleAssignLacisId = useCallback(async (n: TopologyNodeV2) => {
    if (!n.candidate_lacis_id || !n.mac) return;
    setAssigning(true);
    try {
      const idParts = n.id.split(':');
      const actualDeviceId = n.source === 'omada' ? n.mac : (idParts.length >= 2 ? idParts[1] : n.mac);
      await lacisIdApi.assign(actualDeviceId, n.source, n.candidate_lacis_id);
      await fetchTopology();
    } catch (e) {
      console.error('Failed to assign lacis_id:', e);
    } finally {
      setAssigning(false);
    }
  }, [fetchTopology]);

  const handleDelete = useCallback(async () => {
    if (!node || node.source !== 'logic') return;
    if (!confirm(`Delete logic device "${node.label}"?`)) return;
    await deleteLogicDevice(node.id);
    setSelectedNodeId(null);
  }, [node, deleteLogicDevice, setSelectedNodeId]);

  if (!node) {
    return (
      <div className="cg-glass-card" style={{ width: '100%', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#6B7280', fontSize: 13 }}>
        Select a node to view details
      </div>
    );
  }

  const nodeType = node.node_type as NodeType;
  const colors = NODE_COLORS[nodeType] || NODE_COLORS.client;
  const statusColor = STATUS_COLORS[node.status] || STATUS_COLORS.unknown;

  return (
    <div className="cg-glass-card" style={{ width: '100%', height: '100%', overflow: 'auto', padding: '12px 14px' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 8 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <h3 style={{ fontSize: 15, fontWeight: 700, color: '#E5E7EB', margin: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {node.label}
          </h3>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 4 }}>
            <span style={{ fontSize: 10, padding: '1px 6px', borderRadius: 4, background: `${colors.bg}33`, color: colors.bg, fontWeight: 600 }}>
              {node.node_type}
            </span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 11, color: statusColor }}>
              <span className={`cg-status-dot cg-status-dot--${node.status}`} />
              {node.status}
            </span>
          </div>
        </div>
        <button
          onClick={() => setSelectedNodeId(null)}
          style={{ border: 'none', background: 'none', color: '#6B7280', cursor: 'pointer', padding: 2 }}
        >
          <X size={16} />
        </button>
      </div>

      {/* Basic Info */}
      <div className="cg-section">
        <div className="cg-section-title">Basic Info</div>
        <PropRow label="Source" value={node.source} />
        {node.ip && <PropRow label="IP" value={node.ip} mono />}
        {node.mac && <PropRow label="MAC" value={node.mac} mono />}
        {node.product_type && <PropRow label="Product Type" value={node.product_type} />}
        {node.network_device_type && <PropRow label="Device Type" value={node.network_device_type} />}
        {node.fid && <PropRow label="Facility ID" value={node.fid} />}
        {node.facility_name && <PropRow label="Facility" value={node.facility_name} />}
        <PropRow label="Connection" value={node.connection_type} />
      </div>

      {/* LacisID */}
      <div className="cg-section">
        <div className="cg-section-title">LacisID</div>
        {node.lacis_id ? (
          <>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
              <span style={{ fontSize: 10, padding: '1px 6px', borderRadius: 4, background: 'rgba(16,185,129,0.2)', color: '#10B981', fontWeight: 600 }}>
                Assigned
              </span>
            </div>
            <div style={{ fontFamily: 'monospace', fontSize: 11, color: '#10B981', wordBreak: 'break-all' }}>
              {node.lacis_id}
            </div>
          </>
        ) : node.candidate_lacis_id ? (
          <>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
              <span style={{ fontSize: 10, padding: '1px 6px', borderRadius: 4, background: 'rgba(245,158,11,0.2)', color: '#F59E0B', fontWeight: 600 }}>
                Candidate
              </span>
            </div>
            <div style={{ fontFamily: 'monospace', fontSize: 11, color: '#F59E0B', wordBreak: 'break-all', marginBottom: 6 }}>
              {node.candidate_lacis_id}
            </div>
            <button
              onClick={() => handleAssignLacisId(node)}
              disabled={assigning}
              style={{
                width: '100%',
                padding: '6px 0',
                fontSize: 12,
                fontWeight: 600,
                borderRadius: 6,
                border: '1px solid rgba(59,130,246,0.5)',
                background: 'rgba(59,130,246,0.15)',
                color: '#60A5FA',
                cursor: assigning ? 'wait' : 'pointer',
              }}
            >
              {assigning ? 'Assigning...' : 'Assign LacisID'}
            </button>
          </>
        ) : (
          <div style={{ fontSize: 12, color: '#6B7280' }}>N/A</div>
        )}
      </div>

      {/* Type-specific sections */}
      <TypeSpecificSection node={node} />

      {/* Metadata */}
      <div className="cg-section">
        <div className="cg-section-title">Metadata</div>
        {Object.entries(node.metadata).map(([k, v]) => {
          if (v === null || v === undefined) return null;
          const val = typeof v === 'object' ? JSON.stringify(v) : String(v);
          return <PropRow key={k} label={k} value={val} />;
        })}
      </div>

      {/* Topology Info */}
      <div className="cg-section">
        <div className="cg-section-title">Topology</div>
        <PropRow label="Parent" value={node.parent_id || '(root)'} mono />
        <PropRow label="Descendants" value={String(node.descendant_count)} />
        <PropRow label="Position" value={`${node.position.x.toFixed(0)}, ${node.position.y.toFixed(0)}`} />
        <PropRow label="Pinned" value={node.position.pinned ? 'Yes' : 'No'} />
      </div>

      {/* LogicDevice actions */}
      {node.source === 'logic' && (
        <div className="cg-section" style={{ display: 'flex', gap: 8 }}>
          <button
            onClick={handleDelete}
            style={{
              flex: 1,
              padding: '6px 0',
              fontSize: 12,
              fontWeight: 600,
              borderRadius: 6,
              border: '1px solid rgba(239,68,68,0.5)',
              background: 'rgba(239,68,68,0.15)',
              color: '#EF4444',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 4,
            }}
          >
            <Trash2 size={12} /> Delete
          </button>
        </div>
      )}
    </div>
  );
}

function TypeSpecificSection({ node }: { node: TopologyNodeV2 }) {
  const m = node.metadata as Record<string, string | number | boolean | null | undefined | unknown[]>;

  switch (node.node_type) {
    case 'router':
    case 'gateway':
      return (
        <div className="cg-section">
          <div className="cg-section-title">Network</div>
          {m.wan_ip ? <PropRow label="WAN IP" value={String(m.wan_ip)} mono /> : null}
          {m.lan_ip ? <PropRow label="LAN IP" value={String(m.lan_ip)} mono /> : null}
          {m.ssid_24g ? <PropRow label="SSID 2.4G" value={String(m.ssid_24g)} /> : null}
          {m.ssid_5g ? <PropRow label="SSID 5G" value={String(m.ssid_5g)} /> : null}
          {m.client_count !== undefined ? <PropRow label="Clients" value={String(m.client_count)} /> : null}
        </div>
      );

    case 'client':
      return (
        <div className="cg-section">
          <div className="cg-section-title">Client Info</div>
          {m.vendor ? <PropRow label="Vendor" value={String(m.vendor)} /> : null}
          {m.os_name ? <PropRow label="OS" value={String(m.os_name)} /> : null}
          {m.ssid ? <PropRow label="SSID" value={String(m.ssid)} /> : null}
          {m.signal_level != null ? <PropRow label="Signal" value={`${m.signal_level} dBm`} /> : null}
          {m.traffic_down !== undefined ? <PropRow label="Traffic Down" value={formatBytes(Number(m.traffic_down))} /> : null}
          {m.traffic_up !== undefined ? <PropRow label="Traffic Up" value={formatBytes(Number(m.traffic_up))} /> : null}
        </div>
      );

    case 'wg_peer':
      return (
        <div className="cg-section">
          <div className="cg-section-title">WireGuard</div>
          {m.interface_name ? <PropRow label="Interface" value={String(m.interface_name)} /> : null}
          {m.public_key ? <PropRow label="Public Key" value={String(m.public_key)} mono /> : null}
          {m.allow_address ? <PropRow label="Allowed IPs" value={JSON.stringify(m.allow_address)} /> : null}
        </div>
      );

    case 'external':
      return (
        <div className="cg-section">
          <div className="cg-section-title">External</div>
          {m.protocol ? <PropRow label="Protocol" value={String(m.protocol)} /> : null}
          {m.device_model ? <PropRow label="Model" value={String(m.device_model)} /> : null}
          {m.client_count !== undefined ? <PropRow label="Clients" value={String(m.client_count)} /> : null}
        </div>
      );

    case 'logic_device':
      return (
        <div className="cg-section">
          <div className="cg-section-title">Logic Device</div>
          {m.location ? <PropRow label="Location" value={String(m.location)} /> : null}
          {m.note ? <PropRow label="Note" value={String(m.note)} /> : null}
        </div>
      );

    default:
      return null;
  }
}

function PropRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="cg-prop-row">
      <span className="cg-prop-label">{label}</span>
      <span className={`cg-prop-value ${mono ? 'cg-prop-value--mono' : ''}`} title={value}>
        {value}
      </span>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}
