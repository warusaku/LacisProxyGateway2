'use client';

/**
 * DeviceNode Component — mobes2.0 DeviceNodeWithLOD 完全移植
 *
 * SSoT: mobes2.0 DeviceNodeWithLOD.tsx (782行) の視覚構造・Tailwindスタイリング・UXパターンを
 *       LPG2 TopologyNodeV2 データ構造に適合して完全移植
 *
 * mobes2.0 構造:
 *   DeviceNode (外側) — useZoom()
 *   └── FullDetailNode (内側, zoom prop)
 *       ├── LOD CSS切替: isMinimalZoom(<0.4), isBasicZoom(0.4-0.8)
 *       ├── LogicDevice分岐 → 特別描画 (破線+teal)
 *       └── 通常デバイスノード
 *           ├── バッジエリア (右上 absolute)
 *           ├── メインコンテンツ (ステータスドット + アイコン + ラベル + IP + MAC + LacisID)
 *           ├── 折りたたみドットリング (collapsed時)
 *           └── Handle (Left target + Right source)
 */

import React, { memo, useState, useMemo } from 'react';
import { Handle, Position, type NodeProps } from 'reactflow';
import Tooltip from './Tooltip';
import { NetworkDeviceIcon } from './icons';
import type { DeviceNodeData } from '../types';
import {
  resolveComputedStatus,
  STATUS_BADGE_INFO,
  getStatusColor,
  isOfflineStatus,
} from './deviceNode/helpers';
import { useZoom, useNodeTooltipContent, useMindmapHandlePositions } from './deviceNode/hooks';

const LABEL_MAX_LENGTH = 50;

// ============================================================================
// FullDetailNode — mobes2.0 準拠のフル描画
// ============================================================================

