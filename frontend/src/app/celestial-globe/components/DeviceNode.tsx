'use client';

import { memo, useCallback } from 'react';
import { Handle, Position } from 'reactflow';
import {
  Globe, GitBranch, Wifi, Monitor, Shield, Box, HardDrive, Server,
  type LucideIcon,
} from 'lucide-react';
import type { DeviceNodeData, NodeType } from '../types';
import { NODE_COLORS, STATUS_COLORS } from '../constants';

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
  const iconName = ICON_FOR_TYPE[nodeType] || 'Monitor';
  const IconComponent = ICON_MAP[iconName] || Monitor;
  const statusColor = STATUS_COLORS[node.status] || STATUS_COLORS.unknown;

  const isOffline = node.status === 'offline' || node.status === 'inactive';
  const isClient = nodeType === 'client' || nodeType === 'wg_peer';

  const handleCollapseClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    onCollapse(node.id);
  }, [node.id, onCollapse]);

  // Node width: infrastructure nodes wider, clients narrower
  const nodeWidth = isClient ? 160 : 200;

  return (
    <div
      className={`cg-node ${selected ? 'cg-node--selected' : ''} ${isOffline ? 'cg-node--offline' : ''}`}
      style={{
        width: nodeWidth,
        background: `linear-gradient(135deg, ${colors.bg}18, ${colors.bg}30)`,
        border: `1.5px solid ${selected ? '#3B82F6' : isOffline ? '#4B5563' : colors.border}`,
        borderRadius: 8,
        padding: '10px 12px',
        backdropFilter: 'blur(12px)',
        position: 'relative',
      }}
    >
      <Handle type="target" position={Position.Top} style={{ background: colors.border, width: 6, height: 6 }} />

      {/* Header row: icon circle + label + status indicator */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        {/* Icon with status-colored circular background */}
        <div
          style={{
            width: 28,
            height: 28,
            borderRadius: '50%',
            background: isOffline ? '#4B5563' : statusColor,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: 0,
          }}
        >
          <IconComponent size={14} style={{ color: '#FFFFFF' }} />
        </div>

        {/* Label + type */}
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              color: isOffline ? '#6B7280' : '#E5E7EB',
              fontSize: 12,
              fontWeight: 600,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              lineHeight: '16px',
            }}
            title={node.label}
          >
            {node.label}
          </div>
          <div style={{
            fontSize: 10,
            color: '#6B7280',
            lineHeight: '14px',
          }}>
            {node.node_type}
          </div>
        </div>
      </div>

      {/* IP address */}
      {node.ip && (
        <div style={{
          fontSize: 11,
          color: '#9CA3AF',
          fontFamily: 'monospace',
          marginTop: 6,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}>
          {node.ip}
        </div>
      )}

      {/* MAC address (formatted) */}
      {node.mac && !isClient && (
        <div style={{
          fontSize: 9.5,
          color: '#6B7280',
          fontFamily: 'monospace',
          marginTop: 2,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}
          title={node.mac}
        >
          {node.mac}
        </div>
      )}

      {/* Tags: connection type + facility */}
      {(node.fid || node.connection_type === 'vpn' || node.connection_type === 'wireless') && (
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
          {node.fid && (
            <span style={{ fontSize: 9, padding: '1px 5px', borderRadius: 4, background: 'rgba(59,130,246,0.15)', color: '#60A5FA' }}>
              fid:{node.fid}
            </span>
          )}
          {node.connection_type === 'vpn' && (
            <span style={{ fontSize: 9, padding: '1px 5px', borderRadius: 4, background: 'rgba(16,185,129,0.15)', color: '#10B981' }}>
              VPN
            </span>
          )}
          {node.connection_type === 'wireless' && (
            <span style={{ fontSize: 9, padding: '1px 5px', borderRadius: 4, background: 'rgba(6,182,212,0.15)', color: '#06B6D4' }}>
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

      <Handle type="source" position={Position.Bottom} style={{ background: colors.border, width: 6, height: 6 }} />
    </div>
  );
}

export const DeviceNode = memo(DeviceNodeComponent);
