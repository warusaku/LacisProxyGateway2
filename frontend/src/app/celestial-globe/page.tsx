'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { topologyApi, lacisIdApi, TopologyNode, TopologyResponse } from '@/lib/api';

// Color mapping for node types
const NODE_COLORS: Record<string, { background: string; border: string }> = {
  controller: { background: '#1e40af', border: '#3b82f6' },
  gateway: { background: '#1e40af', border: '#3b82f6' },
  router: { background: '#1e40af', border: '#60a5fa' },
  switch: { background: '#065f46', border: '#10b981' },
  ap: { background: '#6b21a8', border: '#a855f7' },
  client: { background: '#374151', border: '#6b7280' },
  external: { background: '#92400e', border: '#f59e0b' },
};

const EDGE_COLORS: Record<string, string> = {
  wired: '#6b7280',
  wireless: '#8b5cf6',
  vpn: '#10b981',
  logical: '#f59e0b',
};

const NODE_SHAPES: Record<string, string> = {
  controller: 'diamond',
  gateway: 'box',
  router: 'box',
  switch: 'box',
  ap: 'triangle',
  client: 'dot',
  external: 'hexagon',
};

export default function CelestialGlobePage() {
  const containerRef = useRef<HTMLDivElement>(null);
  const networkRef = useRef<unknown>(null);
  const [topology, setTopology] = useState<TopologyResponse | null>(null);
  const [selectedNode, setSelectedNode] = useState<TopologyNode | null>(null);
  const [sourceFilter, setSourceFilter] = useState<string[]>(['omada', 'openwrt', 'external']);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [assigning, setAssigning] = useState(false);

  const loadTopology = useCallback(async () => {
    try {
      setLoading(true);
      const data = await topologyApi.getTopology();
      setTopology(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load topology');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadTopology();
  }, [loadTopology]);

  // Initialize vis-network when topology data or filters change
  useEffect(() => {
    if (!topology || !containerRef.current) return;

    let cancelled = false;

    const initNetwork = async () => {
      const vis = await import('vis-network/standalone');
      if (cancelled || !containerRef.current) return;

      // Filter nodes and edges by source
      const filteredNodes = topology.nodes.filter(n => sourceFilter.includes(n.source));
      const filteredNodeIds = new Set(filteredNodes.map(n => n.id));
      const filteredEdges = topology.edges.filter(
        e => filteredNodeIds.has(e.from) && filteredNodeIds.has(e.to)
      );

      const nodeData = filteredNodes.map(n => {
        const colors = NODE_COLORS[n.node_type] || NODE_COLORS.client;
        const isOffline = n.status === 'offline' || n.status === 'inactive' || n.status === 'error' || n.status === 'disconnected';

        return {
          id: n.id,
          label: n.label,
          shape: NODE_SHAPES[n.node_type] || 'dot',
          color: {
            background: isOffline ? '#1f2937' : colors.background,
            border: isOffline ? '#4b5563' : colors.border,
            highlight: { background: colors.border, border: '#fff' },
          },
          font: {
            color: isOffline ? '#6b7280' : '#e5e7eb',
            size: n.node_type === 'client' ? 10 : 14,
          },
          size: n.node_type === 'controller' ? 30 : n.node_type === 'client' ? 10 : 20,
          borderWidth: 2,
          title: `${n.label}\n${n.node_type} (${n.source})\n${n.ip || ''}\n${n.mac || ''}`,
        };
      });

      const edgeData = filteredEdges.map((e, i) => ({
        id: `edge-${i}`,
        from: e.from,
        to: e.to,
        color: { color: EDGE_COLORS[e.edge_type] || '#6b7280', opacity: 0.6 },
        dashes: e.edge_type === 'wireless' || e.edge_type === 'vpn',
        label: e.label || undefined,
        font: { color: '#6b7280', size: 9 },
        arrows: 'to',
        smooth: { enabled: true, type: 'cubicBezier', forceDirection: 'vertical', roundness: 0.5 },
      }));

      const options = {
        layout: {
          hierarchical: {
            direction: 'UD',
            sortMethod: 'hubsize',
            nodeSpacing: 150,
            levelSeparation: 120,
            blockShifting: true,
            edgeMinimization: true,
          },
        },
        physics: false,
        interaction: {
          hover: true,
          tooltipDelay: 200,
          navigationButtons: true,
          keyboard: true,
        },
        nodes: {
          borderWidth: 2,
          shadow: true,
        },
        edges: {
          width: 1.5,
          shadow: false,
        },
      };

      // Destroy previous network if exists
      if (networkRef.current && typeof (networkRef.current as { destroy: () => void }).destroy === 'function') {
        (networkRef.current as { destroy: () => void }).destroy();
      }

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const network = new vis.Network(containerRef.current, { nodes: nodeData as any, edges: edgeData as any }, options);
      networkRef.current = network;

      network.on('selectNode', (params: { nodes: string[] }) => {
        if (params.nodes.length > 0) {
          const node = topology.nodes.find(n => n.id === params.nodes[0]);
          if (node) setSelectedNode(node);
        }
      });

      network.on('deselectNode', () => {
        setSelectedNode(null);
      });
    };

    initNetwork();

    return () => {
      cancelled = true;
    };
  }, [topology, sourceFilter]);

  const toggleSourceFilter = (source: string) => {
    setSourceFilter(prev =>
      prev.includes(source) ? prev.filter(s => s !== source) : [...prev, source]
    );
  };

  const handleAssignLacisId = async (node: TopologyNode) => {
    if (!node.candidate_lacis_id || !node.mac) return;
    setAssigning(true);
    try {
      // Determine device_id based on source
      const deviceId = node.mac; // omada uses MAC; openwrt/external use their IDs from node.id
      const idParts = node.id.split(':');
      // Format: "{source}:{id}:dev:{mac}" or "{source}:{id}:ctrl" etc
      const actualDeviceId = node.source === 'omada' ? node.mac : (idParts.length >= 2 ? idParts[1] : deviceId);
      await lacisIdApi.assign(actualDeviceId, node.source, node.candidate_lacis_id);
      // Refresh topology to reflect change
      await loadTopology();
    } catch (e) {
      console.error('Failed to assign lacis_id:', e);
    } finally {
      setAssigning(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading topology...</div>
      </div>
    );
  }

  if (error) {
    return (
      <Card>
        <div className="text-red-400">Error: {error}</div>
        <button onClick={loadTopology} className="mt-2 text-blue-400 underline">
          Retry
        </button>
      </Card>
    );
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold">CelestialGlobe</h2>
        <button
          onClick={loadTopology}
          className="px-3 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 rounded transition-colors"
        >
          Refresh
        </button>
      </div>

      {/* Summary Cards */}
      {topology && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <Card className="text-center">
            <div className="text-2xl font-bold text-blue-400">{topology.metadata.total_devices}</div>
            <div className="text-xs text-gray-400">Devices</div>
          </Card>
          <Card className="text-center">
            <div className="text-2xl font-bold text-gray-400">{topology.metadata.total_clients}</div>
            <div className="text-xs text-gray-400">Clients</div>
          </Card>
          <Card className="text-center">
            <div className="text-2xl font-bold text-purple-400">{topology.metadata.controllers}</div>
            <div className="text-xs text-gray-400">Controllers</div>
          </Card>
          <Card className="text-center">
            <div className="text-2xl font-bold text-green-400">{topology.metadata.routers}</div>
            <div className="text-xs text-gray-400">Routers</div>
          </Card>
        </div>
      )}

      {/* Filters and Legend */}
      <Card className="mb-4">
        <div className="flex flex-wrap items-center gap-4">
          <span className="text-sm text-gray-400">Source Filter:</span>
          {['omada', 'openwrt', 'external'].map(source => (
            <label key={source} className="flex items-center gap-1.5 cursor-pointer select-none">
              <input
                type="checkbox"
                checked={sourceFilter.includes(source)}
                onChange={() => toggleSourceFilter(source)}
                className="w-3.5 h-3.5 rounded border-gray-600 bg-gray-800 text-blue-500 focus:ring-blue-500 focus:ring-offset-0"
              />
              <span className="text-sm capitalize">{source}</span>
            </label>
          ))}
          <div className="ml-auto flex flex-wrap gap-3 text-xs text-gray-400">
            <span className="flex items-center gap-1">
              <span className="w-3 h-3 rounded bg-blue-600 inline-block" /> Gateway/Router
            </span>
            <span className="flex items-center gap-1">
              <span className="w-3 h-3 rounded bg-green-700 inline-block" /> Switch
            </span>
            <span className="flex items-center gap-1">
              <span className="w-3 h-3 rounded bg-purple-700 inline-block" /> AP
            </span>
            <span className="flex items-center gap-1">
              <span className="w-3 h-3 rounded bg-gray-600 inline-block" /> Client
            </span>
            <span className="flex items-center gap-1">
              <span className="w-3 h-3 rounded bg-yellow-700 inline-block" /> External
            </span>
            <span className="flex items-center gap-1">
              <span className="w-3 h-3 rounded bg-gray-800 border border-gray-600 inline-block" /> Offline
            </span>
          </div>
        </div>
      </Card>

      {/* Main: Graph + Detail Panel */}
      <div className="flex gap-4">
        {/* Graph */}
        <Card className="flex-1">
          <div
            ref={containerRef}
            style={{ height: '600px', width: '100%' }}
            className="bg-gray-900 rounded"
          />
        </Card>

        {/* Detail Panel */}
        {selectedNode && (
          <Card className="w-80 shrink-0">
            <h3 className="text-lg font-bold mb-4 truncate">{selectedNode.label}</h3>
            <div className="space-y-3 text-sm">
              <DetailRow label="Type" value={selectedNode.node_type} />
              <DetailRow label="Source" value={selectedNode.source} />
              <DetailRow label="Status" value={selectedNode.status} />
              {selectedNode.mac && <DetailRow label="MAC" value={selectedNode.mac} mono />}
              {selectedNode.ip && <DetailRow label="IP" value={selectedNode.ip} mono />}
              {selectedNode.product_type && <DetailRow label="ProductType" value={selectedNode.product_type} />}
              {selectedNode.network_device_type && <DetailRow label="DeviceType" value={selectedNode.network_device_type} />}
              {/* LacisID Section */}
              {selectedNode.lacis_id ? (
                <div>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-500">LacisID</span>
                    <Badge variant="success">Assigned</Badge>
                  </div>
                  <div className="font-mono text-xs text-green-400 break-all mt-1">
                    {selectedNode.lacis_id}
                  </div>
                </div>
              ) : selectedNode.candidate_lacis_id ? (
                <div>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-500">Candidate LacisID</span>
                    <Badge variant="warning">Unassigned</Badge>
                  </div>
                  <div className="font-mono text-xs text-yellow-400 break-all mt-1">
                    {selectedNode.candidate_lacis_id}
                  </div>
                  <Button
                    size="sm"
                    className="mt-2 w-full"
                    onClick={() => handleAssignLacisId(selectedNode)}
                    disabled={assigning}
                  >
                    {assigning ? 'Assigning...' : 'Assign LacisID'}
                  </Button>
                </div>
              ) : null}

              {/* Metadata */}
              <div className="pt-2 border-t border-gray-700">
                <div className="text-gray-500 mb-1">Metadata</div>
                <div className="space-y-1">
                  {Object.entries(selectedNode.metadata).map(([k, v]) => {
                    if (v === null || v === undefined) return null;
                    return (
                      <div key={k} className="flex justify-between text-xs">
                        <span className="text-gray-500">{k}</span>
                        <span className="text-gray-300 truncate ml-2 max-w-[150px]">
                          {typeof v === 'object' ? JSON.stringify(v) : String(v)}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          </Card>
        )}
      </div>
    </div>
  );
}

function DetailRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex justify-between">
      <span className="text-gray-500">{label}</span>
      <span className={`${mono ? 'font-mono text-xs' : ''} text-gray-200 truncate ml-2 max-w-[180px]`}>
        {value}
      </span>
    </div>
  );
}
