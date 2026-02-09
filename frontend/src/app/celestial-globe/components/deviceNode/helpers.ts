// CelestialGlobe v2 — DeviceNode Helpers
// mobes2.0 helpers.tsx (518行) から LPG2 向けに移植
// 純粋関数のみ。React hooks なし。

import type { TopologyNodeV2 } from '../../types';
import { STATUS_COLORS, SOURCE_BADGES } from '../../constants';

// ============================================================================
// Status resolution
// ============================================================================

/** ComputedStatus: stateType → 表示ステータスのマッピング */
export function resolveComputedStatus(stateType: string, status: string): string {
  // stateType が明示的なら stateType を優先
  if (stateType && stateType !== 'unknown') {
    return stateType;
  }
  return status || 'unknown';
}

// ============================================================================
// Status badge info
// ============================================================================

export interface StatusBadgeInfo {
  label: string;
  bg: string;
  text: string;
  ring: string;
}

export const STATUS_BADGE_MAP: Record<string, StatusBadgeInfo> = {
  online:          { label: 'Online',         bg: 'bg-emerald-500', text: 'text-white',      ring: 'ring-emerald-400' },
  ImportOnline:    { label: 'Online',         bg: 'bg-emerald-500', text: 'text-white',      ring: 'ring-emerald-400' },
  trackingOnline:  { label: 'Tracking',       bg: 'bg-green-500',   text: 'text-white',      ring: 'ring-green-400' },
  StaticOnline:    { label: 'Static',         bg: 'bg-cyan-500',    text: 'text-white',      ring: 'ring-cyan-400' },
  manual_online:   { label: 'Manual ON',      bg: 'bg-cyan-600',    text: 'text-white',      ring: 'ring-cyan-400' },
  offline:         { label: 'Offline',        bg: 'bg-gray-500',    text: 'text-white',      ring: 'ring-gray-400' },
  ImportOffline:   { label: 'Offline',        bg: 'bg-gray-500',    text: 'text-white',      ring: 'ring-gray-400' },
  trackingOffline: { label: 'Lost',           bg: 'bg-yellow-500',  text: 'text-gray-900',   ring: 'ring-yellow-400' },
  timeoutOffline:  { label: 'Timeout',        bg: 'bg-red-500',     text: 'text-white',      ring: 'ring-red-400' },
  StaticOffline:   { label: 'Static OFF',     bg: 'bg-gray-400',    text: 'text-gray-900',   ring: 'ring-gray-300' },
  unknown:         { label: 'Unknown',        bg: 'bg-gray-400',    text: 'text-gray-900',   ring: 'ring-gray-300' },
};

export function getStatusBadge(stateType: string, status: string): StatusBadgeInfo {
  const computed = resolveComputedStatus(stateType, status);
  return STATUS_BADGE_MAP[computed] ?? STATUS_BADGE_MAP['unknown'];
}

// ============================================================================
// Status color
// ============================================================================

export function getStatusColor(status: string): string {
  return STATUS_COLORS[status] ?? STATUS_COLORS['unknown'] ?? '#9CA3AF';
}

export function isOfflineStatus(status: string): boolean {
  return ['offline', 'ImportOffline', 'trackingOffline', 'timeoutOffline', 'StaticOffline'].includes(status);
}

// ============================================================================
// Source badge
// ============================================================================

export function getSourceBadge(source: string): { label: string; bg: string; text: string } | null {
  return SOURCE_BADGES[source] ?? null;
}

// ============================================================================
// Node display helpers
// ============================================================================

export function formatMacDisplay(mac?: string): string {
  if (!mac) return '';
  // Already formatted (XX:XX:XX:XX:XX:XX)
  if (mac.includes(':')) return mac;
  // 12-char hex → formatted
  if (/^[0-9A-Fa-f]{12}$/.test(mac)) {
    return mac.toUpperCase().match(/.{2}/g)!.join(':');
  }
  return mac.toUpperCase();
}

export function formatNodeTypeLabel(nodeType: string): string {
  const labels: Record<string, string> = {
    internet: 'Internet',
    controller: 'Controller',
    gateway: 'Gateway',
    router: 'Router',
    switch: 'Switch',
    ap: 'Access Point',
    client: 'Client',
    wg_peer: 'WireGuard Peer',
    logic_device: 'Logic Device',
    external: 'External',
    lpg_server: 'LPG Server',
  };
  return labels[nodeType] ?? nodeType;
}

export function getConnectionBadge(connType: string): { label: string; color: string } | null {
  switch (connType) {
    case 'wired': return { label: 'Wired', color: 'bg-blue-500/80' };
    case 'wireless': return { label: 'Wi-Fi', color: 'bg-green-500/80' };
    case 'vpn': return { label: 'VPN', color: 'bg-purple-500/80' };
    default: return null;
  }
}
