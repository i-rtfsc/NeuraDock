import { useState } from 'react';
import { Bell, Trash2, TestTube2, Plus, Check, X, Mail, MessageSquare, Send } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import { NotificationChannelDialog } from './NotificationChannelDialog';
import { useTranslation } from 'react-i18next';
import { cn } from '@/lib/utils';

export interface NotificationChannelDto {
  id: string;
  channel_type: string;
  config: string;  // JSON string from Rust
  enabled: boolean;
  created_at: string;
}

interface NotificationChannelListProps {
  channels: NotificationChannelDto[];
  onUpdate: () => void;
}

export function NotificationChannelList({ channels, onUpdate }: NotificationChannelListProps) {
  const { t } = useTranslation();
  const [showDialog, setShowDialog] = useState(false);
  const [editingChannel, setEditingChannel] = useState<NotificationChannelDto | null>(null);
  const [testingId, setTestingId] = useState<string | null>(null);

  const handleDelete = async (channelId: string) => {
    try {
      await invoke('delete_notification_channel', { channelId });
      toast.success(t('notification.deleted'));
      onUpdate();
    } catch (err) {
      toast.error(t('common.error'), {
        description: String(err),
      });
    }
  };

  const handleToggle = async (channel: NotificationChannelDto) => {
    try {
      await invoke('update_notification_channel', {
        input: {
          channel_id: channel.id,
          enabled: !channel.enabled,
          config: null,
        },
      });
      toast.success(channel.enabled ? t('notification.disabled') : t('notification.enabled'));
      onUpdate();
    } catch (err) {
      toast.error(t('common.error'), {
        description: String(err),
      });
    }
  };

  const handleTest = async (channelId: string) => {
    setTestingId(channelId);
    try {
      const result = await invoke<{ success: boolean; message: string }>('test_notification_channel', {
        channelId,
      });

      if (result.success) {
        toast.success(t('notification.testSuccess'), {
          description: result.message,
        });
      } else {
        toast.error(t('notification.testFailed'), {
          description: result.message,
        });
      }
    } catch (err) {
      toast.error(t('notification.testFailed'), {
        description: String(err),
      });
    } finally {
      setTestingId(null);
    }
  };

  const handleEdit = (channel: NotificationChannelDto) => {
    setEditingChannel(channel);
    setShowDialog(true);
  };

  const handleAddNew = () => {
    setEditingChannel(null);
    setShowDialog(true);
  };

  const handleDialogClose = (success: boolean) => {
    setShowDialog(false);
    setEditingChannel(null);
    if (success) {
      onUpdate();
    }
  };

  const getChannelTypeIcon = (type: string) => {
    switch (type) {
      case 'feishu':
        return MessageSquare;
      case 'dingtalk':
        return Send;
      case 'email':
        return Mail;
      default:
        return Bell;
    }
  };

  const getChannelTypeName = (type: string) => {
    return t(`notification.channel.${type}`, type);
  };

  const getChannelTypeColor = (type: string) => {
    switch (type) {
      case 'feishu':
        return 'bg-blue-500/10 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400';
      case 'dingtalk':
        return 'bg-cyan-500/10 text-cyan-600 dark:bg-cyan-500/20 dark:text-cyan-400';
      case 'email':
        return 'bg-purple-500/10 text-purple-600 dark:bg-purple-500/20 dark:text-purple-400';
      default:
        return 'bg-gray-500/10 text-gray-600 dark:bg-gray-500/20 dark:text-gray-400';
    }
  };

  const parseConfig = (configStr: string) => {
    try {
      return JSON.parse(configStr);
    } catch {
      return {};
    }
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between pb-2">
        <Button onClick={handleAddNew} className="rounded-full ml-auto">
          <Plus className="h-4 w-4 mr-2" />
          {t('notification.addChannel')}
        </Button>
      </div>

      {channels.length === 0 ? (
        <Card className="rounded-2xl border-2 border-dashed">
          <CardContent className="py-16 text-center">
            <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-primary/10 flex items-center justify-center">
              <Bell className="h-10 w-10 text-primary" />
            </div>
            <h3 className="text-lg font-semibold mb-2">{t('notification.noChannels')}</h3>
            <p className="text-muted-foreground mb-6 max-w-md mx-auto">
              {t('notification.noChannelsDesc')}
            </p>
            <Button onClick={handleAddNew} size="lg" className="rounded-full">
              <Plus className="h-5 w-5 mr-2" />
              {t('notification.addFirstChannel')}
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-3">
          {channels.map((channel) => {
            const config = parseConfig(channel.config);
            const Icon = getChannelTypeIcon(channel.channel_type);
            
            return (
              <Card 
                key={channel.id} 
                className={cn(
                  "rounded-2xl border-2 transition-all duration-200",
                  channel.enabled 
                    ? "border-primary/20 bg-primary/5" 
                    : "border-border/50 opacity-60 hover:opacity-100"
                )}
              >
                <CardContent className="py-4">
                  <div className="flex items-center gap-4">
                    {/* Icon */}
                    <div className={cn(
                      "w-12 h-12 rounded-2xl flex items-center justify-center shrink-0",
                      getChannelTypeColor(channel.channel_type)
                    )}>
                      <Icon className="h-6 w-6" />
                    </div>

                    {/* Info */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <h4 className="font-semibold text-base">
                          {getChannelTypeName(channel.channel_type)}
                        </h4>
                        {channel.enabled ? (
                          <Badge variant="default" className="rounded-full px-2 py-0.5">
                            <Check className="h-3 w-3 mr-1" />
                            {t('notification.enabled')}
                          </Badge>
                        ) : (
                          <Badge variant="secondary" className="rounded-full px-2 py-0.5">
                            <X className="h-3 w-3 mr-1" />
                            {t('notification.disabled')}
                          </Badge>
                        )}
                      </div>

                      <div className="text-sm text-muted-foreground truncate">
                        {channel.channel_type === 'feishu' && config.webhook_key && (
                          <span>{t('notification.webhookKey')}: {config.webhook_key.substring(0, 24)}...</span>
                        )}
                        {channel.channel_type === 'dingtalk' && config.webhook_key && (
                          <span>{t('notification.webhookKey')}: {config.webhook_key.substring(0, 24)}...</span>
                        )}
                        {channel.channel_type === 'email' && config.from && (
                          <span>{t('notification.sender')}: {config.from}</span>
                        )}
                      </div>
                    </div>

                    {/* Actions */}
                    <div className="flex items-center gap-2 shrink-0">
                      <Switch
                        checked={channel.enabled}
                        onCheckedChange={() => handleToggle(channel)}
                      />

                      <Button
                        variant="ghost"
                        size="icon"
                        className="rounded-full h-9 w-9"
                        onClick={() => handleTest(channel.id)}
                        disabled={testingId === channel.id}
                        title={t('notification.test')}
                      >
                        <TestTube2 className="h-4 w-4" />
                      </Button>

                      <Button
                        variant="ghost"
                        size="icon"
                        className="rounded-full h-9 w-9 text-destructive hover:text-destructive hover:bg-destructive/10"
                        onClick={() => handleDelete(channel.id)}
                        title={t('common.delete')}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <NotificationChannelDialog
        open={showDialog}
        onClose={handleDialogClose}
        channel={editingChannel}
      />
    </div>
  );
}
