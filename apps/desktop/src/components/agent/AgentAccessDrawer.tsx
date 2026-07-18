import { useEffect, useMemo, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import {
  Check,
  Copy,
  FileCode2,
  KeyRound,
  Loader2,
  LockKeyhole,
  Server,
  ShieldCheck,
  SlidersHorizontal,
} from 'lucide-react';

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { cn } from '@/lib/utils';
import type { ProviderNode } from '@/types/token';
import type { AgentAccess, AgentAccessSource, AgentDef } from './AgentConfigGateway';

type UpstreamKind = 'openai_chat' | 'openai_responses' | 'anthropic';

/** A selectable key, from either a relay account token or an independent key. */
export interface KeyOption {
  label: string;
  masked: string;
  providerName: string;
  providerId: string | null;
  baseUrl: string | null;
  accountId: string | null;
  accountName: string | null;
  sourceGroup: 'relay' | 'independent';
  balance: string | null;
  modelLimitsEnabled: boolean;
  allowedModels: string[];
  deniedModels: string[];
  source: AgentAccessSource;
}

type Method = 'direct' | 'gateway';
type ConfigView = 'form' | 'files';
type FileVersion = 'current' | 'next';

interface AgentConfigPreviewFile {
  path: string;
  format: string;
  current_content: string;
  next_content: string;
}

interface AgentConfigPreview {
  files: AgentConfigPreviewFile[];
  takeover_active: boolean;
  gateway_running: boolean;
}

interface AgentConfigPreviewRequest {
  client: AgentDef['id'];
  method: Method;
  source: AgentAccessSource;
  model: string | null;
  context1m: boolean;
}

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  agent: AgentDef;
  keyOptions: KeyOption[];
  loading?: boolean;
  existing?: AgentAccess;
  /** When opened from a specific token (account detail), keep the same fields but make them read-only. */
  presetOptionId?: string;
  onDone: () => void;
}

function sourceOptionId(source: AgentAccessSource): string {
  return source.type === 'account_token'
    ? `at:${source.account_id}:${source.token_id}`
    : `ik:${source.key_id}`;
}

function optionId(option: KeyOption): string {
  return sourceOptionId(option.source);
}

interface KeySource {
  id: string;
  label: string;
  group: 'relay' | 'independent';
  keys: KeyOption[];
}

interface AccountSource {
  id: string;
  label: string;
  keys: KeyOption[];
}

const INDEPENDENT_SOURCE_ID = 'independent';
function keySourceId(option: KeyOption): string {
  return option.sourceGroup === 'relay'
    ? `relay:${option.providerId ?? option.providerName}`
    : INDEPENDENT_SOURCE_ID;
}

