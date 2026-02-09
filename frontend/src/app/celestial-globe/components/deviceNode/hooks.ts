// CelestialGlobe v2 — DeviceNode Hooks
// mobes2.0 hooks.ts (489行) から LPG2 向けに移植

'use client';

import { useMemo } from 'react';
import { useStore } from 'reactflow';

// ============================================================================
// useZoom — ReactFlow内部ストアからズームレベルを取得
// ============================================================================

export function useZoom(): number {
  return useStore((s) => s.transform[2]);
}

// ============================================================================
// useMindmapHandlePositions — ノードの source/target ハンドル位置を判定
// ============================================================================

export function useMindmapHandlePositions(
  nodeId: string,
  parentId?: string,
): { sourcePos: 'right' | 'left'; targetPos: 'right' | 'left' } {
  // LPG2ではLR(左→右)レイアウトを標準とするため、
  // source=right (子への接続), target=left (親からの接続) が基本
  return useMemo(() => {
    return {
      sourcePos: 'right' as const,
      targetPos: 'left' as const,
    };
  }, [nodeId, parentId]);
}
