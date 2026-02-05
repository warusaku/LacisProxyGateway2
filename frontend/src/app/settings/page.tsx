'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Card } from '@/components/ui/Card';
import { settingsApi, RestartSettings } from '@/lib/api';
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

  useEffect(() => {
    loadSettings();
    loadRestartSettings();
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
      </div>
    </div>
  );
}
