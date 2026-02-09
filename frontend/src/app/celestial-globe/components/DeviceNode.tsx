'use client';

import { memo, useCallback } from 'react';
import { Handle, Position } from 'reactflow';
import {
  Globe, GitBranch, Wifi, Monitor, Shield, Box, HardDrive, Server,
  type LucideIcon,
} from 'lucide-react';
import type { DeviceNodeData, NodeType } from '../types';
import { NODE_COLORS, NODE_SIZES, STATUS_COLORS } from '../constants';

const ICON_MAP: Record<string, LucideIcon> = {
  Globe, GitBranch, Wifi, Monitor, Shield, Box, HardDrive, Server,
};

const ICON_FOR_TYPE: Record<NodeType, string> = {
  controller: 'Globe',
  gateway: 'Globe',
  router: 'Globe',
  switch: 'GitBranch',
  ap: 'Wifi',
  client: 'Monitor',
  wg_peer: 'Shield',
  logic_device: 'Box',
  external: 'HardDrive',
  lpg_server: 'Server',
};

function DeviceNodeComponent({ data }: { data: DeviceNodeData }) {
  const { node, selected, onCollapse } = data;
  const nodeType = node.node_type as NodeType;
  const colors = NODE_COLORS[nodeType] || NODE_COLORS.client;
  const sizes = NODE_SIZES[nodeType] || NODE_SIZES.client;
  const iconName = ICON_FOR_TYPE[nodeType] || 'Monitor';
  const IconComponent = ICON_MAP[iconName] || Monitor;
  const statusColor = STATUS_COLORS[node.status] || STATUS_COLORS.unknown;

  const isOffline = node.status === 'offline' || node.status === 'inactive';

  const handleCollapseClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    onCollapse(node.id);
  }, [node.id, onCollapse]);

  return (
    <div
      className={`cg-node ${selected ? 'cg-node--selected' : ''} ${isOffline ? 'cg-node--offline' : ''}`}
      style={{
        width: sizes.width,
        minHeight: sizes.height,
        background: `linear-gradient(135deg, ${colors.bg}22, ${colors.bg}44)`,
        border: `1.5px solid ${isOffline ? '#4B5563' : colors.border}`,
        borderRadius: '8px',
        padding: '8px 10px',
        backdropFilter: 'blur(12px)',
        position: 'relative',
      }}
    >
      <Handle type="target" position={Position.Left} style={{ background: colors.border, width: 6, height: 6 }} />

      {/* Header row */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
        <IconComponent size={14} style={{ color: isOffline ? '#6B7280' : colors.bg, flexShrink: 0 }} />
        <span
          style={{
            color: isOffline ? '#6B7280' : '#E5E7EB',
            fontSize: nodeType === 'client' ? 10 : 12,
            fontWeight: 600,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            flex: 1,
          }}
        >
          {node.label}
        </span>
        <span className={`cg-status-dot cg-status-dot--${node.status}`} />
      </div>

      {/* IP / MAC */}
      {(node.ip || node.mac) && (
        <div style={{ fontSize: 9, color: '#9CA3AF', fontFamily: 'monospace', marginBottom: 2, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {node.ip}{node.ip && node.mac ? ' / ' : ''}{node.mac}
        </div>
      )}

      {/* Tags */}
      {(node.fid || node.connection_type === 'vpn' || node.connection_type === 'wireless') && (
        <div style={{ display: 'flex', gap: 3, flexWrap: 'wrap', marginTop: 2 }}>
          {node.fid && (
            <span style={{ fontSize: 8, padding: '1px 4px', borderRadius: 3, background: 'rgba(59,130,246,0.2)', color: '#60A5FA' }}>
              fid:{node.fid}
            </span>
          )}
          {node.connection_type === 'vpn' && (
            <span style={{ fontSize: 8, padding: '1px 4px', borderRadius: 3, background: 'rgba(16,185,129,0.2)', color: '#10B981' }}>
              VPN
            </span>
          )}
          {node.connection_type === 'wireless' && (
            <span style={{ fontSize: 8, padding: '1px 4px', borderRadius: 3, background: 'rgba(6,182,212,0.2)', color: '#06B6D4' }}>
              WiFi
            </span>
          )}
        </div>
      )}

      {/* Collapse badge */}
      {node.descendant_count > 0 && (
        <button
          onClick={handleCollapseClick}
          style={{
            position: 'absolute',
            right: -8,
            bottom: -8,
            cursor: 'pointer',
            border: 'none',
            background: 'none',
            padding: 0,
          }}
        >
          <span className="cg-collapse-badge">
            {node.collapsed ? `+${node.collapsed_child_count}` : node.descendant_count}
          </span>
        </button>
      )}

      <Handle type="source" position={Position.Right} style={{ background: colors.border, width: 6, height: 6 }} />
    </div>
  );
}

export const DeviceNode = memo(DeviceNodeComponent);
