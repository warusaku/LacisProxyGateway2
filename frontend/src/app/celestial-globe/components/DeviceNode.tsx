/**
 * DeviceNode Component — mobes2.0 DeviceNodeWithLOD.tsx 忠実再現
 *
 * Container: mindmap-node p-3 rounded-lg shadow-lg
 * LOD: CSS hidden で zoom に応じてテキスト切替
 *   - isMinimalZoom (zoom < 0.4): テキスト非表示
 *   - isBasicZoom (0.4 ≤ zoom < 0.8): アイコン非表示
 *   - isFullZoom (zoom ≥ 0.8): 全詳細表示
 *
 * LPG2で省略: VLAN, Site, aranea, PoE, D&D, bucket, useReportNodeDimensions
 */

'use client';

import { memo, useState, useCallback } from 'react';
import { Handle, Position } from 'reactflow';
import type { NodeProps } from 'reactflow';
import type { DeviceNodeData, TopologyNodeV2 } from '../types';
import { NetworkDeviceIcon } from './icons';
import { Tooltip } from './Tooltip';
import { useZoom } from './deviceNode/hooks';
import {
  resolveComputedStatus,
  getStatusColor,
  getStatusRingColor,
  isOfflineStatus,
  isGateway,
  getSourceBadge,
  STATUS_BADGE_MAP,
  type ComputedStatus,
} from './deviceNode/helpers';

// ============================================================================
// DeviceNode
// ============================================================================

