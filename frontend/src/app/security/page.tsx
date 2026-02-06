'use client';

import { useEffect, useState, useCallback } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Modal } from '@/components/ui/Modal';
import { Table } from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { Card } from '@/components/ui/Card';
import { securityApi } from '@/lib/api';
import type { BlockedIp, SecurityEvent, Severity, BlockIpRequest, SecurityEventSearchParams } from '@/types';

const SEVERITY_OPTIONS = [
  { value: '', label: 'All Severities' },
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
  { value: 'critical', label: 'Critical' },
];

const EVENT_TYPE_OPTIONS = [
  { value: '', label: 'All Types' },
  { value: 'ip_blocked', label: 'IP Blocked' },
  { value: 'rate_limit_exceeded', label: 'Rate Limit' },
  { value: 'suspicious_activity', label: 'Suspicious' },
  { value: 'ddns_failure', label: 'DDNS Failure' },
  { value: 'health_check_failure', label: 'Health Failure' },
];

export default function SecurityPage() {
  const [blockedIps, setBlockedIps] = useState<BlockedIp[]>([]);
  const [events, setEvents] = useState<SecurityEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [formData, setFormData] = useState<BlockIpRequest>({
    ip: '',
    reason: '',
  });
  const [error, setError] = useState('');
  const [activeTab, setActiveTab] = useState<'blocked' | 'events'>('blocked');

  // Event filter state
  const [filterFromDate, setFilterFromDate] = useState('');
  const [filterToDate, setFilterToDate] = useState('');
  const [filterSeverity, setFilterSeverity] = useState('');
  const [filterEventType, setFilterEventType] = useState('');
  const [filterIp, setFilterIp] = useState('');

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      const [ipsData, eventsData] = await Promise.all([
        securityApi.listBlockedIps(),
        securityApi.listEvents(100),
      ]);
      setBlockedIps(ipsData);
      setEvents(eventsData);
    } catch (err) {
      console.error('Failed to load security data:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleEventSearch = useCallback(async () => {
    try {
      const params: SecurityEventSearchParams = { limit: 100 };
      if (filterFromDate) params.from = new Date(filterFromDate).toISOString();
      if (filterToDate) params.to = new Date(filterToDate).toISOString();
      if (filterSeverity) params.severity = filterSeverity;
      if (filterEventType) params.event_type = filterEventType;
      if (filterIp) params.ip = filterIp;
      const results = await securityApi.searchEvents(params);
      setEvents(results);
    } catch (err) {
      console.error('Failed to search events:', err);
    }
  }, [filterFromDate, filterToDate, filterSeverity, filterEventType, filterIp]);

  const handleEventFilterReset = () => {
    setFilterFromDate('');
    setFilterToDate('');
    setFilterSeverity('');
    setFilterEventType('');
    setFilterIp('');
    loadData();
  };

  const handleBlockIp = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    try {
      await securityApi.blockIp(formData);
      setIsModalOpen(false);
      setFormData({ ip: '', reason: '' });
      loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to block IP');
    }
  };

  const handleUnblock = async (id: number) => {
    if (!confirm('Are you sure you want to unblock this IP?')) return;

    try {
      await securityApi.unblockIp(id);
      loadData();
    } catch (err) {
      console.error('Failed to unblock IP:', err);
    }
  };

  const getSeverityBadge = (severity: Severity) => {
    switch (severity) {
      case 'low':
        return <Badge variant="info">Low</Badge>;
      case 'medium':
        return <Badge variant="warning">Medium</Badge>;
      case 'high':
        return <Badge variant="error">High</Badge>;
      case 'critical':
        return <Badge variant="error">Critical</Badge>;
    }
  };

  const getEventTypeName = (type: string) => {
    switch (type) {
      case 'ip_blocked':
        return 'IP Blocked';
      case 'rate_limit_exceeded':
        return 'Rate Limit';
      case 'suspicious_activity':
        return 'Suspicious';
      case 'ddns_failure':
        return 'DDNS Failure';
      case 'health_check_failure':
        return 'Health Failure';
      default:
        return type;
    }
  };

  const blockedIpColumns = [
    {
      key: 'ip',
      header: 'IP Address',
      render: (ip: BlockedIp) => <code className="text-red-400">{ip.ip}</code>,
    },
    {
      key: 'reason',
      header: 'Reason',
      render: (ip: BlockedIp) => (
        <span className="text-sm text-gray-400">{ip.reason || '-'}</span>
      ),
    },
    {
      key: 'blocked_by',
      header: 'Blocked By',
      render: (ip: BlockedIp) => (
        <Badge variant={ip.blocked_by === 'auto' ? 'warning' : 'default'}>
          {ip.blocked_by}
        </Badge>
      ),
    },
    {
      key: 'created_at',
      header: 'Blocked At',
      render: (ip: BlockedIp) => (
        <span className="text-sm text-gray-400">
          {new Date(ip.created_at).toLocaleString()}
        </span>
      ),
    },
    {
      key: 'expires_at',
      header: 'Expires',
      render: (ip: BlockedIp) => (
        <span className="text-sm text-gray-400">
          {ip.expires_at ? new Date(ip.expires_at).toLocaleString() : 'Never'}
        </span>
      ),
    },
    {
      key: 'actions',
      header: 'Actions',
      render: (ip: BlockedIp) => (
        <Button size="sm" variant="ghost" onClick={() => handleUnblock(ip.id)}>
          Unblock
        </Button>
      ),
    },
  ];

  const eventColumns = [
    {
      key: 'timestamp',
      header: 'Time',
      render: (event: SecurityEvent) => (
        <span className="text-sm text-gray-400">
          {new Date(event.timestamp).toLocaleString()}
        </span>
      ),
    },
    {
      key: 'event_type',
      header: 'Type',
      render: (event: SecurityEvent) => getEventTypeName(event.event_type),
    },
    {
      key: 'severity',
      header: 'Severity',
      render: (event: SecurityEvent) => getSeverityBadge(event.severity),
    },
    {
      key: 'ip',
      header: 'IP',
      render: (event: SecurityEvent) => (
        <code className="text-sm">{event.ip || '-'}</code>
      ),
    },
    {
      key: 'details',
      header: 'Details',
      render: (event: SecurityEvent) => (
        <span className="text-sm text-gray-400 truncate max-w-xs block">
          {JSON.stringify(event.details)}
        </span>
      ),
    },
  ];

  if (loading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>;
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Security</h1>
        <Button onClick={() => setIsModalOpen(true)}>Block IP</Button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-red-400">{blockedIps.length}</div>
            <div className="text-sm text-gray-400">Blocked IPs</div>
          </div>
        </Card>
        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold text-yellow-400">
              {events.filter((e) => e.severity === 'high' || e.severity === 'critical').length}
            </div>
            <div className="text-sm text-gray-400">High Severity Events (24h)</div>
          </div>
        </Card>
        <Card>
          <div className="text-center">
            <div className="text-3xl font-bold">{events.length}</div>
            <div className="text-sm text-gray-400">Total Events</div>
          </div>
        </Card>
      </div>

      {/* Tabs */}
      <div className="flex gap-4 mb-4 border-b border-border">
        <button
          className={`pb-2 px-1 ${activeTab === 'blocked' ? 'border-b-2 border-blue-500 text-blue-500' : 'text-gray-400'}`}
          onClick={() => setActiveTab('blocked')}
        >
          Blocked IPs ({blockedIps.length})
        </button>
        <button
          className={`pb-2 px-1 ${activeTab === 'events' ? 'border-b-2 border-blue-500 text-blue-500' : 'text-gray-400'}`}
          onClick={() => setActiveTab('events')}
        >
          Security Events ({events.length})
        </button>
      </div>

      {/* Content */}
      {activeTab === 'blocked' ? (
        <div className="bg-card border border-border rounded-lg">
          <Table
            columns={blockedIpColumns}
            data={blockedIps}
            keyExtractor={(ip) => ip.id}
            emptyMessage="No blocked IPs"
          />
        </div>
      ) : (
        <>
          {/* Event Filters */}
          <Card className="mb-4">
            <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-5 gap-4 mb-4">
              <Input
                label="From"
                type="datetime-local"
                value={filterFromDate}
                onChange={(e) => setFilterFromDate(e.target.value)}
              />
              <Input
                label="To"
                type="datetime-local"
                value={filterToDate}
                onChange={(e) => setFilterToDate(e.target.value)}
              />
              <Select
                label="Severity"
                options={SEVERITY_OPTIONS}
                value={filterSeverity}
                onChange={(e) => setFilterSeverity(e.target.value)}
              />
              <Select
                label="Event Type"
                options={EVENT_TYPE_OPTIONS}
                value={filterEventType}
                onChange={(e) => setFilterEventType(e.target.value)}
              />
              <Input
                label="IP"
                placeholder="192.168.1.1"
                value={filterIp}
                onChange={(e) => setFilterIp(e.target.value)}
              />
            </div>
            <div className="flex gap-2">
              <Button onClick={handleEventSearch}>Search</Button>
              <Button variant="ghost" onClick={handleEventFilterReset}>Reset</Button>
            </div>
          </Card>
          <div className="bg-card border border-border rounded-lg">
            <Table
              columns={eventColumns}
              data={events}
              keyExtractor={(e) => `${e.timestamp}-${e.event_type}-${e.ip || 'none'}`}
              emptyMessage="No security events"
            />
          </div>
        </>
      )}

      {/* Block IP Modal */}
      <Modal isOpen={isModalOpen} onClose={() => setIsModalOpen(false)} title="Block IP Address">
        <form onSubmit={handleBlockIp} className="space-y-4">
          <Input
            label="IP Address"
            placeholder="192.168.1.100"
            value={formData.ip}
            onChange={(e) => setFormData({ ...formData, ip: e.target.value })}
            required
          />
          <Input
            label="Reason (optional)"
            placeholder="Suspicious activity"
            value={formData.reason}
            onChange={(e) => setFormData({ ...formData, reason: e.target.value })}
          />
          {error && <p className="text-red-500 text-sm">{error}</p>}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={() => setIsModalOpen(false)}>
              Cancel
            </Button>
            <Button type="submit" variant="danger">
              Block IP
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}
