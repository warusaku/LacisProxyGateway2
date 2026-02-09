/**
 * DeviceNode hooks
 * SSoT: mobes2.0 deviceNodeWithLOD/hooks.ts を LPG2 向けに適合移植
 *
 * 移植フック:
 *   - useZoom: ReactFlow store からズーム値取得
 *   - useNodeTooltipContent: ツールチップJSX生成
 *   - useMindmapHandlePositions: Handle L/R 決定
 */

import { useMemo } from 'react';
import { Position, useStore } from 'reactflow';
import type { TopologyNodeV2 } from '../../types';
import { buildTooltipLines } from './helpers';

// ============================================================================
// useZoom (mobes2.0 完全移植)
// ============================================================================

export const useZoom = (): number => {
  return useStore((s) => s.transform[2]);
};

// ============================================================================
// useNodeTooltipContent (LPG2 適合版)
// ============================================================================

export const useNodeTooltipContent = (node: TopologyNodeV2): string[] => {
  return useMemo(() => buildTooltipLines({
    label: node.label,
    ip: node.ip,
    mac: node.mac,
    source: node.source,
    stateType: node.state_type,
    nodeType: node.node_type,
    lacisId: node.lacis_id,
    descendantCount: node.descendant_count,
    isGateway: node.node_type === 'gateway',
  }), [node.label, node.ip, node.mac, node.source, node.state_type, node.node_type, node.lacis_id, node.descendant_count]);
};

// ============================================================================
// useMindmapHandlePositions (mobes2.0 準拠 → LPG2簡略版)
// ============================================================================
// LPG2はdraggable:false, 常に左→右ツリーなのでシンプルに
// 将来 orientation対応時にmobes2.0完全版に拡張可能

export const useMindmapHandlePositions = () => {
  return {
    targetPosition: Position.Left,
    sourcePosition: Position.Right,
  } as const;
};
