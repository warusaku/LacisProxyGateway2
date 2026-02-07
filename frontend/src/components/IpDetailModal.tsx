'use client';

import { useEffect, useState } from 'react';
import { Modal } from '@/components/ui/Modal';
import { Table } from '@/components/ui/Table';
import { getStatusColor } from '@/lib/format';
import { countryCodeToFlag } from '@/lib/geo';
import { dashboardApi } from '@/lib/api';
import type { TopEntry, AccessLog } from '@/types';

interface IpDetailModalProps {
  entry: TopEntry | null;
  onClose: () => void;
}

export function IpDetailModal({ entry, onClose }: IpDetailModalProps) {
  const [recentLogs, setRecentLogs] = useState<AccessLog[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!entry) {
      setRecentLogs([]);
      return;
    }
    setLoading(true);
    dashboardApi
      .getFilteredAccessLog({ limit: 20, ip: entry.key })
      .then(setRecentLogs)
      .catch(() => setRecentLogs([]))
      .finally(() => setLoading(false));
  }, [entry]);

  const logColumns = [
    {
      key: 'timestamp',
      header: 'Time',
      render: (log: AccessLog) => (
        <span className="text-xs text-gray-400 whitespace-nowrap">
          {new Date(log.timestamp).toLocaleString()}
        </span>
      ),
    },
    {
      key: 'method',
      header: 'Method',
      render: (log: AccessLog) => (
        <span className="font-mono text-xs">{log.method}</span>
      ),
    },
    {
      key: 'path',
      header: 'Path',
      render: (log: AccessLog) => (
        <code className="text-xs truncate max-w-[200px] block">{log.path}</code>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      render: (log: AccessLog) => (
        <span className={`font-mono text-xs font-bold ${getStatusColor(log.status)}`}>
          {log.status}
        </span>
      ),
    },
    {
      key: 'response_time_ms',
      header: 'Time',
      render: (log: AccessLog) => (
        <span className="text-xs text-gray-400">{log.response_time_ms}ms</span>
      ),
    },
  ];

  const errorRate = entry && entry.count > 0
    ? ((entry.error_count / entry.count) * 100).toFixed(1)
    : '0';

  return (
    <Modal
      isOpen={entry !== null}
      onClose={onClose}
      title="IP Detail"
      className="max-w-3xl"
    >
      {entry && (
        <div className="space-y-4 text-sm">
          {/* IP Summary */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <span className="text-gray-500 text-xs">IP Address</span>
              <div className="font-mono text-blue-400 text-base">{entry.key}</div>
            </div>
            <div>
              <span className="text-gray-500 text-xs">Location</span>
              <div>
                {entry.country_code
                  ? `${countryCodeToFlag(entry.country_code)} ${entry.city || entry.country || entry.country_code}`
                  : '-'}
              </div>
            </div>
            <div>
              <span className="text-gray-500 text-xs">Requests (24h)</span>
              <div className="text-lg font-bold">{entry.count.toLocaleString()}</div>
            </div>
            <div>
              <span className="text-gray-500 text-xs">Errors / Rate</span>
              <div>
                <span className={entry.error_count > 0 ? 'text-red-400 font-bold' : ''}>
                  {entry.error_count.toLocaleString()}
                </span>
                <span className="text-gray-500 ml-1">({errorRate}%)</span>
              </div>
            </div>
          </div>

          {/* Location Map */}
          {entry.latitude != null && entry.longitude != null && (
            <div className="pt-3 border-t border-border">
              <h3 className="text-xs font-semibold text-gray-400 uppercase mb-2">
                Location Map
              </h3>
              <iframe
                title="IP Location Map"
                width="100%"
                height="200"
                style={{ border: 0, borderRadius: '8px' }}
                loading="lazy"
                referrerPolicy="no-referrer-when-downgrade"
                src={`https://maps.google.com/maps?q=${entry.latitude},${entry.longitude}&z=10&output=embed`}
              />
            </div>
          )}

          {/* Recent Access Logs */}
          <div className="pt-3 border-t border-border">
            <h3 className="text-xs font-semibold text-gray-400 uppercase mb-2">
              Recent Logs (last 20)
            </h3>
            {loading ? (
              <div className="text-gray-500 text-center py-4">Loading...</div>
            ) : (
              <div className="max-h-[400px] overflow-auto">
                <Table
                  columns={logColumns}
                  data={recentLogs}
                  keyExtractor={(log) => `${log.timestamp}-${log.path}-${log.status}`}
                  emptyMessage="No logs found"
                />
              </div>
            )}
          </div>
        </div>
      )}
    </Modal>
  );
}
