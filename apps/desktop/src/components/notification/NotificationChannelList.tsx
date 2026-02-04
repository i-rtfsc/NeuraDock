import { useState } from 'react';
import { Bell, Trash2, TestTube2, Plus, Mail, MessageSquare, Send, Edit2, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';
import { NotificationChannelDialog } from './NotificationChannelDialog';
import { useTranslation } from 'react-i18next';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

export interface NotificationChannelDto {
  id: string;
  channel_type: string;
  config: string;
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
      toast.error(t('common.error'), { description: String(err) });
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
      toast.error(t('common.error'), { description: String(err) });
    }
  };

  const handleTest = async (channelId: string) => {
    setTestingId(channelId);
    try {
      const result = await invoke<{ success: boolean; message: string }>('test_notification_channel', {
        channelId,
      });
      if (result.success) {
        toast.success(t('notification.testSuccess'), { description: result.message });
      } else {
        toast.error(t('notification.testFailed'), { description: result.message });
      }
    } catch (err) {
      toast.error(t('notification.testFailed'), { description: String(err) });
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
      case 'feishu': return MessageSquare;
      case 'dingtalk': return Send;
      case 'email': return Mail;
      default: return Bell;
    }
  };

  const getChannelTypeName = (type: string) => t(`notification.channel.${type}`, type);

  const getChannelStyle = (type: string) => {
    switch (type) {
      case 'feishu': return 'bg-[#3370ff]/10 text-[#3370ff] border-[#3370ff]/20';
      case 'dingtalk': return 'bg-[#007fff]/10 text-[#007fff] border-[#007fff]/20';
      case 'email': return 'bg-purple-500/10 text-purple-600 border-purple-500/20';
      default: return 'bg-muted text-muted-foreground border-border';
    }
  };

  const parseConfig = (configStr: string) => {
    try { return JSON.parse(configStr); } catch { return {}; }
  };

  return (
    <div className="w-full space-y-4">
      {/* Main Container */}
      <div className="bg-card/50 backdrop-blur-md border border-border/60 rounded-2xl overflow-hidden shadow-sm w-full">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/40 bg-muted/20">
           <h3 className="text-sm font-semibold text-foreground/70 uppercase tracking-wide flex items-center gap-2">
             <Bell className="h-4 w-4" />
             {t('settings.notification')}
           </h3>
           <Button onClick={handleAddNew} size="sm" className="rounded-full px-4 text-xs font-medium shadow-none hover:shadow-sm transition-all duration-base ease-smooth">
             <Plus className="h-3.5 w-3.5 mr-1.5" />
             {t('notification.addChannel')}
           </Button>
        </div>

        {/* List Content */}
        {channels.length === 0 ? (
          <div className="py-16 flex flex-col items-center justify-center text-center space-y-4 text-muted-foreground/60">
             <div className="w-16 h-16 rounded-full bg-muted/50 flex items-center justify-center mb-2">
                <AlertCircle className="h-8 w-8 opacity-50" />
             </div>
             <p className="text-sm">{t('notification.noChannelsDesc')}</p>
          </div>
        ) : (
          <div className="divide-y divide-border/40">
            {channels.map((channel) => {
              const config = parseConfig(channel.config);
              const Icon = getChannelTypeIcon(channel.channel_type);
              const styleClass = getChannelStyle(channel.channel_type);
              
              return (
                <div 
                  key={channel.id} 
                  className={cn(
                    "group flex items-center gap-5 px-6 py-5 transition-all duration-base ease-smooth hover:bg-muted/30",
                    !channel.enabled && "opacity-70 grayscale-[0.3]"
                  )}
                >
                  {/* Icon */}
                  <div className={cn(
                    "w-10 h-10 rounded-xl flex items-center justify-center shrink-0 border shadow-sm transition-transform duration-base ease-smooth group-hover:scale-[var(--scale-pop)]",
                    styleClass
                  )}>
                    <Icon className="h-5 w-5" />
                  </div>

                  {/* Info */}
                  <div className="flex-1 min-w-0 flex flex-col justify-center gap-0.5">
                    <div className="flex items-center gap-2.5">
                      <span className="text-base font-medium text-foreground">
                        {getChannelTypeName(channel.channel_type)}
                      </span>
                      {!channel.enabled && (
                        <Badge variant="outline" className="text-[10px] h-4 px-1.5 py-0 bg-transparent text-muted-foreground border-muted-foreground/30">
                          {t('notification.disabled')}
                        </Badge>
                      )}
                    </div>
                    
                    <div className="text-sm text-muted-foreground font-mono truncate max-w-md flex items-center gap-2">
                       {channel.channel_type === 'feishu' && config.webhook_key && `Webhook: ${config.webhook_key.slice(0, 12)}...`}
                       {channel.channel_type === 'dingtalk' && config.webhook_key && `Token: ${config.webhook_key.slice(0, 12)}...`}
                       {channel.channel_type === 'email' && config.from && `From: ${config.from}`}
                    </div>
                  </div>

                  {/* Actions (Hover Reveal) */}
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-base ease-smooth -mr-2">
                     <Tooltip>
                        <TooltipTrigger asChild>
                           <Button variant="ghost" size="icon-sm" className="text-muted-foreground hover:text-primary" onClick={() => handleTest(channel.id)}>
                              {testingId === channel.id ? <span className="animate-spin text-xs">⟳</span> : <TestTube2 className="h-4 w-4" />}
                           </Button>
                        </TooltipTrigger>
                        <TooltipContent>{t('notification.test')}</TooltipContent>
                     </Tooltip>

                     <Tooltip>
                        <TooltipTrigger asChild>
                           <Button variant="ghost" size="icon-sm" className="text-muted-foreground hover:text-foreground" onClick={() => handleEdit(channel)}>
                              <Edit2 className="h-4 w-4" />
                           </Button>
                        </TooltipTrigger>
                        <TooltipContent>{t('common.edit')}</TooltipContent>
                     </Tooltip>

                     <Tooltip>
                        <TooltipTrigger asChild>
                           <Button variant="ghost" size="icon-sm" className="text-muted-foreground hover:text-destructive" onClick={() => handleDelete(channel.id)}>
                              <Trash2 className="h-4 w-4" />
                           </Button>
                        </TooltipTrigger>
                        <TooltipContent>{t('common.delete')}</TooltipContent>
                     </Tooltip>

                     <div className="w-px h-4 bg-border/50 mx-1" />
                  </div>

                  {/* Toggle */}
                  <div className="shrink-0 pl-2">
                    <Switch
                      checked={channel.enabled}
                      onCheckedChange={() => handleToggle(channel)}
                      className="data-[state=checked]:bg-primary"
                    />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <NotificationChannelDialog
        open={showDialog}
        onClose={handleDialogClose}
        channel={editingChannel}
      />
    </div>
  );
}
