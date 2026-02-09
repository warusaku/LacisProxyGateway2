// CelestialGlobe v2 — DeviceNode (最重要コンポーネント)
// mobes2.0 DeviceNodeWithLOD.tsx (1062行) 準拠
// LOD制御はCSS data-lod属性で切替。React は常に全要素レンダリング。

'use client';

import React, { memo, useCallback, useState, useRef, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import type { NodeProps } from 'reactflow';
import type { TopologyNodeV2 } from '../types';
import { NODE_COLORS } from '../constants';
import { NetworkDeviceIcon } from './icons';
import { Tooltip } from './Tooltip';
import {
  getStatusBadge,
  getStatusColor,
  getSourceBadge,
  formatMacDisplay,
  getConnectionBadge,
  isOfflineStatus,
} from './deviceNode/helpers';
import { useZoom } from './deviceNode/hooks';
import { useUIStateStore } from '../stores/useUIStateStore';

// ============================================================================
// Types (local — layoutTree builds nodes with { node: TopologyNodeV2 })
// ============================================================================

interface CgDeviceNodeData {
  node: TopologyNodeV2;
}

// ============================================================================
// Collapsed Dot Ring
// ============================================================================

function CollapsedDotRing({ count }: { count: number }) {
  const dotCount = Math.min(count, 12);
  const dots = [];
  for (let i = 0; i < dotCount; i++) {
    const angle = (i / dotCount) * 2 * Math.PI - Math.PI / 2;
    const radius = 32;
    const x = Math.cos(angle) * radius;
    const y = Math.sin(angle) * radius;
    dots.push(
      <div
        key={i}
        className="absolute w-2 h-2 rounded-full bg-emerald-500/70 animate-pulse-subtle"
        style={{
          left: `calc(50% + ${x}px - 4px)`,
          top: `calc(50% + ${y}px - 4px)`,
          animationDelay: `${(i / dotCount) * 3}s`,
        }}
      />
    );
  }
  return (
    <div className="absolute -inset-4 pointer-events-none">
      {dots}
      <div className="absolute top-0 right-0 -translate-y-1/2 translate-x-1/2 bg-slate-800/90 text-[10px] font-semibold text-white px-1.5 py-0.5 rounded-full min-w-[20px] text-center">
        {count}
      </div>
    </div>
  );
}

// ============================================================================
// Collapse Toggle Button (right side, near source handle)
// ============================================================================

function CollapseToggleButton({ nodeId, collapsed }: { nodeId: string; collapsed: boolean }) {
  const handleClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    document.dispatchEvent(new CustomEvent('cg:collapse-toggle', {
      detail: { nodeId },
      bubbles: true,
    }));
  }, [nodeId]);

  return (
    <button
      type="button"
      className="cg-collapse-toggle"
      onClick={handleClick}
      onMouseDown={(e) => e.stopPropagation()}
      title={collapsed ? 'Expand children' : 'Collapse children'}
    >
      {collapsed ? '+' : '\u2212'}
    </button>
  );
}

// ============================================================================
// DeviceNode Component
// ============================================================================

