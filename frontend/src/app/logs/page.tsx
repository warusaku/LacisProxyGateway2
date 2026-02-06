'use client';

import { useEffect, useState, useCallback } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Card } from '@/components/ui/Card';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { Pagination } from '@/components/ui/Pagination';
import { dashboardApi } from '@/lib/api';
import type { AccessLog, AccessLogSearchParams, ErrorSummary } from '@/types';

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
  const [activeTab, setActiveTab] = useState<'search' | 'errors'>('search');
  const [errorSummary, setErrorSummary] = useState<ErrorSummary[]>([]);

  // Filter state
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate] = useState('');
  const [method, setMethod] = useState('');
  const [statusRange, setStatusRange] = useState('');
  const [ip, setIp] = useState('');
  const [path, setPath] = useState('');

  const buildSearchParams = useCallback((): AccessLogSearchParams => {
    const params: AccessLogSearchParams = {
      limit: PER_PAGE,
      offset: (page - 1) * PER_PAGE,
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
  }, [page, fromDate, toDate, method, statusRange, ip, path]);

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
      const params: { from?: string; to?: string } = {};
      if (fromDate) params.from = new Date(fromDate).toISOString();
      if (toDate) params.to = new Date(toDate).toISOString();
      const summary = await dashboardApi.getErrorSummary(params.from, params.to);
      setErrorSummary(summary);
    } catch (err) {
      console.error('Failed to load error summary:', err);
    }
  }, [fromDate, toDate]);

  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  useEffect(() => {
    if (activeTab === 'errors') {
      loadErrorSummary();
    }
  }, [activeTab, loadErrorSummary]);

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

  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return 'text-green-400';
    if (status >= 300 && status < 400) return 'text-blue-400';
    if (status >= 400 && status < 500) return 'text-yellow-400';
    return 'text-red-400';
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
      ) : (
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
      )}
    </div>
  );
}
