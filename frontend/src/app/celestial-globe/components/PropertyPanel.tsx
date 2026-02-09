// CelestialGlobe v2 — PropertyPanel
// mobes2.0 PropertyPanel.tsx (863行) + PropertyPanelBasicSections.tsx (433行) 準拠
// 選択ノードの詳細表示・編集パネル

'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { X, Save, RotateCcw } from 'lucide-react';
import type { TopologyNodeV2 } from '../types';
import { useTopologyStore } from '../stores/useTopologyStore';
import { useUIStateStore } from '../stores/useUIStateStore';
import { NetworkDeviceIcon } from './icons';
import {
  getStatusBadge,
  getSourceBadge,
  formatMacDisplay,
  formatNodeTypeLabel,
  getConnectionBadge,
} from './deviceNode/helpers';

// ============================================================================
// Section Component
// ============================================================================

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mb-4">
      <h3 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">
        {title}
      </h3>
      <div className="space-y-2">{children}</div>
    </div>
  );
}

function Field({ label, value }: { label: string; value?: string | React.ReactNode }) {
  if (!value) return null;
  return (
    <div className="flex items-start gap-2">
      <span className="text-xs text-gray-500 w-20 flex-shrink-0">{label}</span>
      <span className="text-xs text-gray-300 break-all">{value}</span>
    </div>
  );
}

// ============================================================================
// PropertyPanel
// ============================================================================

export function PropertyPanel() {
  const selectedNodeId = useUIStateStore(s => s.selectedNodeId);
  const clearSelection = useUIStateStore(s => s.clearSelection);
  const nodes = useTopologyStore(s => s.nodes);
  const updateNodeLabel = useTopologyStore(s => s.updateNodeLabel);

  const [editingLabel, setEditingLabel] = useState('');
  const [isDirty, setIsDirty] = useState(false);

  const node: TopologyNodeV2 | null = selectedNodeId
    ? nodes.find(n => n.id === selectedNodeId) ?? null
    : null;

  // Stable references for effect dependencies
  const nodeId = node?.id ?? null;
  const nodeLabel = node?.label ?? '';

  // Reset form on node change
  useEffect(() => {
    if (nodeId) {
      setEditingLabel(nodeLabel);
      setIsDirty(false);
    }
  }, [nodeId, nodeLabel]);

  const handleLabelChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setEditingLabel(e.target.value);
    setIsDirty(e.target.value !== node?.label);
  }, [node?.label]);

  const handleSave = useCallback(async () => {
    if (node && editingLabel.trim() && editingLabel !== node.label) {
      await updateNodeLabel(node.id, editingLabel.trim());
      setIsDirty(false);
    }
  }, [node, editingLabel, updateNodeLabel]);

  const handleRevert = useCallback(() => {
    if (node) {
      setEditingLabel(node.label);
      setIsDirty(false);
    }
  }, [node]);

  if (!node) return null;

  const statusBadge = getStatusBadge(node.state_type ?? '', node.status ?? '');
  const sourceBadge = getSourceBadge(node.source ?? '');
  const connBadge = getConnectionBadge(node.connection_type ?? '');
  const metadata = (node.metadata && typeof node.metadata === 'object' && !Array.isArray(node.metadata))
    ? node.metadata as Record<string, unknown>
    : {};

  return (
    <div className="animate-slide-in-right w-[360px] h-full cg-glass-panel flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-white/10">
        <div className="flex items-center gap-2 min-w-0">
          <NetworkDeviceIcon type={node.node_type} size={20} className="text-gray-400 flex-shrink-0" />
          <span className="text-sm font-semibold text-gray-200 truncate">
            {node.label}
          </span>
        </div>
        <button
          onClick={clearSelection}
          className="p-1 rounded-md hover:bg-white/10 text-gray-400 hover:text-gray-200 transition-colors"
        >
          <X size={16} />
        </button>
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto cg-scrollbar p-4">
        {/* Basic Info */}
        <Section title="Basic Information">
          {/* Label edit */}
          <div>
            <label className="text-xs text-gray-500 block mb-1">Label</label>
            <input
              type="text"
              value={editingLabel}
              onChange={handleLabelChange}
              className="w-full bg-white/5 border border-white/10 rounded-md px-3 py-1.5 text-sm text-gray-200 outline-none focus:ring-1 focus:ring-blue-400"
            />
          </div>

          <Field label="Type" value={
            <span className="flex items-center gap-1">
              <NetworkDeviceIcon type={node.node_type} size={14} className="text-gray-400" />
              {formatNodeTypeLabel(node.node_type)}
            </span>
          } />

          <Field label="Status" value={
            <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${statusBadge.bg} ${statusBadge.text}`}>
              {statusBadge.label}
            </span>
          } />

          <Field label="IP" value={node.ip} />
          <Field label="MAC" value={node.mac ? formatMacDisplay(node.mac) : undefined} />

          {sourceBadge && (
            <Field label="Source" value={
              <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${sourceBadge.bg} ${sourceBadge.text}`}>
                {sourceBadge.label}
              </span>
            } />
          )}
        </Section>

        {/* Identifiers */}
        <Section title="Identifiers">
          <Field label="LacisID" value={node.lacis_id} />
          <Field label="FID" value={node.fid} />
          <Field label="Node ID" value={node.id} />
          {node.parent_id && <Field label="Parent" value={node.parent_id} />}
        </Section>

        {/* Connection */}
        <Section title="Connection">
          {connBadge && (
            <Field label="Type" value={
              <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium text-white ${connBadge.color}`}>
                {connBadge.label}
              </span>
            } />
          )}
          <Field label="Order" value={String(node.order)} />
          <Field label="Collapsed" value={node.collapsed ? 'Yes' : 'No'} />
          <Field label="Descendants" value={String(node.descendant_count)} />
        </Section>

        {/* Metadata */}
        {Object.keys(metadata).length > 0 && (
          <Section title="Metadata">
            {Object.entries(metadata).map(([key, value]) => (
              <Field key={key} label={key} value={String(value ?? '')} />
            ))}
          </Section>
        )}
      </div>

      {/* Footer — Save/Revert */}
      {isDirty && (
        <div className="flex items-center gap-2 p-4 border-t border-white/10">
          <button
            onClick={handleSave}
            className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-md transition-colors"
          >
            <Save size={14} />
            Save
          </button>
          <button
            onClick={handleRevert}
            className="flex items-center justify-center gap-2 px-4 py-2 bg-white/5 hover:bg-white/10 text-gray-300 text-sm rounded-md transition-colors"
          >
            <RotateCcw size={14} />
          </button>
        </div>
      )}
    </div>
  );
}