export const DeviceNode = memo(({ data, selected }: NodeProps<DeviceNodeData>) => {
  const { node, onCollapse, onLabelEdit } = data;
  const zoom = useZoom();

  // LOD levels — mobes2.0 L58-62
  const isMinimalZoom = zoom < 0.4;
  const isBasicZoom = zoom >= 0.4 && zoom < 0.8;
  const isFullZoom = zoom >= 0.8;

  const computedStatus = resolveComputedStatus(node);
  const statusColor = getStatusColor(computedStatus);
  const statusRing = getStatusRingColor(computedStatus);
  const offline = isOfflineStatus(computedStatus);
  const gatewayNode = isGateway(node);
  const statusBadge = STATUS_BADGE_MAP[computedStatus];
  const sourceBadge = getSourceBadge(node.source);
  const isLogicDevice = node.node_type === 'logic_device';

  // Label editing state
  const [editing, setEditing] = useState(false);
  const [editLabel, setEditLabel] = useState(node.label);

  const handleDoubleClick = useCallback(() => {
    if (!isFullZoom) return;
    setEditLabel(node.label);
    setEditing(true);
  }, [isFullZoom, node.label]);

  const handleEditSubmit = useCallback(() => {
    if (editLabel.trim() && editLabel.trim() !== node.label) {
      onLabelEdit(node.id, editLabel.trim());
    }
    setEditing(false);
  }, [editLabel, node.label, node.id, onLabelEdit]);

  const handleEditKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleEditSubmit();
    if (e.key === 'Escape') setEditing(false);
  }, [handleEditSubmit]);

  // Collapse handler
  const handleCollapseClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    onCollapse(node.id);
  }, [onCollapse, node.id]);

  // ============================================================================
  // Container classes — mobes2.0 L350-398
  // ============================================================================

  const containerClasses = [
    'mindmap-node group relative rounded-lg shadow-lg p-3 transition-all',
    // Selection
    selected ? 'mindmap-selection-pulse-strong ring-2 ring-primary-400/60' : '',
    // Opacity for offline
    offline ? 'opacity-70' : '',
    // LogicDevice: dashed border + teal gradient
    isLogicDevice
      ? 'border-2 border-dashed border-teal-500/50 bg-gradient-to-br from-teal-950/40 via-dark-900 to-dark-900'
      : 'border border-dark-700 bg-dark-900/95',
  ].filter(Boolean).join(' ');

  return (
    <>
      {/* Target handle (left) */}
      <Handle
        type="target"
        position={Position.Left}
        className="!w-3 !h-3 !bg-gray-400 dark:!bg-gray-600 !border-2 !border-white dark:!border-dark-100"
      />

      <div className={containerClasses} style={{ minWidth: isMinimalZoom ? 40 : 160 }}>
        {/* Badge area — mobes2.0 L399-487 */}
        <div className="absolute -top-2 right-2 flex items-center gap-1">
          {/* Status badge (STATIC/MANUAL) */}
          {statusBadge && !isMinimalZoom && (
            <span
              className={`
                px-1.5 py-0.5 text-[9px] font-bold rounded border
                ${statusBadge.bgClass} ${statusBadge.textClass} ${statusBadge.borderClass}
              `}
            >
              {statusBadge.label}
            </span>
          )}
          {/* Gateway badge */}
          {gatewayNode && !isMinimalZoom && (
            <span className="px-1.5 py-0.5 text-[9px] font-bold rounded bg-sky-500/20 text-sky-400 border border-sky-500/30">
              GW
            </span>
          )}
        </div>

        {/* Minimal zoom: just a colored dot */}
        {isMinimalZoom && (
          <div className="flex items-center justify-center p-1">
            <div className={`w-4 h-4 rounded-full ${statusColor} ring-2 ring-white dark:ring-dark-200`} />
          </div>
        )}

        {/* Basic + Full zoom: main content — mobes2.0 L488-631 */}
        {!isMinimalZoom && (
          <div className="flex items-start gap-2">
            {/* Status dot */}
            <div className={`w-4 h-4 rounded-full ${statusColor} ring-2 ring-white dark:ring-dark-200 mt-1 shrink-0`} />

            <div className="flex-1 min-w-0">
              {/* Icon + Label row */}
              <div className="flex items-center gap-1.5">
                {isFullZoom && (
                  <NetworkDeviceIcon
                    nodeType={node.node_type}
                    className="w-4 h-4 text-dark-400 shrink-0"
                  />
                )}
                {editing ? (
                  <input
                    type="text"
                    value={editLabel}
                    onChange={(e) => setEditLabel(e.target.value)}
                    onBlur={handleEditSubmit}
                    onKeyDown={handleEditKeyDown}
                    autoFocus
                    className="text-sm font-semibold bg-transparent border-b border-primary-400 text-gray-100 outline-none w-full"
                  />
                ) : (
                  <span
                    className="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate cursor-default"
                    onDoubleClick={handleDoubleClick}
                    title={node.label}
                  >
                    {node.label}
                  </span>
                )}
              </div>

              {/* IP + source badge */}
              {isFullZoom && node.ip && (
                <div className="flex items-center gap-1.5 mt-0.5">
                  <span className="text-xs text-gray-500 dark:text-dark-400">{node.ip}</span>
                  {sourceBadge && (
                    <span className={`px-1 py-0 text-[9px] rounded border ${sourceBadge.colorClass}`}>
                      {sourceBadge.label}
                    </span>
                  )}
                </div>
              )}

              {/* MAC */}
              {isFullZoom && node.mac && (
                <div className="text-xs text-gray-500 dark:text-dark-500 mt-0.5 truncate">
                  {node.mac}
                </div>
              )}

              {/* LacisID */}
              {isFullZoom && node.lacis_id && (
                <div className="text-xs text-gray-500 dark:text-dark-500 mt-0.5 truncate">
                  LacisID {node.lacis_id}
                </div>
              )}

              {/* Basic zoom: just IP (no MAC/LacisID) */}
              {isBasicZoom && node.ip && (
                <div className="text-xs text-gray-500 dark:text-dark-400 mt-0.5">{node.ip}</div>
              )}
            </div>
          </div>
        )}

        {/* Collapsed dot ring — mobes2.0 L633-669 */}
        {node.collapsed && node.collapsed_child_count > 0 && !isMinimalZoom && (
          <div className="absolute -inset-4 pointer-events-none">
            {/* Decorative dots */}
            <div className="absolute top-0 right-0 flex items-center gap-0.5 animate-pulse-subtle">
              <div className="w-2 h-2 rounded-full bg-emerald-500/70" />
              <div className="w-2 h-2 rounded-full bg-emerald-500/50" />
              <div className="w-2 h-2 rounded-full bg-emerald-500/30" />
            </div>
            {/* Count badge */}
            <div className="absolute -bottom-1 right-0 pointer-events-auto">
              <button
                onClick={handleCollapseClick}
                className="
                  bg-slate-800/90 text-[10px] font-semibold text-white
                  px-1.5 py-0.5 rounded-full border border-dark-600
                  hover:bg-slate-700 transition-colors cursor-pointer
                "
                title={`${node.collapsed_child_count} collapsed children (click to expand)`}
              >
                +{node.collapsed_child_count}
              </button>
            </div>
          </div>
        )}

        {/* Expand/collapse toggle for non-collapsed nodes with children */}
        {!node.collapsed && node.descendant_count > 0 && !isMinimalZoom && (
          <button
            onClick={handleCollapseClick}
            className="
              absolute -right-2 top-1/2 -translate-y-1/2
              w-5 h-5 rounded-full bg-dark-800 border border-dark-600
              text-[10px] text-dark-400 hover:text-white hover:bg-dark-700
              flex items-center justify-center cursor-pointer transition-colors
              pointer-events-auto
            "
            title={`Collapse (${node.descendant_count} descendants)`}
          >
            -
          </button>
        )}

        {/* Tooltip */}
        <Tooltip zoom={zoom}>
          <div className="space-y-0.5">
            <div className="font-semibold">{node.label}</div>
            <div className="text-dark-400">{node.node_type} / {node.source}</div>
            {node.ip && <div>IP: {node.ip}</div>}
            {node.mac && <div>MAC: {node.mac}</div>}
          </div>
        </Tooltip>
      </div>

      {/* Source handle (right) */}
      <Handle
        type="source"
        position={Position.Right}
        className="!w-3 !h-3 !bg-gray-400 dark:!bg-gray-600 !border-2 !border-white dark:!border-dark-100"
      />
    </>
  );
});

DeviceNode.displayName = 'DeviceNode';
