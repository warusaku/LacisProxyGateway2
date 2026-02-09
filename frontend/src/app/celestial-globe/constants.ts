// CelestialGlobe v2 — Constants (mobes2.0 design spec)
// SSoT: All visual constants for CelestialGlobe rendering

import type { NodeType, EdgeType, LogicDeviceType } from './types';

// ============================================================================
// Node colors by type (Legacy — Legend, PropertyPanel で参照)
// DeviceNode は Tailwind + helpers.ts に移行済み
// ============================================================================

export const NODE_COLORS: Record<NodeType, { bg: string; border: string; text: string }> = {
  internet:     { bg: '#60A5FA', border: '#2563EB', text: '#FFFFFF' },
  controller:   { bg: '#818CF8', border: '#6366F1', text: '#FFFFFF' },
  gateway:      { bg: '#34D399', border: '#10B981', text: '#FFFFFF' },
  router:       { bg: '#34D399', border: '#10B981', text: '#FFFFFF' },
  switch:       { bg: '#34D399', border: '#059669', text: '#FFFFFF' },
  ap:           { bg: '#60A5FA', border: '#3B82F6', text: '#FFFFFF' },
  client:       { bg: '#FBBF24', border: '#F59E0B', text: '#1F2937' },
  wg_peer:      { bg: '#10B981', border: '#059669', text: '#FFFFFF' },
  logic_device: { bg: '#9CA3AF', border: '#6B7280', text: '#FFFFFF' },
  external:     { bg: '#F59E0B', border: '#D97706', text: '#1F2937' },
  lpg_server:   { bg: '#3B82F6', border: '#2563EB', text: '#FFFFFF' },
};

// ============================================================================
// Node sizes by type (width x height)
// ============================================================================

// Node sizes — must match backend node_height() for layout consistency
// Height = actual rendered height (padding + content + border) to prevent overlap
export const NODE_SIZES: Record<NodeType, { width: number; height: number }> = {
  internet:     { width: 160, height: 120 },
  controller:   { width: 200, height: 110 },
  gateway:      { width: 200, height: 110 },
  router:       { width: 200, height: 110 },
  switch:       { width: 200, height: 100 },
  ap:           { width: 200, height: 100 },
  client:       { width: 160, height: 94 },
  wg_peer:      { width: 160, height: 94 },
  logic_device: { width: 200, height: 94 },
  external:     { width: 200, height: 100 },
  lpg_server:   { width: 200, height: 110 },
};

// ============================================================================
// Edge styles by type (mobes2.0 準拠)
// ============================================================================

export const EDGE_STYLES: Record<EdgeType, {
  color: string;
  strokeWidth: number;
  strokeDasharray?: string;
  animated: boolean;
}> = {
  wired:    { color: '#2563EB', strokeWidth: 2, animated: false },                       // mobes2.0 blue-600
  wireless: { color: '#06B6D4', strokeWidth: 2, strokeDasharray: '5 5', animated: true }, // mobes2.0 cyan-500
  vpn:      { color: '#0EA5E9', strokeWidth: 2, strokeDasharray: '10 5', animated: false }, // mobes2.0 sky-500
  logical:  { color: '#00BCD4', strokeWidth: 1, strokeDasharray: '3 3', animated: false }, // mobes2.0 virtual
  route:    { color: '#EF4444', strokeWidth: 3, animated: true },                          // LPG2固有
};

// ============================================================================
// Edge colors (mobes2.0 TopologyEdge 準拠 — MiniMap等で使用)
// ============================================================================

export const EDGE_COLORS: Record<EdgeType, string> = {
  wired: '#2563EB',
  wireless: '#06B6D4',
  vpn: '#0EA5E9',
  logical: '#00BCD4',
  route: '#EF4444',
};

// ============================================================================
// Status colors (MiniMap等で使用、DeviceNodeはTailwind+helpers.tsに移行)
// ============================================================================

export const STATUS_COLORS: Record<string, string> = {
  online:        '#10B981', // emerald
  active:        '#10B981',
  StaticOnline:  '#10B981', // emerald (admin forced online)
  offline:       '#6B7280', // gray
  inactive:      '#6B7280',
  StaticOffline: '#9CA3AF', // gray-400 (admin forced offline / maintenance)
  warning:       '#F59E0B', // amber
  error:         '#EF4444', // red
  unknown:       '#9CA3AF', // gray-400
  manual:        '#9CA3AF',
};

// ============================================================================
// LogicDevice type labels (Japanese)
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
  HORIZONTAL_SPACING: 300,
  VERTICAL_SPACING: 120,
  CLIENT_VERTICAL_SPACING: 80,
  OUTLINE_WIDTH: 320,   // px
  PROPERTY_WIDTH: 360,   // px
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
