'use client';

import { useEffect, useState, useCallback } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Card } from '@/components/ui/Card';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { Pagination } from '@/components/ui/Pagination';
import { Modal } from '@/components/ui/Modal';
import { LogDetailModal } from '@/components/LogDetailModal';
import { dashboardApi, operationLogsApi, toolsApi } from '@/lib/api';
import type { OperationLog, OperationLogSummary } from '@/lib/api';
import { getStatusColor } from '@/lib/format';
import { countryCodeToFlag } from '@/lib/geo';
import type { AccessLog, AccessLogSearchParams, ErrorSummary, IpExclusionParams } from '@/types';

const PER_PAGE = 50;

const METHOD_OPTIONS = [
  { value: '', label: 'All Methods' },
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
  { value: 'PUT', label: 'PUT' },
  { value: 'DELETE', label: 'DELETE' },
  { value: 'PATCH', label: 'PATCH' },
  { value: 'OPTIONS', label: 'OPTIONS' },
];

const STATUS_OPTIONS = [
  { value: '', label: 'All Status' },
  { value: '200-299', label: '2xx Success' },
  { value: '300-399', label: '3xx Redirect' },
  { value: '400-499', label: '4xx Client Error' },
  { value: '500-599', label: '5xx Server Error' },
];

export default function LogsPage() {
  const [logs, setLogs] = useState<AccessLog[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'search' | 'errors' | 'operations'>('search');
  const [errorSummary, setErrorSummary] = useState<ErrorSummary[]>([]);
  const [selectedLog, setSelectedLog] = useState<AccessLog | null>(null);

  // Operations tab state
  const [opLogs, setOpLogs] = useState<OperationLog[]>([]);
  const [opSummary, setOpSummary] = useState<OperationLogSummary | null>(null);
  const [opLoading, setOpLoading] = useState(false);
  const [opTypeFilter, setOpTypeFilter] = useState('');
  const [opStatusFilter, setOpStatusFilter] = useState('');
  const [syncRunning, setSyncRunning] = useState<Record<string, boolean>>({});
  const [toolHost, setToolHost] = useState('');
  const [toolResult, setToolResult] = useState<Record<string, unknown> | null>(null);
  const [toolRunning, setToolRunning] = useState(false);
  const [opDetailLog, setOpDetailLog] = useState<OperationLog | null>(null);

  // IP exclusion filter state
  const [myIp, setMyIp] = useState<string>('');
  const [serverIp, setServerIp] = useState<string>('');
  const [serverIpHistory, setServerIpHistory] = useState<string[]>([]);
  const [adminIpHistory, setAdminIpHistory] = useState<string[]>([]);
  const [ipReady, setIpReady] = useState(false);
  const [excludeMyIp, setExcludeMyIp] = useState<boolean>(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('lpg_exclude_my_ip') !== 'false'; // デフォルトON
    }
    return true;
  });
  const [excludeServerIp, setExcludeServerIp] = useState<boolean>(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('lpg_exclude_server_ip') !== 'false'; // デフォルトON
    }
    return true;
  });
  const [excludeLan, setExcludeLan] = useState<boolean>(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('lpg_exclude_lan') === 'true'; // デフォルトOFF
    }
    return false;
  });

  // 自分のIP + サーバーのグローバルIP + 全履歴IPを取得
  useEffect(() => {
    dashboardApi.getMyIp().then(r => {
      setMyIp(r.ip);
      if (r.server_ip) setServerIp(r.server_ip);
      setServerIpHistory(r.server_ip_history || []);
      setAdminIpHistory(r.admin_ip_history || []);
      setIpReady(true);
    }).catch(() => {
      setIpReady(true); // エラーでもロード続行
    });
  }, []);

  // localStorage 永続化
  useEffect(() => {
    localStorage.setItem('lpg_exclude_my_ip', String(excludeMyIp));
  }, [excludeMyIp]);
  useEffect(() => {
    localStorage.setItem('lpg_exclude_server_ip', String(excludeServerIp));
  }, [excludeServerIp]);
  useEffect(() => {
    localStorage.setItem('lpg_exclude_lan', String(excludeLan));
  }, [excludeLan]);

  // 除外パラメータ構築ヘルパー（全履歴IPをカンマ区切りで送信）
  const buildExclusionParams = useCallback((): IpExclusionParams => {
    const params: IpExclusionParams = {};
    const ipSet = new Set<string>();
    if (excludeMyIp) {
      adminIpHistory.forEach(ip => ipSet.add(ip));
      if (myIp) ipSet.add(myIp);
    }
    if (excludeServerIp) {
      serverIpHistory.forEach(ip => ipSet.add(ip));
      if (serverIp) ipSet.add(serverIp);
    }
    if (ipSet.size > 0) params.exclude_ips = Array.from(ipSet).join(',');
    if (excludeLan) params.exclude_lan = true;
    return params;
  }, [excludeMyIp, excludeServerIp, excludeLan, myIp, serverIp, serverIpHistory, adminIpHistory]);

  // Filter state
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate] = useState('');
  const [method, setMethod] = useState('');
  const [statusRange, setStatusRange] = useState('');
  const [ip, setIp] = useState('');
  const [path, setPath] = useState('');

  const buildSearchParams = useCallback((): AccessLogSearchParams => {
    const exclusion = buildExclusionParams();
    const params: AccessLogSearchParams = {
      limit: PER_PAGE,
      offset: (page - 1) * PER_PAGE,
      ...exclusion,
    };
    if (fromDate) params.from = new Date(fromDate).toISOString();
    if (toDate) params.to = new Date(toDate).toISOString();
    if (method) params.method = method;
    if (statusRange) {
      const [min, max] = statusRange.split('-').map(Number);
      params.status_min = min;
      params.status_max = max;
    }
    if (ip) params.ip = ip;
    if (path) params.path = path;
    return params;
  }, [page, fromDate, toDate, method, statusRange, ip, path, buildExclusionParams]);

  const loadLogs = useCallback(async () => {
    setLoading(true);
    try {
      const result = await dashboardApi.searchAccessLogs(buildSearchParams());
      setLogs(result.logs);
      setTotal(result.total);
    } catch (err) {
      console.error('Failed to search logs:', err);
    } finally {
      setLoading(false);
    }
  }, [buildSearchParams]);

  const loadErrorSummary = useCallback(async () => {
    try {
      const exclusion = buildExclusionParams();
      const params: { from?: string; to?: string } = {};
      if (fromDate) params.from = new Date(fromDate).toISOString();
      if (toDate) params.to = new Date(toDate).toISOString();
      const summary = await dashboardApi.getErrorSummary(params.from, params.to, exclusion);
      setErrorSummary(summary);
    } catch (err) {
      console.error('Failed to load error summary:', err);
    }
  }, [fromDate, toDate, buildExclusionParams]);

  // Operations tab data loader
  const loadOperationLogs = useCallback(async () => {
    setOpLoading(true);
    try {
      const [logs, summary] = await Promise.all([
        operationLogsApi.list({
          operation_type: opTypeFilter || undefined,
          status: opStatusFilter || undefined,
          limit: 50,
        }),
        operationLogsApi.getSummary(),
      ]);
      setOpLogs(logs);
      setOpSummary(summary);
    } catch (err) {
      console.error('Failed to load operation logs:', err);
    } finally {
      setOpLoading(false);
    }
  }, [opTypeFilter, opStatusFilter]);

  const handleSyncTrigger = async (type: 'omada' | 'openwrt' | 'external' | 'ddns') => {
    setSyncRunning(prev => ({ ...prev, [type]: true }));
    try {
      const fn = {
        omada: toolsApi.syncOmada,
        openwrt: toolsApi.syncOpenwrt,
        external: toolsApi.syncExternal,
        ddns: toolsApi.ddnsUpdateAll,
      }[type];
      await fn();
      loadOperationLogs();
    } catch (err) {
      console.error(`Sync ${type} failed:`, err);
    } finally {
      setSyncRunning(prev => ({ ...prev, [type]: false }));
    }
  };

  const handleNetworkTool = async (tool: 'ping' | 'dns') => {
    if (!toolHost.trim()) return;
    setToolRunning(true);
    setToolResult(null);
    try {
      const fn = tool === 'ping' ? toolsApi.ping : toolsApi.dns;
      const result = await fn(toolHost.trim());
      setToolResult(result as unknown as Record<string, unknown>);
      loadOperationLogs();
    } catch (err) {
      setToolResult({ error: err instanceof Error ? err.message : 'Failed' });
    } finally {
      setToolRunning(false);
    }
  };

  useEffect(() => {
    if (!ipReady) return; // IP取得完了まで待つ（race condition防止）
    loadLogs();
  }, [ipReady, loadLogs]);

  useEffect(() => {
    if (activeTab === 'errors') {
      loadErrorSummary();
    }
    if (activeTab === 'operations') {
      loadOperationLogs();
    }
  }, [activeTab, loadErrorSummary, loadOperationLogs]);

  const handleSearch = () => {
    setPage(1);
    loadLogs();
    if (activeTab === 'errors') loadErrorSummary();
  };

  const handleReset = () => {
    setFromDate('');
    setToDate('');
    setMethod('');
    setStatusRange('');
    setIp('');
    setPath('');
    setPage(1);
  };

  const handleExport = async () => {
    try {
      await dashboardApi.exportCsv({ ...buildSearchParams(), limit: 10000, offset: 0 });
    } catch (err) {
      console.error('Failed to export CSV:', err);
    }
  };

  const logColumns = [
    {
      key: 'timestamp',
      header: 'Time',
      render: (log: AccessLog) => (
        <span className="text-sm text-gray-400 whitespace-nowrap">
          {new Date(log.timestamp).toLocaleString()}
        </span>
      ),
    },
    {
      key: 'method',
      header: 'Method',
      render: (log: AccessLog) => (
        <span className="font-mono text-sm">{log.method}</span>
      ),
    },
    {
      key: 'path',
      header: 'Path',
      render: (log: AccessLog) => (
        <code className="text-sm truncate max-w-xs block">{log.path}</code>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      render: (log: AccessLog) => (
        <span className={`font-mono font-bold ${getStatusColor(log.status)}`}>
          {log.status}
        </span>
      ),
    },
    {
      key: 'response_time_ms',
      header: 'Response',
      render: (log: AccessLog) => (
        <span className={`text-sm ${log.response_time_ms > 1000 ? 'text-red-400' : log.response_time_ms > 500 ? 'text-yellow-400' : 'text-gray-400'}`}>
          {log.response_time_ms}ms
        </span>
      ),
    },
    {
      key: 'ip',
      header: 'IP',
      render: (log: AccessLog) => (
        <code className="text-sm">{log.ip}</code>
      ),
    },
    {
      key: 'location',
      header: 'Location',
      render: (log: AccessLog) => (
        <span className="text-sm whitespace-nowrap">
          {log.country_code
            ? `${countryCodeToFlag(log.country_code)} ${log.city || log.country || log.country_code}`
            : '-'}
        </span>
      ),
    },
    {
      key: 'user_agent',
      header: 'User Agent',
      render: (log: AccessLog) => (
        <span className="text-xs text-gray-500 truncate max-w-[200px] block">
          {log.user_agent || '-'}
        </span>
      ),
    },
  ];

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Access Logs</h1>
        <div className="flex gap-2">
          <Button variant="secondary" onClick={handleExport}>
            Export CSV
          </Button>
        </div>
      </div>

      {/* IP Exclusion Filter */}
      <Card className="mb-4">
        <div className="flex flex-wrap items-center gap-6">
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={excludeMyIp}
              onChange={(e) => { setExcludeMyIp(e.target.checked); setPage(1); }}
              className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-500 focus:ring-blue-500 focus:ring-offset-0"
            />
            <span className="text-sm">
              自分のIPを除外
              {myIp && <code className="ml-1 text-xs text-gray-400">({myIp})</code>}
            </span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={excludeServerIp}
              onChange={(e) => { setExcludeServerIp(e.target.checked); setPage(1); }}
              className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-500 focus:ring-blue-500 focus:ring-offset-0"
            />
            <span className="text-sm">
              LPGのグローバルIPを除外
              {serverIp && <code className="ml-1 text-xs text-gray-400">({serverIp})</code>}
            </span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={excludeLan}
              onChange={(e) => { setExcludeLan(e.target.checked); setPage(1); }}
              className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-500 focus:ring-blue-500 focus:ring-offset-0"
            />
            <span className="text-sm">LANアクセスを除外</span>
          </label>
        </div>
      </Card>

      {/* Filter Bar */}
      <Card className="mb-6">
        <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-4">
          <Input
            label="From"
            type="datetime-local"
            value={fromDate}
            onChange={(e) => setFromDate(e.target.value)}
          />
          <Input
            label="To"
            type="datetime-local"
            value={toDate}
            onChange={(e) => setToDate(e.target.value)}
          />
          <Select
            label="Method"
            options={METHOD_OPTIONS}
            value={method}
            onChange={(e) => setMethod(e.target.value)}
          />
          <Select
            label="Status"
            options={STATUS_OPTIONS}
            value={statusRange}
            onChange={(e) => setStatusRange(e.target.value)}
          />
          <Input
            label="IP"
            placeholder="192.168.1.1"
            value={ip}
            onChange={(e) => setIp(e.target.value)}
          />
          <Input
            label="Path"
            placeholder="/api/.*"
            value={path}
            onChange={(e) => setPath(e.target.value)}
          />
        </div>
        <div className="flex gap-2">
          <Button onClick={handleSearch}>Search</Button>
          <Button variant="ghost" onClick={handleReset}>Reset</Button>
        </div>
      </Card>

      {/* Tabs */}
      <div className="flex gap-4 mb-4 border-b border-border">
        <button
          className={`pb-2 px-1 ${activeTab === 'search' ? 'border-b-2 border-blue-500 text-blue-500' : 'text-gray-400'}`}
          onClick={() => setActiveTab('search')}
        >
          Search Results ({total.toLocaleString()})
        </button>
        <button
          className={`pb-2 px-1 ${activeTab === 'errors' ? 'border-b-2 border-blue-500 text-blue-500' : 'text-gray-400'}`}
          onClick={() => setActiveTab('errors')}
        >
          Error Analysis
        </button>
        <button
          className={`pb-2 px-1 ${activeTab === 'operations' ? 'border-b-2 border-blue-500 text-blue-500' : 'text-gray-400'}`}
          onClick={() => setActiveTab('operations')}
        >
          Operations{opSummary ? ` (${opSummary.recent_24h})` : ''}
        </button>
      </div>

      {/* Content */}
      {activeTab === 'search' ? (
        <div className="bg-card border border-border rounded-lg">
          {loading ? (
            <div className="flex items-center justify-center h-32 text-gray-500">Loading...</div>
          ) : (
            <>
              <Table
                columns={logColumns}
                data={logs}
                keyExtractor={(log) => `${log.timestamp}-${log.ip}-${log.path}-${log.status}`}
                emptyMessage="No logs found"
                onRowClick={(log) => setSelectedLog(log)}
              />
              <Pagination
                total={total}
                page={page}
                perPage={PER_PAGE}
                onPageChange={setPage}
              />
            </>
          )}
        </div>
      ) : activeTab === 'errors' ? (
        <div className="space-y-4">
          {errorSummary.length > 0 ? (
            errorSummary.map((es) => (
              <Card key={es.status}>
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <Badge variant={es.status >= 500 ? 'error' : 'warning'}>
                      {es.status}
                    </Badge>
                    <span className="text-lg font-bold">{es.count.toLocaleString()} errors</span>
                  </div>
                </div>
                {es.paths.length > 0 && (
                  <div>
                    <div className="text-sm text-gray-400 mb-1">Most affected paths:</div>
                    <div className="space-y-1">
                      {es.paths.map((p, i) => (
                        <code key={i} className="block text-sm text-blue-400">{p}</code>
                      ))}
                    </div>
                  </div>
                )}
              </Card>
            ))
          ) : (
            <div className="text-center text-gray-500 py-8">No errors in the selected period</div>
          )}
        </div>
      ) : (
        /* Operations Tab */
        <div className="space-y-6">
          {/* Summary Cards */}
          {opSummary && (
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <Card>
                <div className="p-4 text-center">
                  <div className="text-sm text-gray-400">Total</div>
                  <div className="text-2xl font-bold">{opSummary.total}</div>
                </div>
              </Card>
              <Card>
                <div className="p-4 text-center">
                  <div className="text-sm text-gray-400">Last 24h</div>
                  <div className="text-2xl font-bold">{opSummary.recent_24h}</div>
                </div>
              </Card>
              <Card>
                <div className="p-4 text-center">
                  <div className="text-sm text-gray-400">Success (24h)</div>
                  <div className="text-2xl font-bold text-green-400">{opSummary.recent_success}</div>
                </div>
              </Card>
              <Card>
                <div className="p-4 text-center">
                  <div className="text-sm text-gray-400">Errors (24h)</div>
                  <div className="text-2xl font-bold text-red-400">{opSummary.recent_errors}</div>
                </div>
              </Card>
            </div>
          )}

          {/* Sync Triggers */}
          <Card>
            <div className="p-4">
              <h3 className="text-sm font-semibold text-gray-400 mb-3">Manual Sync Triggers</h3>
              <div className="flex flex-wrap gap-3">
                <Button
                  onClick={() => handleSyncTrigger('omada')}
                  disabled={syncRunning.omada}
                  variant="secondary"
                >
                  {syncRunning.omada ? 'Syncing...' : 'Sync Omada'}
                </Button>
                <Button
                  onClick={() => handleSyncTrigger('openwrt')}
                  disabled={syncRunning.openwrt}
                  variant="secondary"
                >
                  {syncRunning.openwrt ? 'Syncing...' : 'Sync OpenWrt'}
                </Button>
                <Button
                  onClick={() => handleSyncTrigger('external')}
                  disabled={syncRunning.external}
                  variant="secondary"
                >
                  {syncRunning.external ? 'Syncing...' : 'Sync External'}
                </Button>
                <Button
                  onClick={() => handleSyncTrigger('ddns')}
                  disabled={syncRunning.ddns}
                  variant="secondary"
                >
                  {syncRunning.ddns ? 'Updating...' : 'Update All DDNS'}
                </Button>
              </div>
            </div>
          </Card>

          {/* Network Diagnostics */}
          <Card>
            <div className="p-4">
              <h3 className="text-sm font-semibold text-gray-400 mb-3">Network Diagnostics</h3>
              <div className="flex items-end gap-3 mb-3">
                <Input
                  label="Hostname / IP"
                  placeholder="example.com or 8.8.8.8"
                  value={toolHost}
                  onChange={(e) => setToolHost(e.target.value)}
                />
                <Button onClick={() => handleNetworkTool('ping')} disabled={toolRunning || !toolHost.trim()}>
                  {toolRunning ? 'Running...' : 'Ping'}
                </Button>
                <Button onClick={() => handleNetworkTool('dns')} disabled={toolRunning || !toolHost.trim()} variant="secondary">
                  {toolRunning ? 'Running...' : 'DNS Lookup'}
                </Button>
              </div>
              {toolResult && (
                <pre className="text-xs bg-gray-900 p-3 rounded overflow-auto max-h-64">
                  {JSON.stringify(toolResult, null, 2)}
                </pre>
              )}
            </div>
          </Card>

          {/* Operation Logs Filter */}
          <div className="flex gap-3 items-end">
            <Select
              label="Type"
              options={[
                { value: '', label: 'All Types' },
                { value: 'sync_omada', label: 'Sync Omada' },
                { value: 'sync_openwrt', label: 'Sync OpenWrt' },
                { value: 'sync_external', label: 'Sync External' },
                { value: 'ddns_update', label: 'DDNS Update' },
                { value: 'ddns_update_all', label: 'DDNS Update All' },
                { value: 'ping', label: 'Ping' },
                { value: 'dns', label: 'DNS' },
              ]}
              value={opTypeFilter}
              onChange={(e) => setOpTypeFilter(e.target.value)}
            />
            <Select
              label="Status"
              options={[
                { value: '', label: 'All Status' },
                { value: 'success', label: 'Success' },
                { value: 'error', label: 'Error' },
                { value: 'running', label: 'Running' },
              ]}
              value={opStatusFilter}
              onChange={(e) => setOpStatusFilter(e.target.value)}
            />
            <Button variant="secondary" onClick={loadOperationLogs}>Refresh</Button>
          </div>

          {/* Operation Logs Table */}
          <div className="bg-card border border-border rounded-lg">
            {opLoading ? (
              <div className="flex items-center justify-center h-32 text-gray-500">Loading...</div>
            ) : (
              <Table
                columns={[
                  {
                    key: 'created_at',
                    header: 'Time',
                    render: (op: OperationLog) => (
                      <span className="text-sm text-gray-400 whitespace-nowrap">
                        {new Date(op.created_at).toLocaleString()}
                      </span>
                    ),
                  },
                  {
                    key: 'operation_type',
                    header: 'Type',
                    render: (op: OperationLog) => (
                      <code className="text-sm">{op.operation_type}</code>
                    ),
                  },
                  {
                    key: 'initiated_by',
                    header: 'Initiated By',
                    render: (op: OperationLog) => (
                      <Badge variant={op.initiated_by === 'manual' ? 'info' : 'default'}>
                        {op.initiated_by}
                      </Badge>
                    ),
                  },
                  {
                    key: 'target',
                    header: 'Target',
                    render: (op: OperationLog) => (
                      <span className="text-sm">{op.target || '-'}</span>
                    ),
                  },
                  {
                    key: 'status',
                    header: 'Status',
                    render: (op: OperationLog) => (
                      <Badge variant={op.status === 'success' ? 'success' : op.status === 'error' ? 'error' : 'warning'}>
                        {op.status}
                      </Badge>
                    ),
                  },
                  {
                    key: 'duration_ms',
                    header: 'Duration',
                    render: (op: OperationLog) => (
                      <span className="text-sm text-gray-400">
                        {op.duration_ms !== undefined ? `${op.duration_ms}ms` : '-'}
                      </span>
                    ),
                  },
                  {
                    key: 'actions',
                    header: 'Detail',
                    render: (op: OperationLog) => (
                      <Button size="sm" variant="ghost" onClick={() => setOpDetailLog(op)}>
                        View
                      </Button>
                    ),
                  },
                ]}
                data={opLogs}
                keyExtractor={(op) => op.operation_id}
                emptyMessage="No operation logs"
              />
            )}
          </div>
        </div>
      )}

      {/* Operation Detail Modal */}
      <Modal isOpen={!!opDetailLog} onClose={() => setOpDetailLog(null)} title={`Operation: ${opDetailLog?.operation_type || ''}`}>
        {opDetailLog && (
          <div className="space-y-3">
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div className="text-gray-400">ID</div>
              <div><code className="text-xs">{opDetailLog.operation_id}</code></div>
              <div className="text-gray-400">Type</div>
              <div>{opDetailLog.operation_type}</div>
              <div className="text-gray-400">Initiated By</div>
              <div>{opDetailLog.initiated_by}</div>
              <div className="text-gray-400">Target</div>
              <div>{opDetailLog.target || '-'}</div>
              <div className="text-gray-400">Status</div>
              <div>
                <Badge variant={opDetailLog.status === 'success' ? 'success' : opDetailLog.status === 'error' ? 'error' : 'warning'}>
                  {opDetailLog.status}
                </Badge>
              </div>
              <div className="text-gray-400">Duration</div>
              <div>{opDetailLog.duration_ms !== undefined ? `${opDetailLog.duration_ms}ms` : '-'}</div>
              <div className="text-gray-400">Created</div>
              <div>{new Date(opDetailLog.created_at).toLocaleString()}</div>
            </div>
            {opDetailLog.error && (
              <div>
                <div className="text-sm text-red-400 mb-1">Error:</div>
                <pre className="text-xs bg-red-900/30 p-2 rounded">{opDetailLog.error}</pre>
              </div>
            )}
            {opDetailLog.result && (
              <div>
                <div className="text-sm text-gray-400 mb-1">Result:</div>
                <pre className="text-xs bg-gray-900 p-3 rounded overflow-auto max-h-64">
                  {JSON.stringify(opDetailLog.result, null, 2)}
                </pre>
              </div>
            )}
          </div>
        )}
      </Modal>

      <LogDetailModal log={selectedLog} onClose={() => setSelectedLog(null)} />
    </div>
  );
}
