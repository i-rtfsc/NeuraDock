import { useTranslation } from 'react-i18next';
import { useEffect, useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Globe, Terminal, Copy, Check, Server } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';
import type { TokenDto, AccountDto } from '@/types/token';
import type { AITool } from '@/hooks/useConfigState';

type UpstreamKindValue = 'openai_chat' | 'openai_responses' | 'anthropic';

interface CommandGeneratorProps {
  token: TokenDto;
  account: AccountDto;
  selectedTool: AITool;
  selectedNode: string;
  selectedModel: string;
  context1m: boolean;
  isCompatible: boolean;
  tempCommands: string;
  setTempCommands: (commands: string) => void;
  displayCommands: string;
  copied: boolean;
  setCopied: (copied: boolean) => void;
  isSingleLine: boolean;
  setIsSingleLine: (singleLine: boolean) => void;
  onSuccess?: () => void;
}

export function CommandGenerator({
  token,
  account,
  selectedTool,
  selectedNode,
  selectedModel,
  context1m,
  isCompatible,
  tempCommands,
  setTempCommands,
  displayCommands,
  copied,
  setCopied,
  isSingleLine,
  setIsSingleLine,
  onSuccess,
}: CommandGeneratorProps) {
  const { t } = useTranslation();

  // Default the upstream protocol to each tool's native shape (passthrough),
  // which most relays support; the user can switch to Chat for chat-only relays.
  const [upstreamKind, setUpstreamKind] = useState<UpstreamKindValue>(
    selectedTool === 'claude' ? 'anthropic' : 'openai_responses'
  );
  useEffect(() => {
    setUpstreamKind(selectedTool === 'claude' ? 'anthropic' : 'openai_responses');
  }, [selectedTool]);

  const configureGlobalMutation = useMutation({
    mutationFn: () => {
      if (selectedTool === 'claude') {
        return invoke<string>('configure_claude_global', {
          tokenId: token.id,
          accountId: token.account_id,
          baseUrl: selectedNode,
          model: selectedModel || null,
          context1m,
        });
      } else if (selectedTool === 'codex') {
        return invoke<string>('configure_codex_global', {
          tokenId: token.id,
          accountId: token.account_id,
          providerId: account.provider_id,
          baseUrl: selectedNode,
          model: selectedModel || null,
          context1m,
        });
      } else {
        throw new Error('Not implemented');
      }
    },
    onSuccess: (message) => {
      toast.success(message);
      onSuccess?.();
    },
    onError: (error: Error) => {
      toast.error(t('token.configError', 'Configuration failed: ') + error.message);
    },
  });

  const bindGatewayMutation = useMutation({
    mutationFn: (upstreamKind: 'openai_chat' | 'openai_responses' | 'anthropic') => {
      const client = selectedTool === 'claude' ? 'claude_code' : 'codex';
      return invoke<unknown>('set_gateway_binding', {
        client,
        source: {
          type: 'account_token',
          account_id: token.account_id,
          provider_id: account.provider_id,
          token_id: token.id,
          base_url: selectedNode || null,
        },
        upstreamKind,
        model: selectedModel || null,
      });
    },
    onSuccess: () => {
      toast.success(t('gateway.bound', '已绑定，可在侧边栏「本地网关」启动'));
      onSuccess?.();
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(t('token.configError', 'Configuration failed: ') + message);
    },
  });

  const generateTempMutation = useMutation({
    mutationFn: () => {
      if (selectedTool === 'claude') {
        return invoke<string>('generate_claude_temp_commands', {
          tokenId: token.id,
          accountId: token.account_id,
          baseUrl: selectedNode,
          model: selectedModel || null,
        });
      } else if (selectedTool === 'codex') {
        return invoke<string>('generate_codex_temp_commands', {
          tokenId: token.id,
          accountId: token.account_id,
          providerId: account.provider_id,
          baseUrl: selectedNode,
          model: selectedModel || null,
        });
      } else {
        throw new Error('Not implemented');
      }
    },
    onSuccess: (commands) => {
      setTempCommands(commands);
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(t('token.generateError', 'Failed to generate commands: ') + message);
    },
  });

  const handleCopyCommands = async () => {
    try {
      await navigator.clipboard.writeText(displayCommands);
      setCopied(true);
      toast.success(t('token.configDialog.copied', 'Commands copied to clipboard'));
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      toast.error(t('token.copyError', 'Failed to copy'));
    }
  };

  return (
    <Tabs defaultValue="global" className="w-full">
      <TabsList className="grid w-full grid-cols-2 h-11 bg-muted/50 p-1">
        <TabsTrigger
          value="global"
          className="data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all duration-base ease-smooth"
        >
          {t('token.configDialog.globalConfig', 'Global Config')}
        </TabsTrigger>
        <TabsTrigger
          value="temp"
          className="data-[state=active]:bg-background data-[state=active]:shadow-sm transition-all duration-base ease-smooth"
        >
          {t('token.configDialog.tempSession', 'Temporary Session')}
        </TabsTrigger>
      </TabsList>

      <TabsContent
        value="global"
        className="mt-4 space-y-3 animate-in fade-in slide-in-from-bottom-2"
      >
        <div className="rounded-lg bg-primary/5 border border-primary/10 p-3">
          <div className="flex gap-2.5 items-start">
            <div className="p-1.5 rounded-lg bg-primary/10 text-primary shrink-0">
              <Globe className="h-3.5 w-3.5" />
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-xs text-muted-foreground leading-relaxed">
                {t('token.configDialog.globalConfigDesc')}
              </p>
              <p className="text-[10px] text-muted-foreground/70 mt-1 font-mono">
                {selectedTool === 'claude'
                  ? '~/.claude/settings.json'
                  : '~/.codex/config.toml'}
              </p>
            </div>
          </div>
        </div>
        <Button
          className="w-full h-10 text-sm font-medium shadow-lg shadow-primary/20"
          onClick={() => configureGlobalMutation.mutate()}
          disabled={
            !selectedNode || configureGlobalMutation.isPending || !isCompatible
          }
        >
          {configureGlobalMutation.isPending
            ? t('common.configuring', 'Installing Configuration...')
            : t('token.configDialog.installConfig', 'Install Configuration')}
        </Button>
        {selectedTool !== 'gemini' && (
          <div className="space-y-2 rounded-lg border border-border/60 bg-muted/10 p-3">
            <div className="flex items-center gap-2">
              <Server className="h-3.5 w-3.5 text-primary shrink-0" />
              <span className="text-xs font-medium">
                {t('gateway.viaGateway', '经本地网关（适配协议）')}
              </span>
            </div>
            <p className="text-[10px] text-muted-foreground leading-relaxed">
              {t(
                'gateway.upstreamKindHint',
                '选择该中转站上游的协议：原生 responses/anthropic 走透传，只支持 chat 的选 Chat 由网关转换。'
              )}
            </p>
            <div className="flex gap-2">
              <select
                value={upstreamKind}
                onChange={(e) => setUpstreamKind(e.target.value as UpstreamKindValue)}
                className="h-9 flex-1 rounded-md border border-border/60 bg-background px-2 text-xs"
              >
                {selectedTool === 'codex' && (
                  <option value="openai_responses">
                    {t('gateway.kindResponses', 'OpenAI Responses（原生透传）')}
                  </option>
                )}
                {selectedTool === 'claude' && (
                  <option value="anthropic">
                    {t('gateway.kindAnthropic', 'Anthropic（原生透传）')}
                  </option>
                )}
                <option value="openai_chat">
                  {t('gateway.kindChat', 'OpenAI Chat（网关转换）')}
                </option>
              </select>
              <Button
                variant="outline"
                className="h-9 text-sm font-medium"
                onClick={() => bindGatewayMutation.mutate(upstreamKind)}
                disabled={!selectedNode || bindGatewayMutation.isPending || !isCompatible}
              >
                {selectedTool === 'claude'
                  ? t('gateway.bindClaude', '绑定给 Claude Code')
                  : t('gateway.bindCodex', '绑定给 Codex')}
              </Button>
            </div>
          </div>
        )}
      </TabsContent>

      <TabsContent
        value="temp"
        className="mt-4 space-y-3 animate-in fade-in slide-in-from-bottom-2"
      >
        <div className="rounded-lg bg-muted/30 border border-border/50 p-3">
          <div className="flex gap-2.5 items-start">
            <div className="p-1.5 rounded-lg bg-muted text-muted-foreground shrink-0">
              <Terminal className="h-3.5 w-3.5" />
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-xs text-muted-foreground leading-relaxed">
                {t('token.configDialog.tempSessionDesc')}
              </p>
            </div>
          </div>
        </div>

        {!tempCommands ? (
          <Button
            className="w-full h-10 text-sm font-medium"
            variant="outline"
            onClick={() => generateTempMutation.mutate()}
            disabled={!selectedNode || generateTempMutation.isPending}
          >
            {generateTempMutation.isPending
              ? t('common.generating', 'Generating Commands...')
              : t('token.configDialog.generateCommands', 'Generate Export Commands')}
          </Button>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center justify-between px-0.5">
              <div className="flex items-center gap-2">
                <Switch
                  id="single-line-mode"
                  checked={isSingleLine}
                  onCheckedChange={setIsSingleLine}
                  className="scale-90"
                />
                <Label
                  htmlFor="single-line-mode"
                  className="text-xs font-medium cursor-pointer"
                >
                  {isSingleLine
                    ? t('token.configDialog.singleLine', '单行命令')
                    : t('token.configDialog.multiLine', '多行命令')}
                </Label>
              </div>
              <Button
                size="sm"
                variant="ghost"
                className="h-6 px-2 text-[10px] text-muted-foreground hover:text-foreground"
                onClick={() => setTempCommands('')}
              >
                {t('token.configDialog.clear', '清除')}
              </Button>
            </div>

            <div className="relative rounded-[var(--radius-control)] border border-[hsl(var(--control-border))] bg-[hsl(var(--popover))] shadow-inner overflow-hidden group">
              <div className="absolute top-0 left-0 right-0 h-7 bg-[hsl(var(--muted)/0.65)] flex items-center px-2.5 gap-1.5 border-b border-[hsl(var(--control-border))] justify-between backdrop-blur-[var(--control-surface-blur,0px)]">
                <div className="flex gap-1.5">
                  <div className="h-2 w-2 rounded-full bg-danger/30" />
                  <div className="h-2 w-2 rounded-full bg-warning/30" />
                  <div className="h-2 w-2 rounded-full bg-success/30" />
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-5 px-1.5 text-[10px] text-muted-foreground hover:text-foreground"
                  onClick={handleCopyCommands}
                >
                  {copied ? (
                    <Check className="h-3 w-3 mr-1" />
                  ) : (
                    <Copy className="h-3 w-3 mr-1" />
                  )}
                  {copied
                    ? t('token.configDialog.copied', '已复制')
                    : t('token.configDialog.copy', '复制')}
                </Button>
              </div>
              <Textarea
                value={displayCommands}
                readOnly
                className={cn(
                  'w-full resize-none bg-transparent font-mono text-[11px] text-foreground border-none focus-visible:ring-0 px-3 pt-9 pb-3 leading-relaxed selection:bg-primary/30',
                  isSingleLine
                    ? 'min-h-[70px] whitespace-nowrap overflow-x-auto'
                    : 'min-h-[100px]'
                )}
              />
            </div>
          </div>
        )}
      </TabsContent>
    </Tabs>
  );
}
