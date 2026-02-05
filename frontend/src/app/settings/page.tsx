'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Card } from '@/components/ui/Card';
import { settingsApi } from '@/lib/api';
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

  useEffect(() => {
    loadSettings();
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

  const getSetting = (key: string) => settings.find((s) => s.setting_key === key);

  const isBooleanSetting = (key: string) =>
    key.includes('enabled') || key.includes('notify_');

  const isChanged = (key: string) => {
    const setting = getSetting(key);
    return (setting?.setting_value || '') !== (editedValues[key] || '');
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
      </div>
    </div>
  );
}
