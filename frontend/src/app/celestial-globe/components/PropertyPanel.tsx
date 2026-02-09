'use client';

import { useMemo, useCallback, useState } from 'react';
import { X, Trash2, Copy, Check, ArrowUpRight } from 'lucide-react';
import { useTopologyStore } from '../stores/useTopologyStore';
import { lacisIdApi } from '@/lib/api';
import { NODE_COLORS, STATUS_COLORS } from '../constants';
import type { TopologyNodeV2, NodeType } from '../types';

export function PropertyPanel() {
  const nodes = useTopologyStore(s => s.nodes);
  const selectedNodeId = useTopologyStore(s => s.selectedNodeId);
  const setSelectedNodeId = useTopologyStore(s => s.setSelectedNodeId);
  const deleteLogicDevice = useTopologyStore(s => s.deleteLogicDevice);
  const updateParent = useTopologyStore(s => s.updateParent);
  const fetchTopology = useTopologyStore(s => s.fetchTopology);
  const [assigning, setAssigning] = useState(false);
  const [reparenting, setReparenting] = useState(false);
  const [showReparent, setShowReparent] = useState(false);

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

  const handleReparent = useCallback(async (newParentId: string) => {
    if (!node) return;
    setReparenting(true);
    try {
      await updateParent(node.id, newParentId);
      setShowReparent(false);
    } catch (e) {
      console.error('Failed to reparent:', e);
    } finally {
      setReparenting(false);
    }
  }, [node, updateParent]);

  // Available parent candidates: all infra nodes except self and descendants
  const parentCandidates = useMemo(() => {
    if (!node) return [];
    const selfAndDescendants = new Set<string>();
    selfAndDescendants.add(node.id);
    // Simple BFS to find descendants
    const queue = [node.id];
    while (queue.length > 0) {
      const current = queue.shift()!;
      for (const n of nodes) {
        if (n.parent_id === current && !selfAndDescendants.has(n.id)) {
          selfAndDescendants.add(n.id);
          queue.push(n.id);
        }
      }
    }
    return nodes
      .filter(n => !selfAndDescendants.has(n.id) && n.id !== '__internet__')
      .sort((a, b) => a.label.localeCompare(b.label));
  }, [node, nodes]);

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

  // Resolve parent label for display
  const parentLabel = node.parent_id
    ? nodes.find(n => n.id === node.parent_id)?.label || node.parent_id
    : '(root)';

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
        {node.ip && <PropRow label="IP" value={node.ip} mono copyable />}
        {node.mac && <PropRow label="MAC" value={node.mac} mono copyable />}
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
            <CopyableValue value={node.lacis_id} color="#10B981" />
          </>
        ) : node.candidate_lacis_id ? (
          <>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
              <span style={{ fontSize: 10, padding: '1px 6px', borderRadius: 4, background: 'rgba(245,158,11,0.2)', color: '#F59E0B', fontWeight: 600 }}>
                Candidate
              </span>
            </div>
            <CopyableValue value={node.candidate_lacis_id} color="#F59E0B" />
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
                marginTop: 6,
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

      {/* Metadata — show all fields, no information suppression */}
      <div className="cg-section">
        <div className="cg-section-title">Metadata</div>
        {Object.keys(node.metadata).length === 0 ? (
          <div style={{ fontSize: 11, color: '#6B7280' }}>No metadata</div>
        ) : (
          Object.entries(node.metadata).map(([k, v]) => {
            if (v === null || v === undefined) return <PropRow key={k} label={k} value="null" />;
            const val = typeof v === 'object' ? JSON.stringify(v) : String(v);
            return <PropRow key={k} label={k} value={val} />;
          })
        )}
      </div>

      {/* Topology Info */}
      <div className="cg-section">
        <div className="cg-section-title">Topology</div>
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <PropRow label="Parent" value={parentLabel} />
          {node.source === 'logic' && (
            <button
              onClick={() => setShowReparent(!showReparent)}
              style={{
                border: 'none',
                background: 'none',
                color: showReparent ? '#3B82F6' : '#6B7280',
                cursor: 'pointer',
                padding: '0 2px',
                flexShrink: 0,
              }}
              title="Change parent"
            >
              <ArrowUpRight size={12} />
            </button>
          )}
        </div>
        {showReparent && node.source === 'logic' && (
          <div style={{ marginTop: 4, marginBottom: 4 }}>
            <select
              onChange={e => {
                if (e.target.value) handleReparent(e.target.value);
              }}
              disabled={reparenting}
              defaultValue=""
              style={{
                width: '100%',
                fontSize: 11,
                padding: '4px 6px',
                background: 'rgba(255,255,255,0.05)',
                border: '1px solid rgba(51,51,51,0.5)',
                borderRadius: 4,
                color: '#E5E7EB',
                outline: 'none',
              }}
            >
              <option value="" disabled>
                {reparenting ? 'Reparenting...' : 'Select new parent...'}
              </option>
              {parentCandidates.map(c => (
                <option key={c.id} value={c.id} style={{ background: '#1a1a1a' }}>
                  {c.label} ({c.node_type})
                </option>
              ))}
            </select>
          </div>
        )}
        <PropRow label="Descendants" value={String(node.descendant_count)} />
        <PropRow label="Position" value={`${node.position.x.toFixed(0)}, ${node.position.y.toFixed(0)}`} />
        <PropRow label="Pinned" value={node.position.pinned ? 'Yes' : 'No'} />
        <PropRow label="ID" value={node.id} mono copyable />
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

/** Inline copyable value with monospace font */
function CopyableValue({ value, color }: { value: string; color: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch { /* ignore */ }
  }, [value]);

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      <div style={{ fontFamily: 'monospace', fontSize: 11, color, wordBreak: 'break-all', flex: 1 }}>
        {value}
      </div>
      <button
        onClick={handleCopy}
        style={{ border: 'none', background: 'none', color: copied ? '#10B981' : '#6B7280', cursor: 'pointer', padding: 2, flexShrink: 0 }}
        title="Copy to clipboard"
      >
        {copied ? <Check size={12} /> : <Copy size={12} />}
      </button>
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
          {m.wan_ip ? <PropRow label="WAN IP" value={String(m.wan_ip)} mono copyable /> : null}
          {m.lan_ip ? <PropRow label="LAN IP" value={String(m.lan_ip)} mono copyable /> : null}
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
          {m.public_key ? <PropRow label="Public Key" value={String(m.public_key)} mono copyable /> : null}
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

function PropRow({ label, value, mono, copyable }: { label: string; value: string; mono?: boolean; copyable?: boolean }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch { /* ignore */ }
  }, [value]);

  return (
    <div className="cg-prop-row" style={{ display: 'flex', alignItems: 'center' }}>
      <span className="cg-prop-label">{label}</span>
      <span className={`cg-prop-value ${mono ? 'cg-prop-value--mono' : ''}`} title={value} style={{ flex: 1 }}>
        {value}
      </span>
      {copyable && (
        <button
          onClick={handleCopy}
          style={{ border: 'none', background: 'none', color: copied ? '#10B981' : '#4B5563', cursor: 'pointer', padding: '0 2px', flexShrink: 0 }}
          title="Copy"
        >
          {copied ? <Check size={10} /> : <Copy size={10} />}
        </button>
      )}
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
