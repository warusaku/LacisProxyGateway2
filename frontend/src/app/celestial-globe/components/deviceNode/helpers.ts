/**
 * DeviceNode helpers
 * SSoT: mobes2.0 deviceNodeWithLOD/helpers.tsx を LPG2 向けに適合移植
 *
 * LPG2適合:
 *   - TopologyNodeV2.state_type → ComputedStatus 変換
 *   - mobes2.0 DeviceData を使わない（TopologyNodeV2 直接参照）
 */

import type { TopologyNodeV2 } from '../../types';

// ============================================================================
// ComputedStatus (mobes2.0 DeviceComputedStatus 互換)
// ============================================================================

export type ComputedStatus =
  | 'online'
  | 'offline'
  | 'manual_online'
  | 'static_online'
  | 'static_offline'
  | 'inferred_online'
  | 'suspected_down'
  | 'detected';

// ============================================================================
// resolveComputedStatus: LPG2 state_type → ComputedStatus
// ============================================================================

export const resolveComputedStatus = (node: TopologyNodeV2): ComputedStatus => {
  const st = node.state_type;
  const source = node.source;

  if (st === 'StaticOnline') return 'static_online';
  if (st === 'StaticOffline') return 'static_offline';
  if (source === 'manual' && (st === 'online' || st === 'active')) return 'manual_online';
  if (st === 'online' || st === 'active') return 'online';
  if (st === 'offline' || st === 'inactive') return 'offline';
  return 'offline';
};

// ============================================================================
// STATUS_BADGE_INFO (mobes2.0 完全移植)
// ============================================================================

export const STATUS_BADGE_INFO: Partial<Record<ComputedStatus, { label: string; className: string }>> = {
  manual_online: {
    label: 'MANUAL',
    className: 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-300 border border-emerald-500/40',
  },
  static_online: {
    label: 'STATIC',
    className: 'bg-sky-500/15 text-sky-600 dark:text-sky-300 border border-sky-500/30',
  },
  static_offline: {
    label: 'STATIC',
    className: 'bg-amber-500/15 text-amber-600 dark:text-amber-300 border border-amber-500/30',
  },
  inferred_online: {
    label: 'INFER',
    className: 'bg-cyan-500/15 text-cyan-600 dark:text-cyan-300 border border-cyan-500/40',
  },
  suspected_down: {
    label: '障害疑い',
    className: 'bg-orange-500/20 text-orange-600 dark:text-orange-300 border border-orange-500/50 animate-pulse',
  },
  detected: {
    label: '未管理',
    className: 'bg-amber-500/20 text-amber-700 dark:text-amber-300 border border-amber-500/50',
  },
};

// ============================================================================
// getStatusColor (mobes2.0 完全移植)
// ============================================================================

export const getStatusColor = (status: ComputedStatus, isDark: boolean = true): string => {
  switch (status) {
    case 'online':
      return isDark ? '#3b82f6' : '#2563eb';
    case 'manual_online':
      return isDark ? '#10b981' : '#16a34a';
    case 'static_online':
      return isDark ? '#eab308' : '#ca8a04';
    case 'static_offline':
      return isDark ? '#1f2937' : '#000000';
    case 'inferred_online':
      return isDark ? '#22d3ee' : '#06b6d4';
    case 'suspected_down':
      return isDark ? '#f97316' : '#ea580c';
    case 'detected':
      return isDark ? '#f59e0b' : '#d97706';
    case 'offline':
    default:
      return isDark ? '#6b7280' : '#9ca3af';
  }
};

// ============================================================================
// isOfflineStatus (mobes2.0 完全移植)
// ============================================================================

export const isOfflineStatus = (status: ComputedStatus): boolean => (
  status === 'offline' || status === 'static_offline' || status === 'suspected_down'
);

// ============================================================================
// formatDuration (mobes2.0 完全移植)
// ============================================================================

export const formatDuration = (durationMs: number | null): string => {
  if (!durationMs || Number.isNaN(durationMs) || durationMs <= 0) {
    return '—';
  }
  const totalSeconds = Math.floor(durationMs / 1000);
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);

  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
};

// ============================================================================
// buildTooltipContent (mobes2.0 準拠 → LPG2向けに簡略版)
// ============================================================================

export interface TooltipParams {
  label: string;
  ip?: string;
  mac?: string;
  source: string;
  stateType: string;
  nodeType: string;
  lacisId?: string;
  descendantCount: number;
  isGateway: boolean;
}

export const buildTooltipLines = (params: TooltipParams): string[] => {
  const lines: string[] = [];
  lines.push(params.label);
  if (params.isGateway) lines.push('🌐 インターネットゲートウェイ');
  lines.push(`IP: ${params.ip?.trim() || '—'}`);
  if (params.mac) lines.push(`MAC: ${params.mac}`);
  lines.push(`Type: ${params.nodeType} / Source: ${params.source}`);
  lines.push(`State: ${params.stateType}`);
  if (params.lacisId) lines.push(`LacisID: ${params.lacisId}`);
  if (params.descendantCount > 0) lines.push(`配下: ${params.descendantCount} ノード`);
  return lines;
};
