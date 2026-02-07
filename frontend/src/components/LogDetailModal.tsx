'use client';

import { Modal } from '@/components/ui/Modal';
import { getStatusColor } from '@/lib/format';
import { countryCodeToFlag } from '@/lib/geo';
import type { AccessLog } from '@/types';

interface LogDetailModalProps {
  log: AccessLog | null;
  onClose: () => void;
}

export function LogDetailModal({ log, onClose }: LogDetailModalProps) {
  return (
    <Modal
      isOpen={log !== null}
      onClose={onClose}
      title="Log Detail"
      className="max-w-2xl"
    >
      {log && (
        <div className="space-y-4 text-sm">
          {/* Request Info */}
          <div>
            <h3 className="text-xs font-semibold text-gray-400 uppercase mb-2">Request</h3>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <span className="text-gray-500">Time</span>
                <div>{new Date(log.timestamp).toLocaleString()}</div>
              </div>
              <div>
                <span className="text-gray-500">Method</span>
                <div className="font-mono">{log.method}</div>
              </div>
              <div className="col-span-2">
                <span className="text-gray-500">Path</span>
                <div className="font-mono break-all">{log.path}</div>
              </div>
              <div>
                <span className="text-gray-500">Status</span>
                <div className={`font-mono font-bold ${getStatusColor(log.status)}`}>
                  {log.status}
                </div>
              </div>
              <div>
                <span className="text-gray-500">Response Time</span>
                <div>{log.response_time_ms}ms</div>
              </div>
              {log.target && (
                <div className="col-span-2">
                  <span className="text-gray-500">Target</span>
                  <div className="font-mono text-blue-400">{log.target}</div>
                </div>
              )}
            </div>
          </div>

          {/* Client Info */}
          <div className="pt-3 border-t border-border">
            <h3 className="text-xs font-semibold text-gray-400 uppercase mb-2">Client</h3>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <span className="text-gray-500">IP</span>
                <div className="font-mono">{log.ip}</div>
              </div>
              <div>
                <span className="text-gray-500">Referer</span>
                <div className="truncate">{log.referer || '-'}</div>
              </div>
              <div className="col-span-2">
                <span className="text-gray-500">User Agent</span>
                <div className="text-xs break-all text-gray-300">{log.user_agent || '-'}</div>
              </div>
            </div>
          </div>

          {/* GeoIP Info */}
          {log.country_code && (
            <div className="pt-3 border-t border-border">
              <h3 className="text-xs font-semibold text-gray-400 uppercase mb-2">Location</h3>
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <span className="text-gray-500">Country</span>
                  <div>
                    {countryCodeToFlag(log.country_code)}{' '}
                    {log.country || log.country_code}
                  </div>
                </div>
                <div>
                  <span className="text-gray-500">City</span>
                  <div>{log.city || '-'}</div>
                </div>
                {log.latitude != null && log.longitude != null && (
                  <>
                    <div className="col-span-2">
                      <span className="text-gray-500">Coordinates</span>
                      <div className="font-mono text-xs">
                        {log.latitude.toFixed(4)}, {log.longitude.toFixed(4)}
                      </div>
                    </div>
                    <div className="col-span-2 mt-2">
                      <iframe
                        title="Access Location Map"
                        width="100%"
                        height="200"
                        style={{ border: 0, borderRadius: '8px' }}
                        loading="lazy"
                        referrerPolicy="no-referrer-when-downgrade"
                        src={`https://maps.google.com/maps?q=${log.latitude},${log.longitude}&z=10&output=embed`}
                      />
                    </div>
                  </>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}
