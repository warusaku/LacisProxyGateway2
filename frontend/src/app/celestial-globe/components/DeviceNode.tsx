'use client';

/**
 * DeviceNode Component
 *
 * ReactFlow カスタムノード — ネットワークデバイス表示
 * SSOT: mobes2.0 DeviceNode.tsx を LPG2 ダークテーマ向けに移植
 *
 * mobes2.0 構造準拠:
 *   - Handle target (top) + Handle source (bottom)
 *   - ステータス色でアイコン円形背景
 *   - タイプ別背景色（LPG2 dark theme 向けに調整）
 *   - インラインラベル編集（ダブルクリック→Enter/Escape）
 *   - Collapse バッジ
 *
 * Handle 方向ルール:
 *   - target (top): 親からのエッジを受信
 *   - source (bottom): 子へのエッジを送出
 *   → 親→子の一方向フローを強制（InternetNode は source のみ）
 */

import { memo, useCallback, useState, useRef, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import {
  Cloud, Globe, GitBranch, Wifi, Monitor, Shield, Box, HardDrive, Server,
  type LucideIcon,
} from 'lucide-react';
import type { DeviceNodeData, NodeType } from '../types';
import { NODE_COLORS, STATUS_COLORS } from '../constants';

// mobes2.0 準拠: デバイスタイプ→アイコンマッピング
const ICON_MAP: Record<string, LucideIcon> = {
  Cloud, Globe, GitBranch, Wifi, Monitor, Shield, Box, HardDrive, Server,
};

const ICON_FOR_TYPE: Record<NodeType, string> = {
  internet:     'Cloud',
  controller:   'Globe',
  gateway:      'Globe',
  router:       'Globe',
  switch:       'GitBranch',
  ap:           'Wifi',
  client:       'Monitor',
  wg_peer:      'Shield',
  logic_device: 'Box',
  external:     'HardDrive',
  lpg_server:   'Server',
};

const LABEL_MAX_LENGTH = 50;

function DeviceNodeComponent({ data }: { data: DeviceNodeData }) {
  const { node, selected, onCollapse, onLabelEdit } = data;
  const nodeType = node.node_type as NodeType;
  const colors = NODE_COLORS[nodeType] || NODE_COLORS.client;
  const iconName = ICON_FOR_TYPE[nodeType] || 'Monitor';
  const IconComponent = ICON_MAP[iconName] || Monitor;
  const statusColor = STATUS_COLORS[node.status] || STATUS_COLORS.unknown;

  const isOffline = node.status === 'offline' || node.status === 'inactive';

  // Inline label editing state
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState('');
  const [shaking, setShaking] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleCollapseClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    onCollapse(node.id);
  }, [node.id, onCollapse]);

  const handleDoubleClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    if (node.id === '__internet__') return;
    setEditValue(node.label);
    setEditing(true);
  }, [node.id, node.label]);

  const commitEdit = useCallback(() => {
    const trimmed = editValue.trim();
    if (!trimmed || trimmed.length > LABEL_MAX_LENGTH) {
      setShaking(true);
      setTimeout(() => setShaking(false), 400);
      return;
    }
    if (trimmed !== node.label) {
      onLabelEdit(node.id, trimmed);
    }
    setEditing(false);
  }, [editValue, node.id, node.label, onLabelEdit]);

  const cancelEdit = useCallback(() => {
    setEditing(false);
  }, []);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    e.stopPropagation();
    if (e.key === 'Enter') {
      commitEdit();
    } else if (e.key === 'Escape') {
      cancelEdit();
    }
  }, [commitEdit, cancelEdit]);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  // mobes2.0 準拠: ノード幅（インフラ=200, クライアント=160）
  const isClient = nodeType === 'client' || nodeType === 'wg_peer';
  const nodeWidth = isClient ? 160 : 200;

  return (
    <div
      className={`cg-node ${selected ? 'cg-node--selected' : ''} ${isOffline ? 'cg-node--offline' : ''}`}
      style={{
        width: nodeWidth,
        background: `linear-gradient(135deg, ${colors.bg}18, ${colors.bg}30)`,
        border: `${selected ? 2.5 : 1.5}px solid ${selected ? '#3B82F6' : isOffline ? '#4B5563' : colors.border}`,
        borderRadius: 8,
        padding: '10px 12px',
        backdropFilter: 'blur(12px)',
        position: 'relative',
        transition: 'border-color 0.2s ease, box-shadow 0.2s ease',
        boxShadow: selected ? '0 0 12px rgba(59, 130, 246, 0.3)' : '0 1px 4px rgba(0, 0, 0, 0.3)',
      }}
    >
      {/* mobes2.0 準拠: Handle target (top) — 親からのエッジ受信 */}
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: statusColor, width: 8, height: 8 }}
      />

      {/* Header: icon circle + label + type */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        {/* mobes2.0 準拠: ステータス色の円形背景 */}
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
          {editing ? (
            <input
              ref={inputRef}
              type="text"
              value={editValue}
              onChange={e => setEditValue(e.target.value.slice(0, LABEL_MAX_LENGTH))}
              onKeyDown={handleKeyDown}
              onBlur={commitEdit}
              maxLength={LABEL_MAX_LENGTH}
              className={shaking ? 'cg-shake' : ''}
              style={{
                width: '100%',
                fontSize: 12,
                fontWeight: 600,
                color: '#E5E7EB',
                background: 'rgba(255,255,255,0.1)',
                border: '1px solid #3B82F6',
                borderRadius: 4,
                padding: '1px 4px',
                outline: 'none',
                lineHeight: '16px',
              }}
            />
          ) : (
            <div
              onDoubleClick={handleDoubleClick}
              style={{
                color: isOffline ? '#6B7280' : '#E5E7EB',
                fontSize: 12,
                fontWeight: 600,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                lineHeight: '16px',
                cursor: 'text',
              }}
              title={`${node.label} (double-click to edit)`}
            >
              {node.label}
            </div>
          )}
          <div style={{ fontSize: 10, color: '#6B7280', lineHeight: '14px' }}>
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

      {/* MAC address (infrastructure nodes only) */}
      {node.mac && !isClient && (
        <div
          style={{
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
            <span style={{ fontSize: 9, padding: '1px 5px', borderRadius: 4, background: 'rgba(156,39,176,0.15)', color: '#BA68C8' }}>
              VPN
            </span>
          )}
          {node.connection_type === 'wireless' && (
            <span style={{ fontSize: 9, padding: '1px 5px', borderRadius: 4, background: 'rgba(76,175,80,0.15)', color: '#66BB6A' }}>
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

      {/* mobes2.0 準拠: Handle source (bottom) — 子へのエッジ送出 */}
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: statusColor, width: 8, height: 8 }}
      />
    </div>
  );
}

export const DeviceNode = memo(DeviceNodeComponent);
