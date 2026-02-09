// CelestialGlobe v2 — Type Definitions
// SSoT: All topology-related types for the CelestialGlobe feature

import type { Node, Edge } from 'reactflow';

// ============================================================================
// Backend API response types (mirrors Rust TopologyV2Response)
// ============================================================================

export interface Position {
  x: number;
  y: number;
  pinned: boolean;
}

export interface TopologyNodeV2 {
  id: string;
  label: string;
  node_type: NodeType;
  mac?: string;
  ip?: string;
  source: DataSource;
  parent_id?: string;
  lacis_id?: string;
  candidate_lacis_id?: string;
  product_type?: string;
  network_device_type?: string;
  status: string;
  metadata: Record<string, unknown>;
  // v2 additions
  position: Position;
  collapsed: boolean;
  collapsed_child_count: number;
  descendant_count: number;
  connection_type: ConnectionType;
  fid?: string;
  facility_name?: string;
}

export interface TopologyEdgeV2 {
  from: string;
  to: string;
  edge_type: EdgeType;
  label?: string;
}

export interface TopologyMetadataV2 {
  total_devices: number;
  total_clients: number;
  controllers: number;
  routers: number;
  logic_devices: number;
  generated_at: string;
}

export interface ViewConfig {
  collapsed_node_ids: string[];
  last_layout_at: string;
}

export interface TopologyV2Response {
  nodes: TopologyNodeV2[];
  edges: TopologyEdgeV2[];
  metadata: TopologyMetadataV2;
  view_config: ViewConfig;
}

// ============================================================================
// LogicDevice (manual device addition)
// ============================================================================

export interface LogicDevice {
  id: string;
  label: string;
  device_type: LogicDeviceType;
  parent_id?: string;
  ip?: string;
  location?: string;
  note?: string;
  lacis_id?: string;
  created_at: string;
}

export interface CreateLogicDeviceRequest {
  label: string;
  device_type: LogicDeviceType;
  parent_id?: string;
  ip?: string;
  location?: string;
  note?: string;
}

export interface UpdateLogicDeviceRequest {
  label?: string;
  device_type?: LogicDeviceType;
  parent_id?: string;
  ip?: string;
  location?: string;
  note?: string;
}

// ============================================================================
// Enums (MECE: mutually exclusive, collectively exhaustive)
// ============================================================================

export type NodeType =
  | 'controller'
  | 'gateway'
  | 'router'
  | 'switch'
  | 'ap'
  | 'client'
  | 'wg_peer'
  | 'logic_device'
  | 'external'
  | 'lpg_server';

export type DataSource =
  | 'omada'
  | 'openwrt'
  | 'external'
  | 'logic'
  | 'lpg';

export type EdgeType =
  | 'wired'
  | 'wireless'
  | 'vpn'
  | 'logical'
  | 'route';

export type ConnectionType =
  | 'wired'
  | 'wireless'
  | 'vpn';

export type LogicDeviceType =
  | 'switch'
  | 'hub'
  | 'converter'
  | 'ups'
  | 'other';

export type ViewMode = 'mindmap' | 'outline' | 'split';

export type TopologyViewFilter = 'full' | 'routes' | 'site';

// ============================================================================
// React Flow node/edge data types
// ============================================================================

export interface DeviceNodeData {
  node: TopologyNodeV2;
  selected: boolean;
  onCollapse: (nodeId: string) => void;
}

export type DeviceFlowNode = Node<DeviceNodeData>;
export type TopologyFlowEdge = Edge;

// ============================================================================
// Store types
// ============================================================================

export interface TopologyStoreState {
  // Data
  nodes: TopologyNodeV2[];
  edges: TopologyEdgeV2[];
  metadata: TopologyMetadataV2 | null;
  viewConfig: ViewConfig | null;

  // UI state
  selectedNodeId: string | null;
  viewMode: ViewMode;
  viewFilter: TopologyViewFilter;
  siteFilter: string | null;
  loading: boolean;
  error: string | null;

  // Actions
  fetchTopology: () => Promise<void>;
  recalcLayout: () => Promise<void>;
  updateNodePosition: (nodeId: string, x: number, y: number) => Promise<void>;
  toggleCollapse: (nodeId: string) => Promise<void>;
  updateParent: (nodeId: string, newParentId: string) => Promise<void>;
  createLogicDevice: (req: CreateLogicDeviceRequest) => Promise<void>;
  updateLogicDevice: (id: string, req: UpdateLogicDeviceRequest) => Promise<void>;
  deleteLogicDevice: (id: string) => Promise<void>;
  setSelectedNodeId: (id: string | null) => void;
  setViewMode: (mode: ViewMode) => void;
  setViewFilter: (filter: TopologyViewFilter, siteFilter?: string) => void;
}
