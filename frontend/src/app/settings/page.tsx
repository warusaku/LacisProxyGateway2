'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Card } from '@/components/ui/Card';
import { settingsApi, auditApi, nginxApi, RestartSettings, AuditLog, NginxStatus } from '@/lib/api';
import type { Setting } from '@/types';

interface SettingGroup {
  title: string;
  description: string;
  settings: string[];
}

const settingGroups: SettingGroup[] = [
  {
    title: 'Discord Notifications',
    description: 'Configure Discord webhook for notifications',
    settings: [
      'discord_webhook_url',
      'discord_notify_security',
      'discord_notify_health',
      'discord_notify_ddns',
    ],
  },
  {
    title: 'Rate Limiting',
    description: 'Configure rate limiting settings',
    settings: ['rate_limit_enabled', 'rate_limit_requests_per_minute'],
  },
  {
    title: 'Health Check',
    description: 'Configure health check settings',
    settings: [
      'health_check_interval_sec',
      'health_check_timeout_ms',
      'health_check_failure_threshold',
    ],
  },
  {
    title: 'Logging',
    description: 'Configure log retention',
    settings: ['access_log_retention_days'],
  },
];

const settingLabels: Record<string, string> = {
  discord_webhook_url: 'Webhook URL',
  discord_notify_security: 'Notify Security Events',
  discord_notify_health: 'Notify Health Check Failures',
  discord_notify_ddns: 'Notify DDNS Updates',
  rate_limit_enabled: 'Enable Rate Limiting',
  rate_limit_requests_per_minute: 'Requests per Minute',
  health_check_interval_sec: 'Check Interval (seconds)',
  health_check_timeout_ms: 'Timeout (ms)',
  health_check_failure_threshold: 'Failure Threshold',
  access_log_retention_days: 'Retention Days',
};

