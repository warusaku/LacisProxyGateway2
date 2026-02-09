/**
 * DeviceNode Hooks — mobes2.0 hooks.ts L8-11 準拠
 *
 * useZoom: ReactFlow の zoom レベルを取得
 */

import { useStore } from 'reactflow';

/**
 * mobes2.0 L8-11: useStore から transform[2] (zoom) を取得
 */
export function useZoom(): number {
  return useStore((s) => s.transform[2]);
}