const FullDetailNode: React.FC<NodeProps<DeviceNodeData> & { zoom?: number }> = ({ id, data, zoom = 1.0 }) => {
  const { node, onCollapse, onLabelEdit } = data;

  // LOD CSS切替 (mobes2.0 準拠)
  const isMinimalZoom = zoom < 0.4;
  const isBasicZoom = zoom >= 0.4 && zoom < 0.8;

  const [isEditing, setIsEditing] = useState(false);
  const isManualEntry = node.source === 'manual';
  const isGateway = node.node_type === 'gateway';
  const isLogicDevice = node.node_type === 'logic_device';
  const isCollapsed = node.collapsed;
  const collapsedChildCount = node.collapsed_child_count ?? 0;

  const computedStatus = resolveComputedStatus(node);
  const statusBadgeMeta = STATUS_BADGE_INFO[computedStatus];
  const statusColor = getStatusColor(computedStatus, true);
  const isOffline = isOfflineStatus(computedStatus);
  const isManualOverride = computedStatus === 'manual_online';
  const manualAccent = isManualEntry || isManualOverride;

  const { targetPosition, sourcePosition } = useMindmapHandlePositions();
  const tooltipLines = useNodeTooltipContent(node);

  const tooltipContent = useMemo(() => (
    <div className="flex w-full flex-col gap-0.5 text-left">
      {tooltipLines.map((line, i) => (
        <div key={i} className={`text-[10px] ${i === 0 ? 'text-[11px] font-semibold text-white' : 'text-gray-200'}`}>
          {line}
        </div>
      ))}
    </div>
  ), [tooltipLines]);

  const handleLabelChange = (newLabel: string) => {
    const trimmed = newLabel.trim();
    if (trimmed && trimmed.length <= LABEL_MAX_LENGTH && trimmed !== node.label) {
      onLabelEdit(node.id, trimmed);
    }
    setIsEditing(false);
  };

  // ============================================================================
  // Selection ring (mobes2.0 準拠)
  // ============================================================================
  const selectionRing = data.selected
    ? 'mindmap-selection-pulse-strong border-2 border-primary-500 ring-2 ring-primary-200 dark:ring-primary-900'
    : manualAccent
      ? 'border-2 border-accent-400 ring-1 ring-accent-200 dark:border-accent-500 dark:ring-accent-900'
      : isGateway
        ? 'border-2 border-sky-500 ring-1 ring-sky-300/70 shadow-[0_0_16px_rgba(59,130,246,0.35)]'
        : 'border border-gray-300 dark:border-dark-300';

  // ============================================================================
  // Background (mobes2.0 準拠)
  // ============================================================================
  const backgroundClass = manualAccent
    ? 'bg-gradient-to-br from-accent-50 via-white to-accent-50/30 dark:from-accent-900/20 dark:via-dark-200 dark:to-accent-900/10'
    : 'bg-white dark:bg-dark-200';

  const offlineClass = isOffline ? 'opacity-60 grayscale' : '';

  // ============================================================================
  // LogicDevice 特別描画 (mobes2.0 準拠: 破線+teal)
  // ============================================================================
  if (isLogicDevice) {
    return (
      <Tooltip label={tooltipContent} className="block" zoom={zoom}>
        <div
          className={`
            mindmap-node relative rounded-xl border-2 border-dashed
            bg-gradient-to-br from-teal-50/80 via-white to-teal-50/50
            dark:from-teal-900/30 dark:via-dark-200 dark:to-teal-900/20
            px-5 py-4 min-w-[200px] transition-all
            ${selectionRing}
            border-teal-400/60 dark:border-teal-500/50
            ${offlineClass}
          `}
        >
          {/* Status Badge */}
          <div className={`absolute -top-2 right-3 flex items-center gap-1 ${isMinimalZoom ? 'hidden' : ''}`}>
            {statusBadgeMeta && (
              <span className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-semibold shadow-sm ${statusBadgeMeta.className}`}>
                {statusBadgeMeta.label}
              </span>
            )}
          </div>

          {/* Zone Header */}
          <div className="flex items-center gap-3">
            <div
              className="w-5 h-5 rounded-md flex-shrink-0 ring-2 ring-white dark:ring-dark-200 flex items-center justify-center"
              style={{ backgroundColor: statusColor }}
            >
              <NetworkDeviceIcon type="logic_device" className="w-3 h-3 text-white" />
            </div>
            <div className={`flex-grow ${isMinimalZoom ? 'hidden' : ''}`}>
              {isEditing ? (
                <input
                  ref={(el) => el?.focus()}
                  type="text"
                  defaultValue={node.label}
                  className="w-full border-0 bg-transparent p-0 text-base font-bold text-teal-700 dark:text-teal-300 outline-none focus:ring-0"
                  onBlur={(e) => handleLabelChange(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleLabelChange(e.currentTarget.value);
                    if (e.key === 'Escape') setIsEditing(false);
                  }}
                />
              ) : (
                <div
                  className="text-base font-bold text-teal-700 dark:text-teal-300 cursor-text"
                  onDoubleClick={() => setIsEditing(true)}
                >
                  {node.label || 'ゾーン'}
                </div>
              )}
              <div className="text-[11px] text-teal-600/70 dark:text-teal-400/70 mt-0.5">
                物理コンテナ
              </div>
            </div>
          </div>

          {/* Handles */}
          <Handle
            id="target-left"
            type="target"
            position={targetPosition}
            className="!bg-teal-500 !border-teal-300 dark:!border-teal-600"
          />
          <Handle
            id="source-right"
            type="source"
            position={sourcePosition}
            className="!bg-teal-500 !border-teal-300 dark:!border-teal-600"
          />
        </div>
      </Tooltip>
    );
  }

  // ============================================================================
  // 通常デバイスノード (mobes2.0 DeviceNodeWithLOD 準拠)
  // ============================================================================
  return (
    <Tooltip label={tooltipContent} className="block" zoom={zoom}>
      <div
        className={`
          mindmap-node relative rounded-lg shadow-lg p-3 transition-all
          ${selectionRing}
          ${backgroundClass}
          ${offlineClass}
        `}
        data-device-type={node.node_type}
        data-status={node.status}
      >
        {/* ================================================================
            Badge area (右上 absolute) — mobes2.0 準拠
            ================================================================ */}
        <div className={`absolute -top-2 right-2 flex items-center gap-1 ${isMinimalZoom ? 'hidden' : ''}`}>
          {/* device_type バッジ (araneaDevice等) */}
          {node.device_type && (
            <span className="inline-flex items-center gap-1 rounded-full border border-rose-300/60 bg-rose-500/10 px-2 py-0.5 text-[10px] font-semibold text-rose-600 dark:border-rose-300/40 dark:text-rose-200 shadow-sm">
              {node.device_type}
            </span>
          )}
          {/* ステータスバッジ (MANUAL/STATIC等) */}
          {statusBadgeMeta && (
            <span className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-semibold shadow-sm ${statusBadgeMeta.className}`}>
              {statusBadgeMeta.label}
            </span>
          )}
          {/* GWバッジ */}
          {isGateway && (
            <span
              className="inline-flex items-center gap-1 rounded-full bg-sky-500 px-2 py-0.5 text-[10px] font-semibold text-white shadow-md"
              title="インターネットゲートウェイ"
            >
              GW
            </span>
          )}
          {/* ソースバッジ (Manual entry dot) */}
          {isManualEntry && !statusBadgeMeta && (
            <span
              className="h-3 w-3 rounded-full bg-accent-500 dark:bg-accent-400"
              title="Manual entry"
            />
          )}
        </div>

        {/* ================================================================
            Main content — mobes2.0 準拠
            ================================================================ */}
        <div className="flex items-start">
          {/* ステータスドット (4px丸, ring-2) — 常に表示 */}
          <div
            className="w-4 h-4 rounded-full mr-3 mt-1 flex-shrink-0 ring-2 ring-white dark:ring-dark-200"
            style={{ backgroundColor: statusColor }}
          />

          {/* テキスト/アイコン — ズームレベルで表示/非表示 */}
          <div className={`flex-grow ${isMinimalZoom ? 'hidden' : ''}`}>
            <div className="flex items-center gap-2">
              {/* NetworkDeviceIcon — isBasicZoom時は非表示 */}
              <div className={`text-gray-700 dark:text-gray-200 ${isBasicZoom ? 'hidden' : ''}`}>
                <NetworkDeviceIcon type={node.node_type} className="w-5 h-5" />
              </div>

              {/* Label + IP + Source Badge */}
              <div className="flex-grow" onDoubleClick={() => setIsEditing(true)}>
                {isEditing ? (
                  <input
                    autoFocus
                    defaultValue={node.label}
                    onBlur={(e) => handleLabelChange(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') handleLabelChange(e.currentTarget.value);
                      if (e.key === 'Escape') setIsEditing(false);
                    }}
                    className="w-full px-1 py-0.5 bg-white dark:bg-dark-100 border border-gray-300 dark:border-dark-400 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                  />
                ) : (
                  <div>
                    <div className="font-semibold text-gray-900 dark:text-gray-100">
                      {node.label}
                    </div>
                    {/* IP + ソースバッジ */}
                    <div className="flex items-center gap-2 mt-1">
                      {node.ip && (
                        <span className="text-xs text-gray-500 dark:text-gray-400">
                          {node.ip}
                        </span>
                      )}
                      {isManualEntry && !statusBadgeMeta && (
                        <span className="text-xs bg-accent-100 dark:bg-accent-900/30 text-accent-700 dark:text-accent-300 px-2 py-0.5 rounded-full font-medium">
                          Manual
                        </span>
                      )}
                      {node.source === 'omada' && (
                        <span className="text-xs bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 px-2 py-0.5 rounded-full font-medium">
                          Omada
                        </span>
                      )}
                    </div>
                    {/* 🆔 MAC + LacisID + 接続タイプ */}
                    <div className={`mt-2 space-y-1 ${isBasicZoom ? 'hidden' : ''}`}>
                      {node.mac && (
                        <div className="text-xs text-gray-500 dark:text-gray-400">
                          🆔 {node.mac}
                        </div>
                      )}
                      {node.lacis_id && (
                        <div className="text-xs text-gray-500 dark:text-gray-400">
                          LacisID {node.lacis_id}
                        </div>
                      )}
                      {/* VPN/WiFi バッジ */}
                      {(node.connection_type === 'vpn' || node.connection_type === 'wireless') && (
                        <div className="flex flex-wrap gap-1 pt-1">
                          {node.connection_type === 'vpn' && (
                            <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full border border-purple-400/60 text-purple-600 dark:text-purple-300 dark:border-purple-500/40">
                              VPN
                            </span>
                          )}
                          {node.connection_type === 'wireless' && (
                            <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full border border-green-400/60 text-green-600 dark:text-green-300 dark:border-green-500/40">
                              WiFi
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* ================================================================
            折りたたみドットリング (mobes2.0 Phase 2 準拠)
            ================================================================ */}
        {isCollapsed && collapsedChildCount > 0 && (
          <div className="absolute -inset-4 pointer-events-none">
            <div className="absolute inset-0 animate-pulse-subtle">
              {Array.from({ length: Math.min(collapsedChildCount, 12) }).map((_, index) => {
                const angle = (360 / Math.min(collapsedChildCount, 12)) * index - 90;
                const radians = (angle * Math.PI) / 180;
                const radius = 50;
                const x = 50 + radius * Math.cos(radians);
                const y = 50 + radius * Math.sin(radians);
                return (
                  <div
                    key={index}
                    className="absolute w-2 h-2 rounded-full bg-emerald-500/70 dark:bg-emerald-400/70"
                    style={{
                      left: `${x}%`,
                      top: `${y}%`,
                      transform: 'translate(-50%, -50%)',
                    }}
                  />
                );
              })}
            </div>
            {/* 折りたたみカウントバッジ */}
            <div className="absolute -bottom-2 left-1/2 transform -translate-x-1/2 flex items-center gap-1">
              <span className="inline-flex items-center gap-1 rounded-full bg-slate-800/90 px-2 py-0.5 text-[10px] font-semibold text-white shadow-md">
                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
                {collapsedChildCount}
              </span>
            </div>
          </div>
        )}

        {/* Collapse toggle button */}
        {node.descendant_count > 0 && !isCollapsed && (
          <button
            onClick={(e) => { e.stopPropagation(); onCollapse(node.id); }}
            className="absolute -right-2 -bottom-2 bg-none border-none p-0 cursor-pointer"
          >
            <span className="cg-collapse-badge">
              {node.descendant_count}
            </span>
          </button>
        )}
        {isCollapsed && (
          <button
            onClick={(e) => { e.stopPropagation(); onCollapse(node.id); }}
            className="absolute -right-2 -bottom-2 bg-none border-none p-0 cursor-pointer z-10"
          >
            <span className="cg-collapse-badge">
              +{collapsedChildCount}
            </span>
          </button>
        )}

        {/* ================================================================
            Handles (mobes2.0 準拠: target=left, source=right)
            ================================================================ */}
        <Handle
          id="target-left"
          type="target"
          position={targetPosition}
          className="!w-3 !h-3 !bg-gray-400 dark:!bg-gray-600 !border-2 !border-white dark:!border-dark-100"
        />
        <Handle
          id="source-right"
          type="source"
          position={sourcePosition}
          className="!w-3 !h-3 !bg-gray-400 dark:!bg-gray-600 !border-2 !border-white dark:!border-dark-100"
        />
      </div>
    </Tooltip>
  );
};

// ============================================================================
// DeviceNodeWithLOD wrapper (mobes2.0 準拠: zoom取得 → FullDetailNodeに渡す)
// ============================================================================

const DeviceNodeComponent: React.FC<NodeProps<DeviceNodeData>> = (props) => {
  const zoom = useZoom();
  return <FullDetailNode {...props} zoom={zoom} />;
};

// ============================================================================
// React.memo + areDeviceNodePropsEqual (mobes2.0 準拠)
// ============================================================================

const areDeviceNodePropsEqual = (
  prev: NodeProps<DeviceNodeData>,
  next: NodeProps<DeviceNodeData>,
): boolean => (
  prev.id === next.id
  && prev.type === next.type
  && prev.selected === next.selected
  && prev.xPos === next.xPos
  && prev.yPos === next.yPos
  && isDeviceDataStable(prev.data, next.data)
);

const isDeviceDataStable = (prev?: DeviceNodeData, next?: DeviceNodeData): boolean => {
  if (prev === next) return true;
  if (!prev || !next) return false;
  return (
    prev.node.status === next.node.status
    && prev.node.state_type === next.node.state_type
    && prev.node.label === next.node.label
    && prev.selected === next.selected
    && prev.node.collapsed === next.node.collapsed
    && prev.node.collapsed_child_count === next.node.collapsed_child_count
  );
};

export const DeviceNode = memo(DeviceNodeComponent, areDeviceNodePropsEqual);
