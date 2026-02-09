'use client';

/**
 * InternetNode Component
 *
 * ReactFlow カスタムノード — インターネット接続点
 * SSOT: mobes2.0 InternetNode.tsx を LPG2 ダークテーマ向けに移植
 *
 * 重要な設計制約:
 *   - Handle type="source" (Position.Bottom) のみを持つ
 *   - Handle type="target" は存在しない
 *   → エッジの逆進（子ノード → InternetNode）を ReactFlow の構造レベルで禁止
 *   → Internet は常にトポロジーツリーのルートであり、親を持たない
 *
 * mobes2.0 仕様:
 *   - Cloud アイコン、中央配置
 *   - 背景: #E3F2FD（mobes2.0） → rgba(37, 99, 235, 0.15)（LPG2 dark）
 *   - ボーダー: #2196F3（mobes2.0） → #3B82F6（LPG2 blue-500）
 *   - Handle サイズ: 10x10（mobes2.0 準拠、DeviceNode の 8x8 より大きい）
 */

import { memo } from 'react';
import { Handle, Position, type NodeProps } from 'reactflow';
import { Cloud } from 'lucide-react';
import type { InternetNodeData } from '../types';

function InternetNodeComponent({ data, selected }: NodeProps<InternetNodeData>) {
  return (
    <div
      style={{
        padding: 16,
        borderRadius: 8,
        backgroundColor: 'rgba(37, 99, 235, 0.15)',
        border: `2px solid ${selected ? '#60A5FA' : '#3B82F6'}`,
        minWidth: 120,
        textAlign: 'center',
        boxShadow: selected ? '0 0 16px rgba(59, 130, 246, 0.4)' : '0 2px 8px rgba(0, 0, 0, 0.3)',
        transition: 'all 0.2s ease',
        cursor: 'pointer',
      }}
    >
      {/* Cloud Icon */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', marginBottom: 8 }}>
        <Cloud size={40} style={{ color: '#3B82F6' }} />
      </div>

      {/* Label */}
      <div style={{ fontWeight: 600, fontSize: 14, color: '#E5E7EB' }}>
        {data.label || 'Internet'}
      </div>

      {/* IP */}
      {data.ip && (
        <div style={{ fontFamily: 'monospace', fontSize: 12, color: '#9CA3AF', marginTop: 4 }}>
          {data.ip}
        </div>
      )}

      {/* Source handle ONLY (right) — target handle は意図的に無い */}
      <Handle
        type="source"
        position={Position.Right}
        style={{ background: '#3B82F6', width: 10, height: 10 }}
      />
    </div>
  );
}

export const InternetNode = memo(InternetNodeComponent);