export function AgentAccessDrawer({
  open,
  onOpenChange,
  agent,
  keyOptions,
  loading = false,
  existing,
  presetOptionId,
  onDone,
}: Props) {
  const { t } = useTranslation();
  const initializedRef = useRef<string | null>(null);
  const [sourceId, setSourceId] = useState('');
  const [accountId, setAccountId] = useState('');
  const [selectedId, setSelectedId] = useState('');
  const [method, setMethod] = useState<Method>('gateway');
  const [kind, setKind] = useState<UpstreamKind>(agent.nativeKind);
  const [impersonate, setImpersonate] = useState(true);
  const [node, setNode] = useState('');
  const [model, setModel] = useState('');
  const [context1m, setContext1m] = useState(false);
  const [busy, setBusy] = useState(false);
  const [configView, setConfigView] = useState<ConfigView>('form');
  const [fileVersion, setFileVersion] = useState<FileVersion>('next');
  const [activeFileIndex, setActiveFileIndex] = useState(0);
  const [copied, setCopied] = useState(false);
  const [previewRequest, setPreviewRequest] = useState<AgentConfigPreviewRequest | null>(null);

  const locked = !!presetOptionId;

  const sources = useMemo<KeySource[]>(() => {
    const relay = new Map<string, KeyOption[]>();
    const independent: KeyOption[] = [];
    for (const option of keyOptions) {
      if (option.sourceGroup === 'relay') {
        const providerKey = option.providerId ?? option.providerName;
        const list = relay.get(providerKey) ?? [];
        list.push(option);
        relay.set(providerKey, list);
      } else {
        independent.push(option);
      }
    }

    const list: KeySource[] = Array.from(relay, ([providerKey, keys]) => ({
      id: `relay:${providerKey}`,
      label: keys[0]?.providerName ?? providerKey,
      group: 'relay' as const,
      keys,
    }));
    if (independent.length > 0) {
      list.push({
        id: INDEPENDENT_SOURCE_ID,
        label: t('agentConfig.groupIndependent', '独立 API Key'),
        group: 'independent',
        keys: independent,
      });
    }
    return list;
  }, [keyOptions, t]);

  const existingOptionId = existing?.source ? sourceOptionId(existing.source) : '';
  const initializationKey = `${agent.id}:${presetOptionId ?? ''}:${existingOptionId}:${keyOptions.length}`;

  useEffect(() => {
    if (!open) {
      initializedRef.current = null;
      return;
    }
    if (loading || initializedRef.current === initializationKey) return;
    initializedRef.current = initializationKey;

    const targetId = presetOptionId ?? existingOptionId;
    const target = keyOptions.find((option) => optionId(option) === targetId);
    const canRestoreExisting = !presetOptionId || !existingOptionId || presetOptionId === existingOptionId;

    setMethod(canRestoreExisting ? existing?.method ?? 'gateway' : 'gateway');
    setKind(
      canRestoreExisting
        ? (existing?.upstream_kind as UpstreamKind) ?? agent.nativeKind
        : agent.nativeKind
    );
    setImpersonate(canRestoreExisting ? existing?.impersonate ?? true : true);
    setModel(canRestoreExisting ? existing?.model ?? '' : '');
    setContext1m(canRestoreExisting ? existing?.context1m ?? false : false);
    setNode(
      canRestoreExisting && existing?.source?.type === 'account_token'
        ? existing.source.base_url ?? ''
        : ''
    );

    if (target) {
      setSelectedId(optionId(target));
      setSourceId(keySourceId(target));
      setAccountId(target.accountId ?? '');
    } else {
      setSelectedId('');
      setSourceId(sources[0]?.id ?? '');
      setAccountId('');
    }
  }, [
    agent.nativeKind,
    existing,
    existingOptionId,
    initializationKey,
    keyOptions,
    loading,
    open,
    presetOptionId,
    sources,
  ]);

  const activeSource = sources.find((source) => source.id === sourceId);
  const accounts = useMemo<AccountSource[]>(() => {
    if (!activeSource || activeSource.group !== 'relay') return [];
    const accountMap = new Map<string, KeyOption[]>();
    for (const option of activeSource.keys) {
      if (!option.accountId) continue;
      const list = accountMap.get(option.accountId) ?? [];
      list.push(option);
      accountMap.set(option.accountId, list);
    }
    return Array.from(accountMap, ([id, keys]) => ({
      id,
      label: keys[0]?.accountName ?? id,
      keys,
    }));
  }, [activeSource]);
  const activeAccount = accounts.find((account) => account.id === accountId);
  const activeKeys =
    activeSource?.group === 'independent' ? activeSource.keys : activeAccount?.keys ?? [];

  useEffect(() => {
    if (!activeSource || activeSource.group !== 'relay') {
      if (accountId) setAccountId('');
      return;
    }
    if (accounts.some((account) => account.id === accountId)) return;
    setAccountId(accounts.length === 1 ? accounts[0].id : '');
  }, [accountId, accounts, activeSource]);

  const selected = keyOptions.find((option) => optionId(option) === selectedId);
  const isAccountToken = selected?.source.type === 'account_token';

  const { data: nodes = [], isLoading: nodesLoading } = useQuery<ProviderNode[]>({
    queryKey: ['provider-nodes', selected?.providerId],
    enabled: !!selected?.providerId,
    queryFn: () => invoke<ProviderNode[]>('get_provider_nodes', { providerId: selected!.providerId }),
  });
  const { data: providerModels = [], isLoading: modelsLoading } = useQuery<string[]>({
    queryKey: ['provider-models', selected?.providerId],
    enabled: !!selected?.providerId,
    queryFn: async () =>
      (await invoke<string[]>('get_cached_provider_models', { providerId: selected!.providerId })) ?? [],
    staleTime: 5 * 60 * 1000,
  });

  useEffect(() => {
    if (isAccountToken && nodes.length > 0 && !node) setNode(nodes[0].base_url);
  }, [isAccountToken, nodes, node]);

  const baseUrl = isAccountToken ? node : selected?.baseUrl ?? '';
  const accessSource = useMemo<AgentAccessSource | null>(() => {
    if (!selected) return null;
    return selected.source.type === 'account_token'
      ? { ...selected.source, base_url: baseUrl || null }
      : selected.source;
  }, [baseUrl, selected]);
  const modelOptions = useMemo(() => {
    if (!isAccountToken || !selected) return [];
    if (!selected.modelLimitsEnabled) return providerModels;
    const allowed = new Set(selected.allowedModels);
    const denied = new Set(selected.deniedModels);
    return providerModels.filter(
      (candidate) => (allowed.size === 0 || allowed.has(candidate)) && !denied.has(candidate)
    );
  }, [isAccountToken, providerModels, selected]);

  const compatibleModelOptions = useMemo(() => {
    if (method === 'gateway' && kind === 'openai_chat') return modelOptions;
    return modelOptions.filter((candidate) => {
      const normalized = candidate.trim().toLowerCase();
      if (agent.id === 'claude_code') {
        return !(
          normalized.startsWith('gpt-') ||
          /^o[134](?:-|$)/.test(normalized) ||
          normalized.includes('codex')
        );
      }
      return !normalized.startsWith('claude-');
    });
  }, [agent.id, kind, method, modelOptions]);

  useEffect(() => {
    if (model && modelOptions.includes(model) && !compatibleModelOptions.includes(model)) {
      setModel('');
      setContext1m(false);
    }
  }, [compatibleModelOptions, model, modelOptions]);

  useEffect(() => {
    if (!model) setContext1m(false);
  }, [model]);

  useEffect(() => {
    if (
      !open ||
      configView !== 'files' ||
      !accessSource ||
      (isAccountToken && !baseUrl)
    ) {
      setPreviewRequest(null);
      return;
    }
    const timer = window.setTimeout(() => {
      setPreviewRequest({
        client: agent.id,
        method,
        source: accessSource,
        model: model || null,
        context1m,
      });
    }, 250);
    return () => window.clearTimeout(timer);
  }, [accessSource, agent.id, baseUrl, configView, context1m, isAccountToken, method, model, open]);

  const previewQuery = useQuery<AgentConfigPreview>({
    queryKey: ['agent-config-preview', previewRequest],
    enabled: !!previewRequest,
    queryFn: () =>
      invoke<AgentConfigPreview>('get_agent_config_preview', { request: previewRequest! }),
    staleTime: 0,
  });

  useEffect(() => {
    setActiveFileIndex(0);
    setCopied(false);
  }, [previewQuery.data]);

  const selectSource = (value: string) => {
    setSourceId(value);
    setAccountId('');
    setSelectedId('');
    setNode('');
    setModel('');
    setContext1m(false);
  };

  const selectAccount = (value: string) => {
    setAccountId(value);
    setSelectedId('');
    setNode('');
    setModel('');
    setContext1m(false);
  };

  const selectKey = (value: string) => {
    setSelectedId(value);
    setNode('');
    setModel('');
    setContext1m(false);
  };

  const confirm = async () => {
    if (!selected) return;
    if (isAccountToken && !baseUrl) {
      toast.error(t('agentConfig.needNode', '请先选择一个节点'));
      return;
    }

    if (!accessSource) return;

    setBusy(true);
    try {
      if (method === 'gateway') {
        await invoke('set_gateway_binding', {
          client: agent.id,
          source: accessSource,
          config: {
            upstreamKind: kind,
            model: model || null,
            context1m,
            impersonate,
          },
        });
        toast.success(t('agentConfig.bound', '已接入（经网关）'));
      } else {
        await directConfigure(agent.id, selected, baseUrl, model || null, context1m);
        await invoke('record_agent_access', {
          record: {
            agent: agent.id,
            method: 'direct',
            label: `${selected.providerName} · ${selected.label}`,
            model: model || null,
            upstream_kind: null,
            impersonate: null,
            source: accessSource,
            context1m,
          },
        });
        toast.success(t('agentConfig.directWritten', '已直接写入配置'));
      }
      onDone();
      onOpenChange(false);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(false);
    }
  };

  const confirmDisabled = busy || loading || !selected || (!!isAccountToken && !baseUrl);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg" className="flex flex-col gap-0 overflow-hidden p-0">
        <DialogHeader className="app-dialog-header">
          <div className="flex flex-wrap items-center gap-2">
            <DialogTitle>
              {t('agentConfig.drawerTitle', '{{name}} 配置', { name: agent.name })}
            </DialogTitle>
            {locked && (
              <Badge variant="secondary" className="gap-1 font-normal">
                <LockKeyhole className="h-3 w-3" />
                {t('agentConfig.keyLocked', '当前 Key 已锁定')}
              </Badge>
            )}
          </div>
        </DialogHeader>

        <div className="app-dialog-body flex-1 p-0">
          {loading ? (
            <div className="m-6 flex min-h-44 items-center justify-center gap-2 rounded-xl border border-dashed border-border/70 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('agentConfig.loadingKeys', '正在加载可用 Key…')}
            </div>
          ) : sources.length === 0 ? (
            <div className="m-6 rounded-xl border border-dashed border-border/70 px-6 py-10 text-center text-sm text-muted-foreground">
              {t('agentConfig.noKeys', '没有可用的 Key。请先添加可用来源或独立 API Key。')}
            </div>
          ) : (
            <div className="grid min-h-full md:grid-cols-[minmax(0,1fr)_minmax(0,1.12fr)]">
              <div className="space-y-5 p-5">
                <FormSection
                number="1"
                title={t('agentConfig.selectKeyTitle', '选择 Key')}
                description={t('agentConfig.selectKeyDesc', '只展示启用中的来源、账号和 Key。')}
                icon={<KeyRound className="h-4 w-4" />}
              >
                <Field label={t('agentConfig.fieldSource', '来源')}>
                    <Select value={sourceId} onValueChange={selectSource} disabled={locked}>
                      <SelectTrigger>
                        <SelectValue placeholder={t('agentConfig.pickSource', '选择来源')} />
                      </SelectTrigger>
                      <SelectContent>
                        {sources
                          .filter((source) => source.group === 'relay')
                          .map((source) => (
                            <SelectItem key={source.id} value={source.id}>
                              {source.label}
                            </SelectItem>
                          ))}
                        {sources.some((source) => source.group === 'independent') && (
                          <SelectItem value={INDEPENDENT_SOURCE_ID}>
                            {t('agentConfig.groupIndependent', '独立 API Key')}
                          </SelectItem>
                        )}
                      </SelectContent>
                    </Select>
                  </Field>

                {activeSource?.group === 'relay' && (
                  <Field label={t('agentConfig.fieldAccount', '账号')}>
                    <Select
                      value={accountId}
                      onValueChange={selectAccount}
                      disabled={locked || accounts.length === 0}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder={t('agentConfig.pickAccount', '选择账号')} />
                      </SelectTrigger>
                      <SelectContent>
                        {accounts.map((account) => (
                          <SelectItem key={account.id} value={account.id}>
                            {account.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </Field>
                )}

                <Field label={t('agentConfig.fieldKey', 'Key')}>
                  <Select
                    value={selectedId}
                    onValueChange={selectKey}
                    disabled={
                      locked ||
                      !activeSource ||
                      (activeSource.group === 'relay' && !activeAccount)
                    }
                  >
                    <SelectTrigger>
                      <SelectValue placeholder={t('agentConfig.pickKey', '选择 Key')} />
                    </SelectTrigger>
                    <SelectContent>
                      {activeKeys.map((option) => (
                        <SelectItem key={optionId(option)} value={optionId(option)}>
                          <span className="flex min-w-0 items-center gap-2">
                            <span className="truncate font-medium">{option.label}</span>
                            <code className="shrink-0 text-xs text-muted-foreground">
                              {option.masked}
                            </code>
                          </span>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>

                </FormSection>

                <FormSection
                  number="2"
                  title={t('agentConfig.methodTitle', '接入方式')}
                  description={t('agentConfig.methodDesc', '原生兼容时直接写入；需要协议转换时使用本地网关。')}
                  icon={<SlidersHorizontal className="h-4 w-4" />}
                >
                  <div className="grid gap-3 sm:grid-cols-2">
                    <MethodOption
                      active={method === 'direct'}
                      icon={<Server className="h-4 w-4" />}
                      title={t('agentConfig.methodDirect', '直接写入配置')}
                      onClick={() => setMethod('direct')}
                    />
                    <MethodOption
                      active={method === 'gateway'}
                      icon={<ShieldCheck className="h-4 w-4" />}
                      title={t('agentConfig.methodGateway', '经本地网关')}
                      onClick={() => setMethod('gateway')}
                    />
                  </div>
                </FormSection>
              </div>

              <div className="min-w-0 p-5">
                <Tabs
                  value={configView}
                  onValueChange={(value) => setConfigView(value as ConfigView)}
                  className="min-w-0"
                >
                  <TabsList className="mb-4 grid w-full grid-cols-2">
                    <TabsTrigger value="form">
                      <SlidersHorizontal className="mr-2 h-4 w-4" />
                      {t('agentConfig.visualConfig', '可视化配置')}
                    </TabsTrigger>
                    <TabsTrigger value="files">
                      <FileCode2 className="mr-2 h-4 w-4" />
                      {t('agentConfig.configFiles', '配置文件')}
                    </TabsTrigger>
                  </TabsList>

                  {configView === 'form' ? (
                    <FormSection
                      number="3"
                      title={t('agentConfig.runtimeTitle', '运行参数')}
                      description={t('agentConfig.runtimeDesc', '设置节点和默认模型；留空模型时沿用 Agent 自身选择。')}
                      icon={<SlidersHorizontal className="h-4 w-4" />}
                    >
                      <div className="grid gap-4 sm:grid-cols-2">
                        <Field label={t('agentConfig.fieldNode', '节点')}>
                          {isAccountToken ? (
                            <Select value={node} onValueChange={setNode} disabled={nodesLoading}>
                              <SelectTrigger>
                                <SelectValue placeholder={t('agentConfig.pickNode', '选择节点')} />
                              </SelectTrigger>
                              <SelectContent>
                                {nodes.map((providerNode) => (
                                  <SelectItem key={providerNode.base_url} value={providerNode.base_url}>
                                    <span className="flex min-w-0 items-center gap-2">
                                      <span className="truncate">{providerNode.name}</span>
                                      <code className="max-w-72 truncate text-xs text-muted-foreground">
                                        {providerNode.base_url}
                                      </code>
                                    </span>
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          ) : (
                            <Input value={selected?.baseUrl ?? ''} disabled placeholder={t('agentConfig.notApplicable', '不适用')} />
                          )}
                        </Field>

                        <Field label={t('agentConfig.fieldModel', '模型（可选）')}>
                          {isAccountToken && modelOptions.length > 0 ? (
                            <Select value={model} onValueChange={setModel} disabled={modelsLoading}>
                              <SelectTrigger>
                                <SelectValue placeholder={t('agentConfig.pickModel', '默认（不指定）')} />
                              </SelectTrigger>
                              <SelectContent>
                                {compatibleModelOptions.map((modelName) => (
                                  <SelectItem key={modelName} value={modelName}>
                                    {modelName}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          ) : (
                            <Input
                              value={model}
                              onChange={(event) => setModel(event.target.value)}
                              placeholder={t('agentConfig.modelPlaceholder', '默认（留空自动）')}
                              disabled={!selected || modelsLoading}
                            />
                          )}
                        </Field>
                      </div>

                      <SettingRow
                        title={t('agentConfig.context1m', '1M 上下文')}
                        description={
                          method === 'gateway'
                            ? t('agentConfig.context1mGatewayHint', '随网关绑定保存，并写入 Agent 接入配置。')
                            : t('agentConfig.context1mDirectHint', '直接写入 Agent 配置；需要先选择模型。')
                        }
                      >
                        <Switch checked={context1m} onCheckedChange={setContext1m} disabled={!model} />
                      </SettingRow>

                      {method === 'gateway' && (
                        <Accordion type="single" collapsible className="rounded-lg bg-muted/20 px-3">
                          <AccordionItem value="gateway-advanced" className="border-b-0">
                            <AccordionTrigger className="py-3 text-sm hover:no-underline">
                              <span className="text-left">
                                <span className="block font-medium">
                                  {t('agentConfig.advancedTitle', '网关高级参数')}
                                </span>
                                <span className="mt-0.5 block text-xs font-normal text-muted-foreground">
                                  {t('agentConfig.advancedDesc', '协议识别、客户端兼容等扩展设置')}
                                </span>
                              </span>
                            </AccordionTrigger>
                            <AccordionContent className="space-y-4">
                              <Field label={t('agentConfig.upstreamKind', '上游协议')}>
                                <div className="grid gap-2 sm:grid-cols-2">
                                  <KindRadio
                                    active={kind === agent.nativeKind}
                                    label={
                                      agent.id === 'codex'
                                        ? t('agentConfig.kindResponses', 'OpenAI Responses（原生透传）')
                                        : t('agentConfig.kindAnthropic', 'Anthropic（原生透传）')
                                    }
                                    onClick={() => setKind(agent.nativeKind)}
                                  />
                                  <KindRadio
                                    active={kind === 'openai_chat'}
                                    label={t('agentConfig.kindChat', 'OpenAI Chat（网关转换）')}
                                    onClick={() => setKind('openai_chat')}
                                  />
                                </div>
                              </Field>

                              <SettingRow
                                title={t('agentConfig.impersonateLabel', '伪装为 {{name}}', { name: agent.name })}
                                description={t('agentConfig.impersonateDesc', '让接入服务把请求识别为标准客户端。')}
                              >
                                <Switch checked={impersonate} onCheckedChange={setImpersonate} />
                              </SettingRow>
                            </AccordionContent>
                          </AccordionItem>
                        </Accordion>
                      )}
                    </FormSection>
                  ) : (
                    <ConfigPreviewPanel
                      preview={previewQuery.data}
                      loading={previewQuery.isLoading || (!!previewRequest && previewQuery.isFetching)}
                      error={previewQuery.error}
                      method={method}
                      fileVersion={fileVersion}
                      onFileVersionChange={setFileVersion}
                      activeFileIndex={activeFileIndex}
                      onActiveFileIndexChange={setActiveFileIndex}
                      copied={copied}
                      onCopy={async (content) => {
                        try {
                          await navigator.clipboard.writeText(content);
                          setCopied(true);
                          toast.success(t('common.copied', '已复制！'));
                          window.setTimeout(() => setCopied(false), 1500);
                        } catch {
                          toast.error(t('token.copyError', '复制失败'));
                        }
                      }}
                    />
                  )}
                </Tabs>
              </div>
            </div>
          )}
        </div>

        <DialogFooter className="app-dialog-footer">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={busy}>
            {t('common.cancel', '取消')}
          </Button>
          <Button onClick={confirm} disabled={confirmDisabled}>
            {busy && <Loader2 className="h-4 w-4 animate-spin" />}
            {t('agentConfig.confirmAccess', '确认接入')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ConfigPreviewPanel({
  preview,
  loading,
  error,
  method,
  fileVersion,
  onFileVersionChange,
  activeFileIndex,
  onActiveFileIndexChange,
  copied,
  onCopy,
}: {
  preview?: AgentConfigPreview;
  loading: boolean;
  error: Error | null;
  method: Method;
  fileVersion: FileVersion;
  onFileVersionChange: (value: FileVersion) => void;
  activeFileIndex: number;
  onActiveFileIndexChange: (value: number) => void;
  copied: boolean;
  onCopy: (content: string) => Promise<void>;
}) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="flex min-h-80 items-center justify-center rounded-xl bg-muted/15 text-sm text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        {t('agentConfig.previewLoading', '正在生成配置预览…')}
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-xl bg-destructive/5 px-5 py-8 text-center text-sm text-destructive">
        <p className="font-medium">{t('agentConfig.previewFailed', '配置预览生成失败')}</p>
        <p className="mt-2 break-words text-xs opacity-80">{String(error)}</p>
      </div>
    );
  }

  if (!preview?.files.length) {
    return (
      <div className="flex min-h-80 items-center justify-center rounded-xl bg-muted/15 px-6 text-center text-sm text-muted-foreground">
        {t('agentConfig.previewEmpty', '选择 Key 和节点后即可查看完整配置文件。')}
      </div>
    );
  }

  const activeFile = preview.files[Math.min(activeFileIndex, preview.files.length - 1)];
  const content = fileVersion === 'current' ? activeFile.current_content : activeFile.next_content;
  const statusText =
    method === 'direct'
      ? t('agentConfig.previewDirectStatus', '确认后会直接写入以下配置')
      : preview.gateway_running
        ? preview.takeover_active
          ? t('agentConfig.previewTakenOver', '网关正在接管此 Agent 配置')
          : t('agentConfig.previewWillTakeOver', '确认后网关会接管此 Agent 配置')
        : t('agentConfig.previewGatewayStopped', '网关未启动，保存绑定暂不改写文件')
  const statusHint =
    method === 'gateway' && !preview.gateway_running
      ? t('agentConfig.previewGatewayStoppedHint', '启动网关时会自动写入本地网关地址。')
      : t('agentConfig.previewRedacted', '敏感 Token 已自动脱敏，预览不会修改磁盘。');

  return (
    <div className="space-y-3">
      <div className="rounded-xl bg-muted/20 px-4 py-3">
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant={preview.takeover_active ? 'default' : 'secondary'} className="font-normal">
            {method === 'gateway'
              ? preview.takeover_active
                ? t('agentConfig.takeoverActive', '网关接管中')
                : t('agentConfig.takeoverInactive', '未接管')
              : t('agentConfig.directPreview', '直接写入')}
          </Badge>
          <span className="text-sm font-medium">{statusText}</span>
        </div>
        <p className="mt-1.5 text-xs leading-5 text-muted-foreground">{statusHint}</p>
      </div>

      <div className="rounded-xl bg-muted/15 p-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex min-w-0 flex-wrap gap-1.5">
            {preview.files.map((file, index) => (
              <Button
                key={file.path}
                type="button"
                size="sm"
                variant={index === activeFileIndex ? 'secondary' : 'ghost'}
                className="h-8 max-w-52 font-mono text-xs"
                onClick={() => onActiveFileIndexChange(index)}
              >
                <span className="truncate">{file.path.split('/').pop()}</span>
              </Button>
            ))}
          </div>
          <div className="flex rounded-lg bg-background/80 p-1">
            <Button
              type="button"
              size="sm"
              variant={fileVersion === 'current' ? 'secondary' : 'ghost'}
              className="h-7 px-2.5 text-xs"
              onClick={() => onFileVersionChange('current')}
            >
              {t('agentConfig.currentConfig', '当前配置')}
            </Button>
            <Button
              type="button"
              size="sm"
              variant={fileVersion === 'next' ? 'secondary' : 'ghost'}
              className="h-7 px-2.5 text-xs"
              onClick={() => onFileVersionChange('next')}
            >
              {t('agentConfig.nextConfig', '保存后')}
            </Button>
          </div>
        </div>

        <div className="mt-3 overflow-hidden rounded-lg bg-slate-950 text-slate-100 shadow-inner dark:bg-black/45">
          <div className="flex items-center justify-between gap-3 bg-white/5 px-3 py-2">
            <code className="min-w-0 truncate text-[11px] text-slate-300">{activeFile.path}</code>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="h-7 w-7 shrink-0 text-slate-300 hover:bg-white/10 hover:text-white"
              onClick={() => void onCopy(content)}
              disabled={!content}
              aria-label={t('agentConfig.copyConfig', '复制配置')}
            >
              {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            </Button>
          </div>
          <pre className="max-h-[46vh] min-h-64 overflow-auto p-4 font-mono text-xs leading-5">
            <code>{content || t('agentConfig.fileNotFound', '文件尚不存在')}</code>
          </pre>
        </div>
      </div>
    </div>
  );
}

function FormSection({
  number,
  title,
  description,
  icon,
  children,
}: {
  number: string;
  title: string;
  description: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-3 rounded-xl bg-muted/15 p-4">
      <div className="flex items-start gap-3">
        <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-bold text-primary">
          {number}
        </span>
        <div className="min-w-0 flex-1">
          <h3 className="flex items-center gap-2 text-sm font-semibold">
            {icon}
            {title}
          </h3>
          <p className="mt-0.5 text-xs leading-4 text-muted-foreground">{description}</p>
        </div>
      </div>
      <div className="space-y-3 pl-0 sm:pl-10">{children}</div>
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-2">
      <label className="text-sm font-medium text-foreground/85">{label}</label>
      {children}
    </div>
  );
}

function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex min-h-14 items-center justify-between gap-3 rounded-lg bg-muted/25 px-3 py-2.5">
      <div className="min-w-0">
        <p className="text-sm font-medium">{title}</p>
        <p className="mt-0.5 text-xs leading-4 text-muted-foreground">{description}</p>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function MethodOption({
  active,
  icon,
  title,
  onClick,
}: {
  active: boolean;
  icon: React.ReactNode;
  title: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'flex min-h-11 w-full items-center gap-2.5 rounded-lg border px-3 py-2 text-left transition-colors',
        active
          ? 'border-primary/60 bg-primary/5 ring-1 ring-primary/15'
          : 'border-border/60 bg-background hover:border-primary/30 hover:bg-muted/30'
      )}
    >
      <span
        className={cn(
          'mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border',
          active ? 'border-primary' : 'border-muted-foreground/40'
        )}
      >
        {active && <span className="h-2.5 w-2.5 rounded-full bg-primary" />}
      </span>
      <span className="min-w-0 flex-1">
        <span className={cn('flex items-center gap-2 text-sm font-medium', active && 'text-primary')}>
          {icon}
          {title}
        </span>
      </span>
    </button>
  );
}

function KindRadio({ active, label, onClick }: { active: boolean; label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'flex min-h-11 items-center gap-2 rounded-lg border px-3 py-2 text-left text-sm transition-colors',
        active
          ? 'border-primary/50 bg-primary/5 text-primary'
          : 'border-border/60 bg-background hover:bg-muted/40'
      )}
    >
      <span
        className={cn(
          'h-3.5 w-3.5 shrink-0 rounded-full border',
          active ? 'border-primary bg-primary shadow-[inset_0_0_0_3px_hsl(var(--background))]' : 'border-muted-foreground/40'
        )}
      />
      {label}
    </button>
  );
}

async function directConfigure(
  agent: string,
  option: KeyOption,
  baseUrl: string,
  model: string | null,
  context1m: boolean
): Promise<void> {
  if (option.source.type === 'independent_key') {
    const command =
      agent === 'claude_code' ? 'configure_independent_key_claude' : 'configure_independent_key_codex';
    await invoke<string>(command, { keyId: option.source.key_id, model, context1m });
    return;
  }

  if (agent === 'claude_code') {
    await invoke<string>('configure_claude_global', {
      tokenId: option.source.token_id,
      accountId: option.source.account_id,
      baseUrl,
      model,
      context1m,
    });
  } else {
    await invoke<string>('configure_codex_global', {
      tokenId: option.source.token_id,
      accountId: option.source.account_id,
      providerId: option.source.provider_id,
      baseUrl,
      model,
      context1m,
    });
  }
}