export default function SettingsPage() {
  const [settings, setSettings] = useState<Setting[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);
  const [editedValues, setEditedValues] = useState<Record<string, string>>({});
  const [testingDiscord, setTestingDiscord] = useState(false);

  // Restart scheduler state
  const [restartSettings, setRestartSettings] = useState<RestartSettings | null>(null);
  const [restartLoading, setRestartLoading] = useState(true);
  const [restartSaving, setRestartSaving] = useState(false);
  const [triggeringRestart, setTriggeringRestart] = useState(false);
  const [editedRestartSettings, setEditedRestartSettings] = useState<RestartSettings | null>(null);

  // Audit log state
  const [auditLogs, setAuditLogs] = useState<AuditLog[]>([]);
  const [auditLoading, setAuditLoading] = useState(true);

  // Nginx state
  const [nginxStatus, setNginxStatus] = useState<NginxStatus | null>(null);
  const [nginxLoading, setNginxLoading] = useState(true);
  const [enablingFullProxy, setEnablingFullProxy] = useState(false);
  const [reloadingNginx, setReloadingNginx] = useState(false);
  const [serverName, setServerName] = useState('');
  const [backendPort, setBackendPort] = useState('8080');

  useEffect(() => {
    loadSettings();
    loadRestartSettings();
    loadAuditLogs();
    loadNginxStatus();
  }, []);

  const loadSettings = async () => {
    try {
      const data = await settingsApi.list();
      setSettings(data);
      // Initialize edited values
      const values: Record<string, string> = {};
      data.forEach((s) => {
        values[s.setting_key] = s.setting_value || '';
      });
      setEditedValues(values);
    } catch (err) {
      console.error('Failed to load settings:', err);
    } finally {
      setLoading(false);
    }
  };

  const loadRestartSettings = async () => {
    try {
      const data = await settingsApi.getRestartSettings();
      setRestartSettings(data);
      setEditedRestartSettings(data);
    } catch (err) {
      console.error('Failed to load restart settings:', err);
    } finally {
      setRestartLoading(false);
    }
  };

  const loadAuditLogs = async () => {
    try {
      const data = await auditApi.getLogs(20);
      setAuditLogs(data);
    } catch (err) {
      console.error('Failed to load audit logs:', err);
    } finally {
      setAuditLoading(false);
    }
  };

  const loadNginxStatus = async () => {
    try {
      const data = await nginxApi.getStatus();
      setNginxStatus(data);
    } catch (err) {
      console.error('Failed to load nginx status:', err);
    } finally {
      setNginxLoading(false);
    }
  };

  const handleEnableFullProxy = async () => {
    if (!serverName) {
      alert('Please enter the server name (domain)');
      return;
    }
    if (!confirm(`This will configure nginx to route ALL traffic through LacisProxyGateway2.\n\nServer: ${serverName}\nBackend Port: ${backendPort}\n\nContinue?`)) {
      return;
    }
    setEnablingFullProxy(true);
    try {
      await nginxApi.enableFullProxy({
        enable_full_proxy: true,
        server_name: serverName,
        backend_port: parseInt(backendPort) || 8080,
      });
      alert('Full proxy mode enabled! All routes are now managed through the UI.');
      await loadNginxStatus();
    } catch (err) {
      alert('Failed to enable full proxy: ' + (err instanceof Error ? err.message : 'Unknown error'));
    } finally {
      setEnablingFullProxy(false);
    }
  };

  const handleReloadNginx = async () => {
    setReloadingNginx(true);
    try {
      await nginxApi.reload();
      alert('Nginx reloaded successfully!');
      await loadNginxStatus();
    } catch (err) {
      alert('Failed to reload nginx: ' + (err instanceof Error ? err.message : 'Unknown error'));
    } finally {
      setReloadingNginx(false);
    }
  };

  const handleSave = async (key: string) => {
    setSaving(key);
    try {
      const value = editedValues[key];
      await settingsApi.update(key, value || null);
      await loadSettings();
    } catch (err) {
      console.error('Failed to save setting:', err);
    } finally {
      setSaving(null);
    }
  };

  const handleTestDiscord = async () => {
    setTestingDiscord(true);
    try {
      await settingsApi.testDiscord();
      alert('Test notification sent successfully!');
    } catch (err) {
      alert('Failed to send test notification: ' + (err instanceof Error ? err.message : 'Unknown error'));
    } finally {
      setTestingDiscord(false);
    }
  };

  const handleSaveRestartSettings = async () => {
    if (!editedRestartSettings) return;
    setRestartSaving(true);
    try {
      await settingsApi.updateRestartSettings(editedRestartSettings);
      await loadRestartSettings();
      alert('Restart settings saved successfully!');
    } catch (err) {
      alert('Failed to save restart settings: ' + (err instanceof Error ? err.message : 'Unknown error'));
    } finally {
      setRestartSaving(false);
    }
  };

  const handleTriggerRestart = async () => {
    if (!confirm('Are you sure you want to restart the server? All connections will be terminated.')) {
      return;
    }
    setTriggeringRestart(true);
    try {
      await settingsApi.triggerRestart();
      alert('Restart initiated. The server will reboot shortly.');
    } catch (err) {
      alert('Failed to trigger restart: ' + (err instanceof Error ? err.message : 'Unknown error'));
    } finally {
      setTriggeringRestart(false);
    }
  };

  const getSetting = (key: string) => settings.find((s) => s.setting_key === key);

  const isBooleanSetting = (key: string) =>
    key.includes('enabled') || key.includes('notify_');

  const isChanged = (key: string) => {
    const setting = getSetting(key);
    return (setting?.setting_value || '') !== (editedValues[key] || '');
  };

  const isRestartSettingsChanged = () => {
    if (!restartSettings || !editedRestartSettings) return false;
    return (
      restartSettings.scheduled_enabled !== editedRestartSettings.scheduled_enabled ||
      restartSettings.scheduled_time !== editedRestartSettings.scheduled_time ||
      restartSettings.auto_restart_enabled !== editedRestartSettings.auto_restart_enabled ||
      restartSettings.cpu_threshold !== editedRestartSettings.cpu_threshold ||
      restartSettings.ram_threshold !== editedRestartSettings.ram_threshold
    );
  };

  const renderSettingInput = (key: string) => {
    if (isBooleanSetting(key)) {
      return (
        <div className="flex items-center gap-4">
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={editedValues[key] === 'true'}
              onChange={(e) =>
                setEditedValues({ ...editedValues, [key]: e.target.checked ? 'true' : 'false' })
              }
              className="rounded"
            />
            <span className="text-sm">{editedValues[key] === 'true' ? 'Enabled' : 'Disabled'}</span>
          </label>
          {isChanged(key) && (
            <Button size="sm" onClick={() => handleSave(key)} loading={saving === key}>
              Save
            </Button>
          )}
        </div>
      );
    }

    return (
      <div className="flex gap-2">
        <Input
          type={key.includes('url') ? 'text' : 'number'}
          value={editedValues[key] || ''}
          onChange={(e) => setEditedValues({ ...editedValues, [key]: e.target.value })}
          placeholder={key.includes('url') ? 'https://discord.com/api/webhooks/...' : ''}
          className="flex-1"
        />
        {isChanged(key) && (
          <Button onClick={() => handleSave(key)} loading={saving === key}>
            Save
          </Button>
        )}
      </div>
    );
  };

  if (loading) {
    return <div className="flex items-center justify-center h-64">Loading...</div>;
  }

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Settings</h1>

      <div className="space-y-6">
        {settingGroups.map((group) => (
          <Card key={group.title} title={group.title}>
            <p className="text-sm text-gray-400 mb-4">{group.description}</p>
            <div className="space-y-4">
              {group.settings.map((key) => (
                <div key={key} className="flex flex-col gap-1">
                  <label className="text-sm font-medium text-gray-300">
                    {settingLabels[key] || key}
                  </label>
                  {renderSettingInput(key)}
                </div>
              ))}
            </div>
            {group.title === 'Discord Notifications' && (
              <div className="mt-4 pt-4 border-t border-border">
                <Button
                  variant="secondary"
                  onClick={handleTestDiscord}
                  loading={testingDiscord}
                >
                  Send Test Notification
                </Button>
              </div>
            )}
          </Card>
        ))}

        {/* Nginx Configuration Section */}
        <Card title="Nginx Configuration">
          <p className="text-sm text-gray-400 mb-4">
            Configure nginx to route all traffic through LacisProxyGateway2. This enables full UI-based route management.
          </p>

          {nginxLoading ? (
            <div className="text-center py-4">Loading nginx status...</div>
          ) : nginxStatus ? (
            <div className="space-y-6">
              {/* Status */}
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="p-3 bg-gray-800/50 rounded-lg">
                  <div className="text-sm text-gray-400">Status</div>
                  <div className={`font-bold ${nginxStatus.running ? 'text-green-400' : 'text-red-400'}`}>
                    {nginxStatus.running ? 'Running' : 'Stopped'}
                  </div>
                </div>
                <div className="p-3 bg-gray-800/50 rounded-lg">
                  <div className="text-sm text-gray-400">Config Valid</div>
                  <div className={`font-bold ${nginxStatus.config_valid ? 'text-green-400' : 'text-red-400'}`}>
                    {nginxStatus.config_valid ? 'Valid' : 'Invalid'}
                  </div>
                </div>
                <div className="p-3 bg-gray-800/50 rounded-lg">
                  <div className="text-sm text-gray-400">Proxy Mode</div>
                  <div className={`font-bold ${nginxStatus.proxy_mode === 'full_proxy' ? 'text-green-400' : 'text-yellow-400'}`}>
                    {nginxStatus.proxy_mode === 'full_proxy' ? 'Full Proxy' : 'Selective'}
                  </div>
                </div>
                <div className="p-3 bg-gray-800/50 rounded-lg">
                  <div className="text-sm text-gray-400">Config Path</div>
                  <div className="font-mono text-xs truncate">
                    {nginxStatus.config_path || 'Not found'}
                  </div>
                </div>
              </div>

              {/* Warning for Selective Mode */}
              {nginxStatus.proxy_mode !== 'full_proxy' && (
                <div className="p-4 bg-yellow-900/30 border border-yellow-600 rounded-lg">
                  <h4 className="font-bold text-yellow-400 mb-2">⚠️ Selective Mode Active</h4>
                  <p className="text-sm text-gray-300 mb-3">
                    Currently, nginx only forwards specific paths to LacisProxyGateway2. 
                    New routes added through the UI may not work until nginx is configured for full proxy mode.
                  </p>
                  <p className="text-sm text-gray-400">
                    Enable &quot;Full Proxy Mode&quot; below to route all traffic through LacisProxyGateway2, 
                    allowing complete UI-based route management.
                  </p>
                </div>
              )}

              {/* Success for Full Proxy Mode */}
              {nginxStatus.proxy_mode === 'full_proxy' && (
                <div className="p-4 bg-green-900/30 border border-green-600 rounded-lg">
                  <h4 className="font-bold text-green-400 mb-2">✓ Full Proxy Mode Active</h4>
                  <p className="text-sm text-gray-300">
                    All traffic is routed through LacisProxyGateway2. 
                    You can manage all routes through the Routes page.
                  </p>
                </div>
              )}

              {/* Enable Full Proxy Form */}
              {nginxStatus.proxy_mode !== 'full_proxy' && (
                <div className="p-4 bg-gray-800/50 rounded-lg">
                  <h3 className="text-lg font-medium mb-3">Enable Full Proxy Mode</h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                    <div>
                      <label className="text-sm text-gray-400 block mb-1">Server Name (Domain)</label>
                      <Input
                        type="text"
                        value={serverName}
                        onChange={(e) => setServerName(e.target.value)}
                        placeholder="akbdevs.dnsalias.com"
                      />
                    </div>
                    <div>
                      <label className="text-sm text-gray-400 block mb-1">Backend Port</label>
                      <Input
                        type="number"
                        value={backendPort}
                        onChange={(e) => setBackendPort(e.target.value)}
                        placeholder="8080"
                      />
                    </div>
                  </div>
                  <Button
                    onClick={handleEnableFullProxy}
                    loading={enablingFullProxy}
                    disabled={!serverName}
                  >
                    Enable Full Proxy Mode
                  </Button>
                </div>
              )}

              {/* Reload Button */}
              <div className="flex gap-3">
                <Button
                  variant="secondary"
                  onClick={handleReloadNginx}
                  loading={reloadingNginx}
                >
                  Reload Nginx
                </Button>
                <Button
                  variant="ghost"
                  onClick={loadNginxStatus}
                >
                  Refresh Status
                </Button>
              </div>

              {/* Error Display */}
              {nginxStatus.error && (
                <div className="p-4 bg-red-900/30 border border-red-600 rounded-lg">
                  <h4 className="font-bold text-red-400 mb-2">Error</h4>
                  <pre className="text-sm text-gray-300 whitespace-pre-wrap">{nginxStatus.error}</pre>
                </div>
              )}
            </div>
          ) : (
            <div className="text-red-400">Failed to load nginx status</div>
          )}
        </Card>

        {/* Restart Scheduler Section */}
        <Card title="Restart Scheduler">
          <p className="text-sm text-gray-400 mb-4">
            Configure automatic server restart based on schedule or resource usage
          </p>

          {restartLoading ? (
            <div className="text-center py-4">Loading restart settings...</div>
          ) : editedRestartSettings ? (
            <div className="space-y-6">
              {/* Scheduled Restart */}
              <div className="p-4 bg-gray-800/50 rounded-lg">
                <h3 className="text-lg font-medium mb-3">Scheduled Restart</h3>
                <div className="space-y-3">
                  <label className="flex items-center gap-3">
                    <input
                      type="checkbox"
                      checked={editedRestartSettings.scheduled_enabled}
                      onChange={(e) =>
                        setEditedRestartSettings({
                          ...editedRestartSettings,
                          scheduled_enabled: e.target.checked,
                        })
                      }
                      className="rounded w-5 h-5"
                    />
                    <span>Enable Scheduled Restart</span>
                  </label>
                  <div className="flex items-center gap-3">
                    <span className="text-sm text-gray-400 min-w-[100px]">Restart Time:</span>
                    <Input
                      type="time"
                      value={editedRestartSettings.scheduled_time}
                      onChange={(e) =>
                        setEditedRestartSettings({
                          ...editedRestartSettings,
                          scheduled_time: e.target.value,
                        })
                      }
                      disabled={!editedRestartSettings.scheduled_enabled}
                      className="w-32"
                    />
                    <span className="text-sm text-gray-500">Daily at this time</span>
                  </div>
                </div>
              </div>

              {/* Auto Restart on High Resource */}
              <div className="p-4 bg-gray-800/50 rounded-lg">
                <h3 className="text-lg font-medium mb-3">Auto Restart on High Resource Usage</h3>
                <div className="space-y-3">
                  <label className="flex items-center gap-3">
                    <input
                      type="checkbox"
                      checked={editedRestartSettings.auto_restart_enabled}
                      onChange={(e) =>
                        setEditedRestartSettings({
                          ...editedRestartSettings,
                          auto_restart_enabled: e.target.checked,
                        })
                      }
                      className="rounded w-5 h-5"
                    />
                    <span>Enable Auto Restart</span>
                  </label>
                  <p className="text-sm text-yellow-500">
                    Warning: This will automatically reboot the server when resource usage exceeds thresholds
                  </p>
                  <div className="grid grid-cols-2 gap-4 mt-3">
                    <div>
                      <label className="text-sm text-gray-400 block mb-1">CPU Threshold (%)</label>
                      <Input
                        type="number"
                        min={50}
                        max={100}
                        value={editedRestartSettings.cpu_threshold}
                        onChange={(e) =>
                          setEditedRestartSettings({
                            ...editedRestartSettings,
                            cpu_threshold: parseInt(e.target.value) || 90,
                          })
                        }
                        disabled={!editedRestartSettings.auto_restart_enabled}
                      />
                    </div>
                    <div>
                      <label className="text-sm text-gray-400 block mb-1">RAM Threshold (%)</label>
                      <Input
                        type="number"
                        min={50}
                        max={100}
                        value={editedRestartSettings.ram_threshold}
                        onChange={(e) =>
                          setEditedRestartSettings({
                            ...editedRestartSettings,
                            ram_threshold: parseInt(e.target.value) || 90,
                          })
                        }
                        disabled={!editedRestartSettings.auto_restart_enabled}
                      />
                    </div>
                  </div>
                </div>
              </div>

              {/* Save Button */}
              <div className="flex gap-3">
                <Button
                  onClick={handleSaveRestartSettings}
                  loading={restartSaving}
                  disabled={!isRestartSettingsChanged()}
                >
                  Save Restart Settings
                </Button>
              </div>

              {/* Manual Restart */}
              <div className="pt-4 border-t border-border">
                <h3 className="text-lg font-medium mb-3">Manual Restart</h3>
                <p className="text-sm text-gray-400 mb-3">
                  Manually trigger a server restart. All connections will be terminated.
                </p>
                <Button
                  variant="danger"
                  onClick={handleTriggerRestart}
                  loading={triggeringRestart}
                >
                  Restart Server Now
                </Button>
              </div>
            </div>
          ) : (
            <div className="text-red-400">Failed to load restart settings</div>
          )}
        </Card>

        {/* Audit Log Section */}
        <Card title="Configuration Audit Log">
          <p className="text-sm text-gray-400 mb-4">
            Recent configuration changes (routes, settings, etc.)
          </p>

          {auditLoading ? (
            <div className="text-center py-4">Loading audit logs...</div>
          ) : auditLogs.length === 0 ? (
            <div className="text-center py-4 text-gray-500">No configuration changes recorded yet</div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-gray-700">
                    <th className="text-left py-2 px-2">Time</th>
                    <th className="text-left py-2 px-2">Type</th>
                    <th className="text-left py-2 px-2">Action</th>
                    <th className="text-left py-2 px-2">Details</th>
                  </tr>
                </thead>
                <tbody>
                  {auditLogs.map((log) => (
                    <tr key={log.id} className="border-b border-gray-800 hover:bg-gray-800/50">
                      <td className="py-2 px-2 text-gray-400 whitespace-nowrap">
                        {log.created_at ? new Date(log.created_at).toLocaleString('ja-JP') : '-'}
                      </td>
                      <td className="py-2 px-2">
                        <span className={`px-2 py-0.5 rounded text-xs ${
                          log.entity_type === 'route' ? 'bg-blue-600' :
                          log.entity_type === 'setting' ? 'bg-purple-600' :
                          log.entity_type === 'ddns' ? 'bg-green-600' :
                          'bg-gray-600'
                        }`}>
                          {log.entity_type}
                        </span>
                      </td>
                      <td className="py-2 px-2">
                        <span className={`px-2 py-0.5 rounded text-xs ${
                          log.action === 'create' ? 'bg-green-700' :
                          log.action === 'update' ? 'bg-yellow-700' :
                          log.action === 'delete' ? 'bg-red-700' :
                          'bg-gray-600'
                        }`}>
                          {log.action}
                        </span>
                      </td>
                      <td className="py-2 px-2 text-gray-300">
                        {log.field_name ? (
                          <span>
                            <span className="text-gray-500">{log.field_name}:</span>{' '}
                            {log.old_value && <span className="text-red-400 line-through">{log.old_value}</span>}
                            {log.old_value && log.new_value && ' → '}
                            {log.new_value && <span className="text-green-400">{log.new_value}</span>}
                          </span>
                        ) : (
                          <span>{log.new_value || log.old_value || '-'}</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          <div className="mt-4 pt-4 border-t border-border">
            <Button variant="secondary" onClick={loadAuditLogs}>
              Refresh
            </Button>
          </div>
        </Card>
      </div>
    </div>
  );
}
