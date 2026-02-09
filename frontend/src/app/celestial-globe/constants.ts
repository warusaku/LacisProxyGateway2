// CelestialGlobe v2 — Constants (mobes2.0 design spec)
// SSoT: All visual constants for CelestialGlobe rendering

import type { NodeType, EdgeType, LogicDeviceType } from './types';

// ============================================================================
// Node colors by type (mobes2.0 spec — Tailwind class names for dark theme)
// ============================================================================

export const NODE_COLORS: Record<NodeType, {
  bg: string;
  border: string;
  text: string;
  gradient: string;
}> = {
  internet:     { bg: 'bg-indigo-500/20',   border: 'border-indigo-400',   text: 'text-indigo-100',  gradient: 'from-indigo-100 via-white to-indigo-50 dark:from-indigo-950/60 dark:via-indigo-900/30 dark:to-indigo-800/40' },
  controller:   { bg: 'bg-violet-500/20',   border: 'border-violet-400',   text: 'text-violet-100',  gradient: 'from-violet-50 to-white dark:from-violet-950/40 dark:to-zinc-900' },
  gateway:      { bg: 'bg-emerald-500/20',  border: 'border-emerald-400',  text: 'text-emerald-100', gradient: 'from-emerald-50 to-white dark:from-emerald-950/40 dark:to-zinc-900' },
  router:       { bg: 'bg-emerald-500/20',  border: 'border-emerald-400',  text: 'text-emerald-100', gradient: 'from-emerald-50 to-white dark:from-emerald-950/40 dark:to-zinc-900' },
  switch:       { bg: 'bg-sky-500/20',      border: 'border-sky-400',      text: 'text-sky-100',     gradient: 'from-sky-50 to-white dark:from-sky-950/40 dark:to-zinc-900' },
  ap:           { bg: 'bg-blue-500/20',     border: 'border-blue-400',     text: 'text-blue-100',    gradient: 'from-blue-50 to-white dark:from-blue-950/40 dark:to-zinc-900' },
  client:       { bg: 'bg-amber-500/20',    border: 'border-amber-400',    text: 'text-amber-100',   gradient: 'from-amber-50 to-white dark:from-amber-950/40 dark:to-zinc-900' },
  wg_peer:      { bg: 'bg-teal-500/20',     border: 'border-teal-400',     text: 'text-teal-100',    gradient: 'from-teal-50 to-white dark:from-teal-950/40 dark:to-zinc-900' },
  logic_device: { bg: 'bg-teal-500/10',     border: 'border-teal-500/50',  text: 'text-teal-100',    gradient: 'from-teal-50/80 to-white dark:from-teal-950/40 dark:to-zinc-900' },
  external:     { bg: 'bg-orange-500/20',   border: 'border-orange-400',   text: 'text-orange-100',  gradient: 'from-orange-50 to-white dark:from-orange-950/40 dark:to-zinc-900' },
  lpg_server:   { bg: 'bg-blue-500/20',     border: 'border-blue-500',     text: 'text-blue-100',    gradient: 'from-blue-50 to-white dark:from-blue-950/40 dark:to-zinc-900' },
};

// ============================================================================
// Edge styles by type
// ============================================================================

export const EDGE_STYLES: Record<EdgeType, {
  color: string;
  strokeWidth: number;
  strokeDasharray?: string;
  animated: boolean;
}> = {
  wired:    { color: '#607D8B', strokeWidth: 2, animated: false },
  wireless: { color: '#4CAF50', strokeWidth: 2, strokeDasharray: '5 5', animated: true },
  vpn:      { color: '#9C27B0', strokeWidth: 2, strokeDasharray: '10 5', animated: false },
  logical:  { color: '#00BCD4', strokeWidth: 1, strokeDasharray: '3 3', animated: false },
  route:    { color: '#EF4444', strokeWidth: 3, animated: true },
};

// ============================================================================
// Status colors
// ============================================================================

export const STATUS_COLORS: Record<string, string> = {
  online:         '#10B981',
  active:         '#10B981',
  ImportOnline:   '#10B981',
  trackingOnline: '#10B981',
  StaticOnline:   '#22D3EE',
  manual_online:  '#22D3EE',
  offline:        '#6B7280',
  inactive:       '#6B7280',
  ImportOffline:  '#6B7280',
  trackingOffline:'#6B7280',
  timeoutOffline: '#EF4444',
  StaticOffline:  '#9CA3AF',
  warning:        '#F59E0B',
  error:          '#EF4444',
  unknown:        '#9CA3AF',
  manual:         '#9CA3AF',
};

// ============================================================================
// Source badge config
// ============================================================================

export const SOURCE_BADGES: Record<string, { label: string; bg: string; text: string }> = {
  omada:    { label: 'Omada',    bg: 'bg-sky-600',     text: 'text-white' },
  openwrt:  { label: 'OpenWrt',  bg: 'bg-blue-600',    text: 'text-white' },
  external: { label: 'External', bg: 'bg-orange-600',  text: 'text-white' },
  manual:   { label: 'Manual',   bg: 'bg-gray-600',    text: 'text-white' },
  logic:    { label: 'Logic',    bg: 'bg-teal-600',    text: 'text-white' },
  lpg:      { label: 'LPG',      bg: 'bg-blue-700',    text: 'text-white' },
};

// ============================================================================
// LogicDevice type labels
// ============================================================================

export const LOGIC_DEVICE_TYPE_LABELS: Record<LogicDeviceType, string> = {
  switch:    'Switch',
  hub:       'Hub',
  converter: 'Media Converter',
  ups:       'UPS',
  other:     'Other',
};

// ============================================================================
// Layout constants
// ============================================================================

export const LAYOUT = {
  OUTLINE_WIDTH: 320,
  PROPERTY_WIDTH: 360,
  DEPTH_SPACING: 280,
  SIBLING_GAP: 24,
  NODE_HEIGHT_DEFAULT: 80,
  NODE_HEIGHT_COMPACT: 48,
} as const;

// ============================================================================
// Theme
// ============================================================================

export const THEME = {
  BG: '#020202',
  CARD_BG: 'rgba(10, 10, 10, 0.85)',
  CARD_BORDER: 'rgba(51, 51, 51, 0.5)',
  TEXT_PRIMARY: '#E5E7EB',
  TEXT_SECONDARY: '#9CA3AF',
  TEXT_MUTED: '#6B7280',
} as const;
