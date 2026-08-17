import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

export type ReminderChannel = 'app' | 'feishu';

export interface ReminderSettings {
  enabled: boolean;
  /** Local wall-clock times as `HH:MM`. */
  times: string[];
  channels: ReminderChannel[];
  stop_when_all_done: boolean;
}

/** Mirrors the backend default so first paint matches what's stored. */
const DEFAULT_SETTINGS: ReminderSettings = {
  enabled: false,
  times: ['14:00', '15:00', '20:00'],
  channels: ['app', 'feishu'],
  stop_when_all_done: true,
};

export function useReminderSettings() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<ReminderSettings>(DEFAULT_SETTINGS);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  // Distinguishes "no reminders configured" from "couldn't reach the backend".
  // Without this the UI would render an empty, apparently-valid form after a
  // load failure and a subsequent save would wipe the real settings.
  const [loadFailed, setLoadFailed] = useState(false);

  const loadSettings = async () => {
    try {
      setIsLoading(true);
      const data = await invoke<ReminderSettings>('get_reminder_settings');
      setSettings(data);
      setLoadFailed(false);
    } catch (error) {
      console.error('Failed to load reminder settings:', error);
      setLoadFailed(true);
      toast.error(t('settings.reminderLoadFailed', '加载提醒设置失败'));
    } finally {
      setIsLoading(false);
    }
  };

  const saveSettings = async (next: ReminderSettings) => {
    try {
      setIsSaving(true);
      const result = await invoke<ReminderSettings>('update_reminder_settings', {
        input: next,
      });
      // Trust the backend's response: it sorts and de-dupes times, so echoing
      // the local value back would briefly show an order the server rejected.
      setSettings(result);
      toast.success(t('settings.reminderSaveSuccess', '提醒设置已保存'));
      return true;
    } catch (error) {
      console.error('Failed to save reminder settings:', error);
      const message =
        error instanceof Error
          ? error.message
          : typeof error === 'string'
            ? error
            : JSON.stringify(error);
      toast.error(
        t('settings.reminderSaveFailed', { message, defaultValue: `保存提醒设置失败: ${message}` })
      );
      return false;
    } finally {
      setIsSaving(false);
    }
  };

  const updateField = <K extends keyof ReminderSettings>(
    field: K,
    value: ReminderSettings[K]
  ) => {
    setSettings((prev) => ({ ...prev, [field]: value }));
  };

  const addTime = (time: string) => {
    setSettings((prev) =>
      prev.times.includes(time)
        ? prev
        : { ...prev, times: [...prev.times, time].sort() }
    );
  };

  const removeTime = (time: string) => {
    setSettings((prev) => ({
      ...prev,
      times: prev.times.filter((t) => t !== time),
    }));
  };

  const toggleChannel = (channel: ReminderChannel) => {
    setSettings((prev) => ({
      ...prev,
      channels: prev.channels.includes(channel)
        ? prev.channels.filter((c) => c !== channel)
        : [...prev.channels, channel],
    }));
  };

  useEffect(() => {
    loadSettings();
  }, []);

  return {
    settings,
    isLoading,
    isSaving,
    loadFailed,
    updateField,
    addTime,
    removeTime,
    toggleChannel,
    saveSettings,
    reload: loadSettings,
  };
}