function DeviceNodeInner({ id, data, selected }: NodeProps<CgDeviceNodeData>) {
  const node = data.node;
  const zoom = useZoom();
  const draggedNodeIds = useUIStateStore(s => s.draggedNodeIds);
  const isDragging = draggedNodeIds.includes(id);

  // Label editing
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState(node.label);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isEditing]);

  const handleDoubleClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setEditValue(node.label);
    setIsEditing(true);
  }, [node.label]);

  const handleLabelSave = useCallback(() => {
    if (editValue.trim() && editValue !== node.label) {
      // Emit event for canvas to handle
      const event = new CustomEvent('cg:label-edit', {
        detail: { nodeId: id, label: editValue.trim() },
        bubbles: true,
      });
      document.dispatchEvent(event);
    }
    setIsEditing(false);
  }, [editValue, node.label, id]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleLabelSave();
    if (e.key === 'Escape') setIsEditing(false);
  }, [handleLabelSave]);

  // Derived values (null-safe — API fields may be missing for some node types)
  const nodeType = node.node_type ?? 'client';
  const colors = NODE_COLORS[nodeType] ?? NODE_COLORS['client'];
  const statusBadge = getStatusBadge(node.state_type ?? '', node.status ?? '');
  const statusColor = getStatusColor(node.state_type || node.status || 'unknown');
  const sourceBadge = getSourceBadge(node.source ?? '');
  const connBadge = getConnectionBadge(node.connection_type ?? '');
  const isLogicDevice = nodeType === 'logic_device';
  const isCollapsed = node.collapsed && (node.collapsed_child_count ?? 0) > 0;
  const isOffline = isOfflineStatus(node.state_type || node.status || 'unknown');
  const isGateway = nodeType === 'gateway';
  const hasChildren = (node.descendant_count ?? 0) > 0;
  const showCollapseToggle = hasChildren;

  // Container classes
  const containerClasses = [
    'mindmap-node relative rounded-lg shadow-lg p-3 transition-all',
    `bg-gradient-to-br ${colors.gradient}`,
    isLogicDevice ? 'border-dashed border-2 border-teal-500/50' : `border ${colors.border}`,
    selected ? 'selected animate-selection-pulse' : '',
    isDragging ? 'drag-source' : '',
    isOffline ? 'opacity-70' : '',
  ].filter(Boolean).join(' ');

  return (
    <div className={containerClasses}>
      {/* Collapsed dot ring */}
      {isCollapsed && <CollapsedDotRing count={node.collapsed_child_count} />}

      {/* Badges (top-right) */}
      <div className="absolute -top-2 right-2 flex items-center gap-1 lod-mid">
        {/* Status badge */}
        <span
          className={`inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-medium ${statusBadge.bg} ${statusBadge.text}`}
        >
          {statusBadge.label}
        </span>
        {/* Gateway badge */}
        {isGateway && (
          <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-medium bg-sky-500 text-white">
            GW
          </span>
        )}
        {/* Source badge */}
        {sourceBadge && (
          <span
            className={`inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-medium ${sourceBadge.bg} ${sourceBadge.text}`}
          >
            {sourceBadge.label}
          </span>
        )}
      </div>

      {/* Main content */}
      <div className="flex items-start gap-2">
        {/* Status dot */}
        <Tooltip content={statusBadge.label} disabled={isDragging}>
          <div
            className="w-4 h-4 rounded-full mt-1 ring-2 ring-white dark:ring-zinc-800 flex-shrink-0"
            style={{ backgroundColor: statusColor }}
          />
        </Tooltip>

        {/* Icon */}
        <div className="flex-shrink-0 mt-0.5">
          <NetworkDeviceIcon
            type={node.node_type}
            className="text-gray-900 dark:text-gray-100"
            size={20}
          />
        </div>

        {/* Label + details */}
        <div className="flex-1 min-w-0">
          {/* Label */}
          {isEditing ? (
            <input
              ref={inputRef}
              type="text"
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onBlur={handleLabelSave}
              onKeyDown={handleKeyDown}
              className="w-full bg-white/90 dark:bg-zinc-800/90 rounded px-1 py-0.5 text-sm font-semibold text-gray-900 dark:text-gray-100 outline-none ring-2 ring-blue-400"
            />
          ) : (
            <div
              className="font-semibold text-sm text-gray-900 dark:text-gray-100 truncate cursor-text"
              onDoubleClick={handleDoubleClick}
              title={node.label}
            >
              {node.label}
            </div>
          )}

          {/* IP address */}
          {node.ip && (
            <div className="text-xs text-gray-500 dark:text-gray-400 truncate">
              {node.ip}
            </div>
          )}

          {/* MAC — lod-high */}
          {node.mac && (
            <div className="text-xs text-gray-500 dark:text-gray-400 truncate lod-high">
              {formatMacDisplay(node.mac)}
            </div>
          )}

          {/* LacisID — lod-high */}
          {node.lacis_id && (
            <div className="text-xs text-gray-500 dark:text-gray-400 truncate lod-high">
              LacisID {node.lacis_id}
            </div>
          )}

          {/* Connection type — lod-full */}
          {connBadge && (
            <div className="lod-full mt-1">
              <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-medium text-white ${connBadge.color}`}>
                {connBadge.label}
              </span>
            </div>
          )}
        </div>
      </div>

      {/* Descendant count (if collapsed) */}
      {isCollapsed && (
        <div className="mt-1 text-[10px] text-gray-400 text-center lod-mid">
          {node.descendant_count} device{node.descendant_count !== 1 ? 's' : ''} hidden
        </div>
      )}

      {/* Handles */}
      <Handle
        type="target"
        position={Position.Left}
        className="!w-3 !h-3 !bg-gray-400 dark:!bg-gray-600 !border-2 !border-white dark:!border-zinc-800"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!w-3 !h-3 !bg-gray-400 dark:!bg-gray-600 !border-2 !border-white dark:!border-zinc-800"
      />

      {/* Collapse toggle button (right side, near source handle) */}
      {showCollapseToggle && (
        <CollapseToggleButton nodeId={id} collapsed={!!node.collapsed} />
      )}
    </div>
  );
}

export const DeviceNode = memo(DeviceNodeInner);
