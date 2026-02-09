// CelestialGlobe v2 — ContextMenu
// mobes2.0 ContextMenu.tsx (534行) 準拠
// ノード/ペインの右クリックメニュー

'use client';

import React, { useEffect, useCallback, useRef } from 'react';
import { useUIStateStore } from '../stores/useUIStateStore';
import { useTopologyStore } from '../stores/useTopologyStore';
import {
  ChevronRight,
  FoldVertical,
  UnfoldVertical,
  Trash2,
  Edit3,
  Plus,
  Copy,
  XCircle,
} from 'lucide-react';

// ============================================================================
// Menu Item Component
// ============================================================================

interface MenuItemProps {
  icon?: React.ReactNode;
  label: string;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  onClick: () => void;
}

function MenuItem({ icon, label, shortcut, danger, disabled, onClick }: MenuItemProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={[
        'w-full flex items-center gap-2 px-3 py-1.5 text-sm rounded-md transition-colors',
        danger
          ? 'text-red-400 hover:bg-red-500/10'
          : 'text-gray-300 hover:bg-white/10',
        disabled ? 'opacity-40 pointer-events-none' : '',
      ].join(' ')}
    >
      <span className="w-4 h-4 flex-shrink-0 flex items-center justify-center">
        {icon}
      </span>
      <span className="flex-1 text-left">{label}</span>
      {shortcut && (
        <span className="text-xs text-gray-500 ml-2">{shortcut}</span>
      )}
    </button>
  );
}

// ============================================================================
// Divider
// ============================================================================

function Divider() {
  return <div className="border-t border-white/10 my-1" />;
}

// ============================================================================
// Context Menu
// ============================================================================

export function ContextMenu() {
  const { contextMenu, closeContextMenu, selectOnly } = useUIStateStore();
  const { toggleCollapse, deleteLogicDevice } = useTopologyStore();
  const nodes = useTopologyStore(s => s.nodes);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close on click outside
  useEffect(() => {
    if (!contextMenu.isOpen) return;

    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as HTMLElement)) {
        closeContextMenu();
      }
    };
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeContextMenu();
    };

    document.addEventListener('mousedown', handleClick);
    document.addEventListener('keydown', handleEscape);
    return () => {
      document.removeEventListener('mousedown', handleClick);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [contextMenu.isOpen, closeContextMenu]);

  if (!contextMenu.isOpen) return null;

  const targetNode = contextMenu.nodeId
    ? nodes.find(n => n.id === contextMenu.nodeId)
    : null;

  const isLogicDevice = targetNode?.node_type === 'logic_device';
  const isInternet = targetNode?.node_type === 'internet';

  const handleCollapse = () => {
    if (contextMenu.nodeId) {
      toggleCollapse(contextMenu.nodeId);
    }
    closeContextMenu();
  };

  const handleEdit = () => {
    if (contextMenu.nodeId) {
      // Dispatch label edit event
      const event = new CustomEvent('cg:start-edit', {
        detail: { nodeId: contextMenu.nodeId },
        bubbles: true,
      });
      document.dispatchEvent(event);
    }
    closeContextMenu();
  };

  const handleDelete = () => {
    if (contextMenu.nodeId && isLogicDevice) {
      if (window.confirm(`Delete "${targetNode?.label}"?`)) {
        deleteLogicDevice(contextMenu.nodeId);
      }
    }
    closeContextMenu();
  };

  const handleSelect = () => {
    if (contextMenu.nodeId) {
      selectOnly([contextMenu.nodeId]);
    }
    closeContextMenu();
  };

  const handleDeselect = () => {
    useUIStateStore.getState().clearSelection();
    closeContextMenu();
  };

  return (
    <div
      ref={menuRef}
      className="fixed z-[100] animate-pop-in"
      style={{ left: contextMenu.x, top: contextMenu.y }}
    >
      <div className="cg-glass-card py-1 min-w-[200px] shadow-2xl">
        {targetNode ? (
          <>
            {/* Header */}
            <div className="px-3 py-1.5 text-xs font-medium text-gray-500 truncate">
              {targetNode.label}
            </div>
            <Divider />

            {/* Node actions */}
            <MenuItem
              icon={<Edit3 size={14} />}
              label="Edit Label"
              shortcut="F2"
              onClick={handleEdit}
              disabled={isInternet}
            />

            <MenuItem
              icon={targetNode.collapsed ? <UnfoldVertical size={14} /> : <FoldVertical size={14} />}
              label={targetNode.collapsed ? 'Expand' : 'Collapse'}
              onClick={handleCollapse}
              disabled={isInternet}
            />

            <MenuItem
              icon={<Copy size={14} />}
              label="Select"
              onClick={handleSelect}
            />

            {isLogicDevice && (
              <>
                <Divider />
                <MenuItem
                  icon={<Trash2 size={14} />}
                  label="Delete"
                  shortcut="Del"
                  danger
                  onClick={handleDelete}
                />
              </>
            )}
          </>
        ) : (
          <>
            {/* Pane context menu */}
            <MenuItem
              icon={<Plus size={14} />}
              label="Add Device"
              onClick={() => {
                // Dispatch add device event
                const event = new CustomEvent('cg:add-device', {
                  detail: { x: contextMenu.x, y: contextMenu.y },
                  bubbles: true,
                });
                document.dispatchEvent(event);
                closeContextMenu();
              }}
            />
            <Divider />
            <MenuItem
              icon={<XCircle size={14} />}
              label="Clear Selection"
              onClick={handleDeselect}
            />
          </>
        )}
      </div>
    </div>
  );
}
