'use client';

/**
 * TopologyEdge Component
 *
 * ReactFlow カスタムエッジ - ネットワーク接続線
 * SSOT: mobes2.0 TopologyEdge.tsx を LPG2 ダークテーマ向けに移植
 *
 * mobes2.0 仕様:
 *   - getBezierPath + BaseEdge + EdgeLabelRenderer
 *   - edgeTypes = { topology: TopologyEdge } で登録
 *   - defaultEdgeOptions = { type: 'topology' }
 *   - data.connectionType でスタイル決定
 *   - selected 時は stroke=#1976d2, strokeWidth+1
 *   - animated class で wireless エッジアニメーション
 */

import { memo } from 'react';
import {
  getSmoothStepPath,
  EdgeLabelRenderer,
  type EdgeProps,
} from 'reactflow';
import { EDGE_STYLES } from '../constants';
import type { EdgeType, TopologyEdgeData } from '../types';

export const TopologyEdge = memo(({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  selected,
  markerEnd,
}: EdgeProps) => {
  const edgeData = data as TopologyEdgeData | undefined;
  const connectionType: EdgeType = edgeData?.connectionType || 'wired';
  const style = EDGE_STYLES[connectionType] || EDGE_STYLES.wired;
  const animated = edgeData?.animated ?? connectionType === 'wireless';

  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    borderRadius: 8,
  });

  // reactflow v11 の BaseEdge は className を受け付けないため、
  // 直接 <path> を描画してアニメーション用クラスを付与する
  return (
    <>
      <path
        id={id}
        className={`react-flow__edge-path ${animated ? 'react-flow__edge-path--animated' : ''}`}
        d={edgePath}
        markerEnd={markerEnd}
        style={{
          stroke: selected ? '#60A5FA' : style.color,
          strokeWidth: selected ? style.strokeWidth + 1 : style.strokeWidth,
          strokeDasharray: style.strokeDasharray,
          fill: 'none',
        }}
      />
      {/* Invisible interaction path for easier click target */}
      <path
        d={edgePath}
        style={{
          stroke: 'transparent',
          strokeWidth: 20,
          fill: 'none',
        }}
      />

      {/* Label */}
      {edgeData?.label && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: 'absolute',
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              background: 'rgba(10, 10, 10, 0.9)',
              padding: '2px 6px',
              borderRadius: 4,
              fontSize: 10,
              fontWeight: 500,
              color: style.color,
              border: `1px solid ${style.color}`,
              pointerEvents: 'all',
            }}
          >
            {edgeData.label}
          </div>
        </EdgeLabelRenderer>
      )}

      {/* Bandwidth label */}
      {edgeData?.bandwidth && !edgeData?.label && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: 'absolute',
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              background: 'rgba(10, 10, 10, 0.8)',
              padding: '1px 4px',
              borderRadius: 2,
              fontSize: 9,
              color: '#6B7280',
              pointerEvents: 'none',
            }}
          >
            {edgeData.bandwidth}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
});

TopologyEdge.displayName = 'TopologyEdge';
