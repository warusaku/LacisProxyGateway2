/**
 * DeviceNode Helpers — mobes2.0 helpers.tsx (L135-312) LPG2向け移植
 *
 * resolveComputedStatus: state_type → ComputedStatus マッピング
 * STATUS_BADGE_INFO: Tailwind クラス付きバッジ定義
 * getStatusColor: ドットカラー
 * isOfflineStatus: オフライン判定
 */

import type { TopologyNodeV2 } from '../../types';

// ============================================================================
// ComputedStatus — mobes2.0 L20-30
// ============================================================================

export type ComputedStatus =
  | 'online'
  | 'offline'
  | 'staticOnline'
  | 'staticOffline'
  | 'manual'
  | 'unknown';

/**
 * state_type (backend) → ComputedStatus (frontend display)
 * mobes2.0 resolveComputedStatus 準拠
 */
export function resolveComputedStatus(node: TopologyNodeV2): ComputedStatus {
  const st = node.state_type;
  switch (st) {
    case 'online':
    case 'active':
      return 'online';
    case 'offline':
    case 'inactive':
      return 'offline';
    case 'StaticOnline':
      return 'staticOnline';
    case 'StaticOffline':
      return 'staticOffline';
    case 'manual':
      return 'manual';
    default:
      return 'unknown';
  }
}

// ============================================================================
// Status badge info — mobes2.0 L135-160 完全一致
// ============================================================================

export interface StatusBadgeInfo {
  label: string;
  bgClass: string;
  textClass: string;
  borderClass: string;
}

export const STATUS_BADGE_MAP: Record<ComputedStatus, StatusBadgeInfo | null> = {
  online: null, // no badge for normal online
  offline: null, // no badge for normal offline
  staticOnline: {
    label: 'STATIC',
    bgClass: 'bg-sky-500/20',
    textClass: 'text-sky-400',
    borderClass: 'border-sky-500/30',
  },
  staticOffline: {
    label: 'STATIC',
    bgClass: 'bg-amber-500/20',
    textClass: 'text-amber-400',
    borderClass: 'border-amber-500/30',
  },
  manual: {
    label: 'MANUAL',
    bgClass: 'bg-gray-500/20',
    textClass: 'text-gray-400',
    borderClass: 'border-gray-500/30',
  },
  unknown: null,
};

// ============================================================================
// Status dot color — mobes2.0 L172-192 完全一致
// ============================================================================

export function getStatusColor(status: ComputedStatus): string {
  switch (status) {
    case 'online':
    case 'staticOnline':
      return 'bg-emerald-500';
    case 'offline':
      return 'bg-red-500';
    case 'staticOffline':
      return 'bg-amber-500';
    case 'manual':
      return 'bg-gray-400';
    case 'unknown':
    default:
      return 'bg-gray-500';
  }
}

export function getStatusRingColor(status: ComputedStatus): string {
  switch (status) {
    case 'online':
    case 'staticOnline':
      return 'ring-emerald-500/30';
    case 'offline':
      return 'ring-red-500/30';
    case 'staticOffline':
      return 'ring-amber-500/30';
    case 'manual':
      return 'ring-gray-400/30';
    case 'unknown':
    default:
      return 'ring-gray-500/30';
  }
}

// ============================================================================
// isOfflineStatus — mobes2.0 L168-170 完全一致
// ============================================================================

export function isOfflineStatus(status: ComputedStatus): boolean {
  return status === 'offline' || status === 'staticOffline';
}

// ============================================================================
// Gateway badge check
// ============================================================================

export function isGateway(node: TopologyNodeV2): boolean {
  return node.node_type === 'gateway';
}

// ============================================================================
// Source display label
// ============================================================================

export function getSourceBadge(source: string): { label: string; colorClass: string } | null {
  switch (source) {
    case 'omada':
      return { label: 'Omada', colorClass: 'text-blue-400 bg-blue-500/10 border-blue-500/20' };
    case 'openwrt':
      return { label: 'OpenWrt', colorClass: 'text-green-400 bg-green-500/10 border-green-500/20' };
    case 'external':
      return { label: 'External', colorClass: 'text-orange-400 bg-orange-500/10 border-orange-500/20' };
    case 'manual':
      return { label: 'Manual', colorClass: 'text-gray-400 bg-gray-500/10 border-gray-500/20' };
    case 'lpg':
      return { label: 'LPG', colorClass: 'text-indigo-400 bg-indigo-500/10 border-indigo-500/20' };
    default:
      return null;
  }
}
