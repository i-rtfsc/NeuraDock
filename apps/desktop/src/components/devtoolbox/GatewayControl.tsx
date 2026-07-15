import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Server, Power, Copy, Check } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { IndependentKeyDto } from '@/types/independentKey';

interface GatewayStatus {
  running: boolean;
  base_url: string | null;
  key_id: number | null;
}

interface GatewayControlProps {
  keys: IndependentKeyDto[];
}

export function GatewayControl({ keys }: GatewayControlProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedKeyId, setSelectedKeyId] = useState<string>('');
  const [isBusy, setIsBusy] = useState(false);
  const [copied, setCopied] = useState(false);

  const { data: status } = useQuery<GatewayStatus>({
    queryKey: ['gateway-status'],
    queryFn: () => invoke<GatewayStatus>('get_provider_gateway_status'),
    refetchInterval: 5000,
  });

  const activeKeys = keys.filter((k) => k.is_active);

  const handleStart = async () => {
    if (!selectedKeyId) return;
    setIsBusy(true);
    try {
      await invoke<GatewayStatus>('start_provider_gateway', {
        keyId: Number(selectedKeyId),
        model: null,
      });
      toast.success(t('devToolbox.gateway.started', '网关已启动'));
      queryClient.invalidateQueries({ queryKey: ['gateway-status'] });
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsBusy(false);
    }
  };

  const handleStop = async () => {
    setIsBusy(true);
    try {
      await invoke<GatewayStatus>('stop_provider_gateway');
      toast.success(t('devToolbox.gateway.stopped', '网关已停止'));
      queryClient.invalidateQueries({ queryKey: ['gateway-status'] });
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
      toast.success(t('devToolbox.gateway.copied', '已复制'));
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error(t('token.copyError', '复制失败'));
    }
  };

  const runningKey = status?.running
    ? keys.find((k) => k.id === status.key_id)
    : undefined;

  return (
    <Card className="p-5 space-y-4">
      <div className="flex items-center justify-between">
        <div className="space-y-0.5">
          <h3 className="text-sm font-semibold flex items-center gap-2">
            <Server className="h-4 w-4 text-primary" />
            {t('devToolbox.gateway.title', '本地适配网关')}
            {status?.running && (
              <Badge variant="outline" className="text-[10px] text-success border-success/40">
                {t('devToolbox.gateway.running', '运行中')}
              </Badge>
            )}
          </h3>
          <p className="text-xs text-muted-foreground max-w-lg">
            {t(
              'devToolbox.gateway.description',
              '把只支持 chat 接口的中转站，适配成 Codex 的 responses 或 Claude Code 的 Anthropic 接口。启动后把工具的 base URL 指向下面的本地地址。'
            )}
          </p>
        </div>
      </div>

      {status?.running ? (
        <div className="space-y-3">
          <div className="flex items-center gap-3 rounded-lg border border-border/60 bg-muted/20 px-4 py-3">
            <div className="flex-1 min-w-0">
              <p className="text-xs text-muted-foreground">
                {t('devToolbox.gateway.baseUrlLabel', '本地 base URL')}
                {runningKey && <span className="mx-1.5">·</span>}
                {runningKey?.name}
              </p>
              <code className="text-sm font-mono">{status.base_url}</code>
            </div>
            <Button variant="ghost" size="sm" className="h-8 px-2" onClick={handleCopy}>
              {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
            </Button>
          </div>
          <Button variant="outline" size="sm" onClick={handleStop} disabled={isBusy}>
            <Power className="h-3.5 w-3.5" />
            {t('devToolbox.gateway.stop', '停止网关')}
          </Button>
        </div>
      ) : (
        <div className="flex flex-wrap items-end gap-3">
          <div className="space-y-1.5">
            <Label className="text-xs">{t('devToolbox.gateway.selectKey', '选择中转站 Key')}</Label>
            <Select value={selectedKeyId} onValueChange={setSelectedKeyId}>
              <SelectTrigger className="w-64 h-9">
                <SelectValue placeholder={t('devToolbox.gateway.selectKeyPlaceholder', '选择一个 key')} />
              </SelectTrigger>
              <SelectContent>
                {activeKeys.map((key) => (
                  <SelectItem key={key.id} value={String(key.id)}>
                    {key.name} · {key.provider_type_display}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <Button size="sm" onClick={handleStart} disabled={isBusy || !selectedKeyId}>
            <Power className="h-3.5 w-3.5" />
            {t('devToolbox.gateway.start', '启动网关')}
          </Button>
        </div>
      )}
    </Card>
  );
}
