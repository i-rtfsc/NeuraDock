import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { KeyRound, Settings2, Eraser, Terminal, Code2 } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { EmptyState } from '@/components/ui/empty-state';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { IndependentKeyConfigDialog } from '@/components/token/IndependentKeyConfigDialog';
import { GatewayControl } from '@/components/devtoolbox/GatewayControl';
import type { IndependentKeyDto } from '@/types/independentKey';

type ClearTarget = 'claude' | 'codex';

export function AiToolConfigurator() {
  const { t } = useTranslation();
  const [configKey, setConfigKey] = useState<IndependentKeyDto | null>(null);
  const [configOpen, setConfigOpen] = useState(false);
  const [clearTarget, setClearTarget] = useState<ClearTarget | null>(null);
  const [isClearing, setIsClearing] = useState(false);

  const { data: keys = [], isLoading } = useQuery<IndependentKeyDto[]>({
    queryKey: ['independent-keys'],
    queryFn: () => invoke<IndependentKeyDto[]>('get_all_independent_keys'),
  });

  const handleClear = async () => {
    if (!clearTarget) return;
    setIsClearing(true);
    try {
      const command = clearTarget === 'claude' ? 'clear_claude_global' : 'clear_codex_global';
      const message = await invoke<string>(command);
      toast.success(message);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsClearing(false);
      setClearTarget(null);
    }
  };

  return (
    <div className="space-y-6">
      {/* Key list */}
      <Card className="p-5 space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <h3 className="text-sm font-semibold flex items-center gap-2">
              <KeyRound className="h-4 w-4 text-primary" />
              {t('devToolbox.aiConfig.keysTitle', '独立 API Key')}
            </h3>
            <p className="text-xs text-muted-foreground">
              {t(
                'devToolbox.aiConfig.keysDescription',
                '选择一个 key，一键写入 Codex CLI / Claude Code 的全局配置（字段级更新，保留你的其它设置）。'
              )}
            </p>
          </div>
        </div>

        {isLoading ? (
          <p className="text-sm text-muted-foreground py-6 text-center">
            {t('common.loading', '加载中...')}
          </p>
        ) : keys.length === 0 ? (
          <EmptyState
            icon={<KeyRound className="h-10 w-10" />}
            title={t('devToolbox.aiConfig.emptyTitle', '还没有独立 API Key')}
            description={t(
              'devToolbox.aiConfig.emptyDescription',
              '先到「API Keys」页面添加一个中转站 key，再回来配置 AI 工具。'
            )}
          />
        ) : (
          <div className="divide-y divide-border/60 rounded-lg border border-border/60">
            {keys.map((key) => (
              <div key={key.id} className="flex items-center gap-4 px-4 py-3">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium truncate">{key.name}</span>
                    <Badge variant="outline" className="text-[10px] shrink-0">
                      {key.provider_type_display}
                    </Badge>
                    {!key.is_active && (
                      <Badge variant="secondary" className="text-[10px] shrink-0">
                        {t('devToolbox.aiConfig.inactive', '已停用')}
                      </Badge>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground truncate mt-0.5">
                    <code>{key.masked_key}</code>
                    <span className="mx-1.5">·</span>
                    {key.base_url}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!key.is_active}
                  onClick={() => {
                    setConfigKey(key);
                    setConfigOpen(true);
                  }}
                >
                  <Settings2 className="h-3.5 w-3.5" />
                  {t('devToolbox.aiConfig.configure', '配置')}
                </Button>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Local adaptation gateway */}
      <GatewayControl keys={keys} />

      {/* Clear managed config */}
      <Card className="p-5 space-y-4">
        <div className="space-y-0.5">
          <h3 className="text-sm font-semibold flex items-center gap-2">
            <Eraser className="h-4 w-4 text-primary" />
            {t('devToolbox.aiConfig.clearTitle', '清除已写入的配置')}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t(
              'devToolbox.aiConfig.clearDescription',
              '只移除 NeuraDock 写入的字段（写入前有 .neuradock.bak 备份），你的其它配置保持不变。'
            )}
          </p>
        </div>
        <div className="flex gap-3">
          <Button variant="outline" size="sm" onClick={() => setClearTarget('claude')}>
            <Terminal className="h-3.5 w-3.5" />
            {t('devToolbox.aiConfig.clearClaude', '清除 Claude Code 配置')}
          </Button>
          <Button variant="outline" size="sm" onClick={() => setClearTarget('codex')}>
            <Code2 className="h-3.5 w-3.5" />
            {t('devToolbox.aiConfig.clearCodex', '清除 Codex 配置')}
          </Button>
        </div>
      </Card>

      <IndependentKeyConfigDialog
        open={configOpen}
        onOpenChange={setConfigOpen}
        keyData={configKey}
      />

      <AlertDialog open={clearTarget !== null} onOpenChange={(open) => !open && setClearTarget(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {clearTarget === 'claude'
                ? t('devToolbox.aiConfig.clearClaudeConfirmTitle', '清除 Claude Code 配置？')
                : t('devToolbox.aiConfig.clearCodexConfirmTitle', '清除 Codex 配置？')}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                'devToolbox.aiConfig.clearConfirmDescription',
                '仅移除 NeuraDock 管理的字段（API 地址、密钥、模型等），其余内容不受影响。'
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isClearing}>
              {t('common.cancel', '取消')}
            </AlertDialogCancel>
            <AlertDialogAction onClick={handleClear} disabled={isClearing}>
              {t('devToolbox.aiConfig.clearConfirm', '清除')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
