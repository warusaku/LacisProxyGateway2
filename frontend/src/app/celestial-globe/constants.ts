// CelestialGlobe v2 — Constants (mobes2.0 design spec)
// SSoT: All visual constants for CelestialGlobe rendering

import type { NodeType, EdgeType, LogicDeviceType } from './types';

// ============================================================================
// Node colors by type (mobes2.0 spec)
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
  online:        '#10B981',
  active:        '#10B981',
  StaticOnline:  '#10B981',
  offline:       '#6B7280',
  inactive:      '#6B7280',
  StaticOffline: '#9CA3AF',
  warning:       '#F59E0B',
  error:         '#EF4444',
  unknown:       '#9CA3AF',
  manual:        '#9CA3AF',
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
