// CelestialGlobe v2 — TopologyEdge
// mobes2.0 edgeHelpers.ts (279行) 準拠
// edge_type別スタイル, LODラベル, アニメーション

'use client';

import React, { memo } from 'react';
import {
  getBezierPath,
  EdgeLabelRenderer,
} from 'reactflow';
import type { EdgeProps } from 'reactflow';
import { EDGE_STYLES } from '../constants';
import type { EdgeType } from '../types';

function TopologyEdgeInner({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  selected,
}: EdgeProps) {
  const edgeType = (data?.connectionType ?? 'wired') as EdgeType;
  const style = EDGE_STYLES[edgeType] ?? EDGE_STYLES['wired'];

  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
  });

  const strokeColor = selected ? '#60A5FA' : style.color;

  return (
    <>
      <path
        id={id}
        className={style.animated ? 'edge-animated' : ''}
        d={edgePath}
        fill="none"
        stroke={strokeColor}
        strokeWidth={style.strokeWidth}
        strokeDasharray={style.strokeDasharray}
        strokeLinecap="round"
      />
      {/* Edge label — LOD mid以上で表示 */}
      {data?.label && (
        <EdgeLabelRenderer>
          <div
            className="lod-mid absolute pointer-events-none"
            style={{
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
            }}
          >
            <span className="cg-glass-card px-2 py-0.5 text-[10px] text-gray-300">
              {data.label}
            </span>
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}

export const TopologyEdge = memo(TopologyEdgeInner);
