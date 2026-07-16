import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { useState } from 'react';
import { Copy, Check, Power, Unplug, Server, Terminal, Code2 } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export interface GatewayBinding {
  client: 'codex' | 'claude_code';
  label: string;
  model: string | null;
}

export interface GatewayStatus {
  running: boolean;
  base_url: string | null;
  bindings: GatewayBinding[];
}

export const GATEWAY_STATUS_QUERY_KEY = ['gateway-status'];

export function useGatewayStatus() {
  return useQuery<GatewayStatus>({
    queryKey: GATEWAY_STATUS_QUERY_KEY,
    queryFn: () => invoke<GatewayStatus>('get_gateway_status'),
    refetchInterval: 5000,
  });
}

const CLIENT_META = {
  codex: { name: 'Codex', icon: Code2 },
  claude_code: { name: 'Claude Code', icon: Terminal },
} as const;

interface GatewayPanelDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function GatewayPanelDialog({ open, onOpenChange }: GatewayPanelDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data: status } = useGatewayStatus();
  const [isBusy, setIsBusy] = useState(false);
  const [copied, setCopied] = useState(false);

  const refresh = () => queryClient.invalidateQueries({ queryKey: GATEWAY_STATUS_QUERY_KEY });

  const handleToggle = async () => {
    setIsBusy(true);
    try {
      if (status?.running) {
        await invoke('stop_gateway_server');
        toast.success(t('gateway.stopped', '网关已停止，工具配置已还原'));
      } else {
        await invoke('start_gateway_server');
        toast.success(t('gateway.started', '网关已启动，已接管绑定客户端的配置'));
      }
      refresh();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsBusy(false);
    }
  };

  const handleUnbind = async (client: GatewayBinding['client']) => {
    setIsBusy(true);
    try {
      await invoke('clear_gateway_binding', { client });
      toast.success(t('gateway.unbound', '已解绑并还原该客户端配置'));
      refresh();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsBusy(false);
    }
  };

  const handleCopy = async () => {
    if (!status?.base_url) return;
    try {
      await navigator.clipboard.writeText(status.base_url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error(t('token.copyError', '复制失败'));
    }
  };

  const bindings = status?.bindings ?? [];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Server className="h-5 w-5 text-primary" />
            {t('gateway.title', '本地适配网关')}
            <Badge
              variant="outline"
              className={
                status?.running
                  ? 'text-success border-success/40'
                  : 'text-muted-foreground border-border'
              }
            >
              {status?.running ? t('gateway.running', '运行中') : t('gateway.stoppedBadge', '已停止')}
            </Badge>
          </DialogTitle>
          <DialogDescription>
            {t(
              'gateway.description',
              '把只支持 chat 接口的中转站适配给 Codex / Claude Code。启动后自动接管工具配置（原配置已备份，停止时还原）。'
            )}
          </DialogDescription>
        </DialogHeader>

        {status?.running && status.base_url && (
          <div className="flex items-center gap-2 rounded-lg border border-border/60 bg-muted/20 px-3 py-2">
            <code className="flex-1 text-sm font-mono truncate">{status.base_url}</code>
            <Button variant="ghost" size="sm" className="h-7 px-2" onClick={handleCopy}>
              {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            </Button>
          </div>
        )}

        <div className="space-y-2">
          <p className="text-xs font-medium text-muted-foreground">
            {t('gateway.bindingsTitle', '客户端绑定')}
          </p>
          {bindings.length === 0 ? (
            <p className="text-sm text-muted-foreground py-3 text-center rounded-lg border border-dashed border-border/60">
              {t(
                'gateway.noBindings',
                '还没有绑定。到中转站账号详情或 API Key 详情里，把一个 key 绑定给 Codex / Claude Code。'
              )}
            </p>
          ) : (
            bindings.map((binding) => {
              const meta = CLIENT_META[binding.client];
              const Icon = meta.icon;
              return (
                <div
                  key={binding.client}
                  className="flex items-center gap-3 rounded-lg border border-border/60 px-3 py-2"
                >
                  <Icon className="h-4 w-4 text-primary shrink-0" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium">{meta.name}</p>
                    <p className="text-xs text-muted-foreground truncate">
                      {binding.label}
                      {binding.model && (
                        <>
                          <span className="mx-1">·</span>
                          {binding.model}
                        </>
                      )}
                    </p>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 px-2 text-muted-foreground"
                    disabled={isBusy}
                    onClick={() => handleUnbind(binding.client)}
                  >
                    <Unplug className="h-3.5 w-3.5" />
                    {t('gateway.unbind', '解绑')}
                  </Button>
                </div>
              );
            })
          )}
        </div>

        <Button
          onClick={handleToggle}
          disabled={isBusy || (!status?.running && bindings.length === 0)}
          variant={status?.running ? 'outline' : 'default'}
          className="w-full"
        >
          <Power className="h-4 w-4" />
          {status?.running ? t('gateway.stop', '停止网关') : t('gateway.start', '启动网关')}
        </Button>
      </DialogContent>
    </Dialog>
  );
}
