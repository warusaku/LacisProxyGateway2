// CelestialGlobe v2 — Constants (mobes2.0 design spec)
// SSoT: All visual constants for CelestialGlobe rendering

import type { NodeType, EdgeType, LogicDeviceType } from './types';

// ============================================================================
// Node colors by type (mobes2.0 spec)
// ============================================================================

export const NODE_COLORS: Record<NodeType, { bg: string; border: string; text: string }> = {
  internet:     { bg: '#60A5FA', border: '#2563EB', text: '#FFFFFF' }, // blue (cloud)
  controller:   { bg: '#818CF8', border: '#6366F1', text: '#FFFFFF' }, // indigo
  gateway:      { bg: '#34D399', border: '#10B981', text: '#FFFFFF' }, // emerald
  router:       { bg: '#34D399', border: '#10B981', text: '#FFFFFF' }, // emerald
  switch:       { bg: '#34D399', border: '#059669', text: '#FFFFFF' }, // emerald
  ap:           { bg: '#60A5FA', border: '#3B82F6', text: '#FFFFFF' }, // blue
  client:       { bg: '#FBBF24', border: '#F59E0B', text: '#1F2937' }, // amber
  wg_peer:      { bg: '#10B981', border: '#059669', text: '#FFFFFF' }, // green
  logic_device: { bg: '#9CA3AF', border: '#6B7280', text: '#FFFFFF' }, // gray
  external:     { bg: '#F59E0B', border: '#D97706', text: '#1F2937' }, // orange
  lpg_server:   { bg: '#3B82F6', border: '#2563EB', text: '#FFFFFF' }, // blue
};

// ============================================================================
// Node sizes by type (width x height)
// ============================================================================

// Node sizes (reference only — DeviceNode.tsx computes width dynamically: infra=200, client=160)
export const NODE_SIZES: Record<NodeType, { width: number; height: number }> = {
  internet:     { width: 160, height: 72 },
  controller:   { width: 200, height: 100 },
  gateway:      { width: 200, height: 80 },
  router:       { width: 200, height: 80 },
  switch:       { width: 200, height: 64 },
  ap:           { width: 200, height: 64 },
  client:       { width: 160, height: 52 },
  wg_peer:      { width: 160, height: 52 },
  logic_device: { width: 200, height: 60 },
  external:     { width: 200, height: 72 },
  lpg_server:   { width: 200, height: 80 },
};

// ============================================================================
// Lucide icon names by node type
// ============================================================================

export const NODE_ICONS: Record<NodeType, string> = {
  internet:     'Cloud',
  controller:   'Globe',
  gateway:      'Globe',
  router:       'Globe',
  switch:       'GitBranch',
  ap:           'Wifi',
  client:       'Monitor',
  wg_peer:      'Shield',
  logic_device: 'Box',
  external:     'HardDrive',
  lpg_server:   'Server',
};

// ============================================================================
// Edge styles by type
// ============================================================================

// mobes2.0 準拠のエッジスタイル (SSOT: mobes2.0 TopologyEdge.tsx EDGE_STYLES)
// 色はmobes2.0原色をダークテーマ向けに微調整（暗い色は視認性確保のため明度+10%）
export const EDGE_STYLES: Record<EdgeType, {
  color: string;
  strokeWidth: number;
  strokeDasharray?: string;
  animated: boolean;
}> = {
  wired:    { color: '#607D8B', strokeWidth: 2, animated: false },                       // mobes2.0=#455A64 → dark bg用明度調整
  wireless: { color: '#4CAF50', strokeWidth: 2, strokeDasharray: '5 5', animated: true }, // mobes2.0=#4CAF50 exact
  vpn:      { color: '#9C27B0', strokeWidth: 2, strokeDasharray: '10 5', animated: false }, // mobes2.0=#9C27B0 exact
  logical:  { color: '#00BCD4', strokeWidth: 1, strokeDasharray: '3 3', animated: false }, // mobes2.0 virtual=#00BCD4 exact
  route:    { color: '#EF4444', strokeWidth: 3, animated: true },                          // LPG2固有: プロキシルート表示用
};

// ============================================================================
// Status colors
// ============================================================================

export const STATUS_COLORS: Record<string, string> = {
  online:   '#10B981', // emerald
  active:   '#10B981',
  offline:  '#6B7280', // gray
  inactive: '#6B7280',
  warning:  '#F59E0B', // amber
  error:    '#EF4444', // red
  unknown:  '#9CA3AF', // gray-400
  manual:   '#9CA3AF',
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
