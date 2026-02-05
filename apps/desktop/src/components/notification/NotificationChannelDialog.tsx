import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import type { NotificationChannelDto } from '@/types/notification';

interface NotificationChannelDialogProps {
  open: boolean;
  onClose: (success: boolean) => void;
  channel?: NotificationChannelDto | null;
}

type ChannelType = 'feishu' | 'dingtalk' | 'email';

interface FeishuConfig {
  type: 'feishu';
  webhook_key: string;
}

interface DingTalkConfig {
  type: 'dingtalk';
  webhook_key: string;
  secret?: string;
}

interface EmailConfig {
  type: 'email';
  smtp_host: string;
  smtp_port: number;
  username: string;
  password: string;
  from: string;
  to: string[];
}

type ChannelConfig = FeishuConfig | DingTalkConfig | EmailConfig;

export function NotificationChannelDialog({ open, onClose, channel }: NotificationChannelDialogProps) {
  const { t } = useTranslation();
  const [channelType, setChannelType] = useState<ChannelType>('feishu');
  const [webhookKey, setWebhookKey] = useState('');
  const [secret, setSecret] = useState('');
  const [smtpHost, setSmtpHost] = useState('');
  const [smtpPort, setSmtpPort] = useState(465);
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (channel) {
      try {
        const config = JSON.parse(channel.config);
        setChannelType(channel.channel_type as ChannelType);

        if (config.type === 'feishu') {
          setWebhookKey(config.webhook_key || '');
        } else if (config.type === 'dingtalk') {
          setWebhookKey(config.webhook_key || '');
          setSecret(config.secret || '');
        } else if (config.type === 'email') {
          setSmtpHost(config.smtp_host || '');
          setSmtpPort(config.smtp_port || 465);
          setUsername(config.username || '');
          setPassword(config.password || '');
          setFrom(config.from || '');
          setTo((config.to || []).join(', '));
        }
      } catch (err) {
        console.error('Failed to parse channel config:', err);
      }
    } else {
      resetForm();
    }
  }, [channel, open]);

  const resetForm = () => {
    setChannelType('feishu');
    setWebhookKey('');
    setSecret('');
    setSmtpHost('');
    setSmtpPort(465);
    setUsername('');
    setPassword('');
    setFrom('');
    setTo('');
  };

  const buildConfig = (): ChannelConfig => {
    if (channelType === 'feishu') {
      return {
        type: 'feishu',
        webhook_key: webhookKey.trim(),
      };
    } else if (channelType === 'dingtalk') {
      return {
        type: 'dingtalk',
        webhook_key: webhookKey.trim(),
        secret: secret.trim() || undefined,
      };
    } else {
      return {
        type: 'email',
        smtp_host: smtpHost.trim(),
        smtp_port: smtpPort,
        username: username.trim(),
        password: password.trim(),
        from: from.trim(),
        to: to.split(',').map(email => email.trim()).filter(Boolean),
      };
    }
  };

  const validateForm = (): boolean => {
    if (channelType === 'feishu') {
      if (!webhookKey.trim()) {
        toast.error(t('notification.validation.webhookKeyRequired'));
        return false;
      }
    } else if (channelType === 'dingtalk') {
      if (!webhookKey.trim()) {
        toast.error(t('notification.validation.webhookKeyRequired'));
        return false;
      }
    } else if (channelType === 'email') {
      if (!smtpHost.trim()) {
        toast.error(t('notification.validation.smtpHostRequired'));
        return false;
      }
      if (!username.trim()) {
        toast.error(t('notification.validation.usernameRequired'));
        return false;
      }
      if (!password.trim()) {
        toast.error(t('notification.validation.passwordRequired'));
        return false;
      }
      if (!from.trim()) {
        toast.error(t('notification.validation.fromRequired'));
        return false;
      }
      if (!to.trim()) {
        toast.error(t('notification.validation.toRequired'));
        return false;
      }
    }
    return true;
  };

  const handleSave = async () => {
    if (!validateForm()) {
      return;
    }

    setSaving(true);
    try {
      const config = buildConfig();

      if (channel) {
        // Update existing channel
        await invoke('update_notification_channel', {
          input: {
            channel_id: channel.id,
            config: config,
            enabled: null,
          },
        });
        toast.success(t('notification.toast.channelUpdated'));
      } else {
        // Create new channel
        await invoke('create_notification_channel', {
          input: {
            channel_type: channelType,
            config: config,
          },
        });
        toast.success(t('notification.toast.channelCreated'));
      }

      onClose(true);
    } catch (err) {
      toast.error(channel ? t('notification.toast.updateFailed') : t('notification.toast.createFailed'), {
        description: String(err),
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose(false)}>
      <DialogContent className="sm:max-w-[500px] rounded-[var(--radius-popover)]">
        <DialogHeader>
          <DialogTitle>{channel ? t('notification.dialog.editTitle') : t('notification.dialog.createTitle')}</DialogTitle>
          <DialogDescription>
            {t('notification.dialog.description')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Channel Type */}
          <div className="space-y-2">
            <Label>{t('notification.dialog.channelType')}</Label>
            <Select
              value={channelType}
              onValueChange={(value) => setChannelType(value as ChannelType)}
              disabled={!!channel}
            >
              <SelectTrigger className="rounded-lg">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="feishu">{t('notification.dialog.feishuOption')}</SelectItem>
                <SelectItem value="dingtalk">{t('notification.dialog.dingtalkOption')}</SelectItem>
                <SelectItem value="email">{t('notification.dialog.emailOption')}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Feishu Config */}
          {channelType === 'feishu' && (
            <div className="space-y-2">
              <Label htmlFor="webhook-key">{t('notification.dialog.webhookKeyLabel')} *</Label>
              <Input
                id="webhook-key"
                value={webhookKey}
                onChange={(e) => setWebhookKey(e.target.value)}
                placeholder={t('notification.dialog.webhookKeyPlaceholder')}
                className="rounded-lg"
              />
              <p className="text-xs text-muted-foreground">
                {t('notification.dialog.webhookKeyHelp')}<strong>xxx</strong>
                <br />
                {t('notification.dialog.webhookKeyHelpSuffix')}
              </p>
            </div>
          )}

          {/* DingTalk Config */}
          {channelType === 'dingtalk' && (
            <>
              <div className="space-y-2">
                <Label htmlFor="webhook-key-dt">{t('notification.dialog.webhookKeyLabel')} *</Label>
                <Input
                  id="webhook-key-dt"
                  value={webhookKey}
                  onChange={(e) => setWebhookKey(e.target.value)}
                  placeholder={t('notification.dialog.dingtalkWebhookPlaceholder')}
                  className="rounded-lg"
                  disabled
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="secret">{t('notification.dialog.secretLabel')}</Label>
                <Input
                  id="secret"
                  value={secret}
                  onChange={(e) => setSecret(e.target.value)}
                  placeholder={t('notification.dialog.secretPlaceholder')}
                  className="rounded-lg"
                  disabled
                />
              </div>
              <p className="text-xs text-warning">
                {t('notification.dialog.dingtalkComingSoon')}
              </p>
            </>
          )}

          {/* Email Config */}
          {channelType === 'email' && (
            <>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="smtp-host">{t('notification.dialog.smtpHostLabel')} *</Label>
                  <Input
                    id="smtp-host"
                    value={smtpHost}
                    onChange={(e) => setSmtpHost(e.target.value)}
                    placeholder={t('notification.dialog.smtpHostPlaceholder')}
                    className="rounded-lg"
                    disabled
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="smtp-port">{t('notification.dialog.smtpPortLabel')} *</Label>
                  <Input
                    id="smtp-port"
                    type="number"
                    value={smtpPort}
                    onChange={(e) => setSmtpPort(parseInt(e.target.value))}
                    placeholder="465"
                    className="rounded-lg"
                    disabled
                  />
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="username">{t('notification.dialog.usernameLabel')} *</Label>
                <Input
                  id="username"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder={t('notification.dialog.usernamePlaceholder')}
                  className="rounded-lg"
                  disabled
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="password">{t('notification.dialog.passwordLabel')} *</Label>
                <Input
                  id="password"
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder={t('notification.dialog.passwordPlaceholder')}
                  className="rounded-lg"
                  disabled
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="from">{t('notification.dialog.fromLabel')} *</Label>
                <Input
                  id="from"
                  value={from}
                  onChange={(e) => setFrom(e.target.value)}
                  placeholder={t('notification.dialog.fromPlaceholder')}
                  className="rounded-lg"
                  disabled
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="to">{t('notification.dialog.toLabel')} *</Label>
                <Input
                  id="to"
                  value={to}
                  onChange={(e) => setTo(e.target.value)}
                  placeholder={t('notification.dialog.toPlaceholder')}
                  className="rounded-lg"
                  disabled
                />
              </div>

              <p className="text-xs text-warning">
                {t('notification.dialog.emailComingSoon')}
              </p>
            </>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onClose(false)} className="rounded-full">
            {t('notification.dialog.buttonCancel')}
          </Button>
          <Button
            onClick={handleSave}
            disabled={saving || (channelType !== 'feishu')}
            className="rounded-full"
          >
            {saving ? t('notification.dialog.buttonSaving') : channel ? t('notification.dialog.buttonUpdate') : t('notification.dialog.buttonCreate')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
