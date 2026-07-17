import { useEffect, useMemo, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import {
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
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
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

const INDEPENDENT_SOURCE_ID = 'independent';

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

  const locked = !!presetOptionId;

  const sources = useMemo<KeySource[]>(() => {
    const relay = new Map<string, KeyOption[]>();
    const independent: KeyOption[] = [];
    for (const option of keyOptions) {
      if (option.sourceGroup === 'relay') {
        const list = relay.get(option.providerName) ?? [];
        list.push(option);
        relay.set(option.providerName, list);
      } else {
        independent.push(option);
      }
    }

    const list: KeySource[] = Array.from(relay, ([provider, keys]) => ({
      id: `relay:${provider}`,
      label: provider,
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
      setSourceId(
        target.sourceGroup === 'relay' ? `relay:${target.providerName}` : INDEPENDENT_SOURCE_ID
      );
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
  const isRelay = activeSource?.group === 'relay';

  const accounts = useMemo(() => {
    if (!activeSource || activeSource.group !== 'relay') return [];
    const map = new Map<string, string>();
    for (const option of activeSource.keys) {
      if (option.accountId) map.set(option.accountId, option.accountName ?? option.accountId);
    }
    return Array.from(map, ([id, name]) => ({ id, name }));
  }, [activeSource]);

  const activeKeys = useMemo(() => {
    if (!activeSource) return [];
    if (activeSource.group !== 'relay') return activeSource.keys;
    if (!accountId) return [];
    return activeSource.keys.filter((option) => option.accountId === accountId);
  }, [activeSource, accountId]);

  useEffect(() => {
    if (isRelay && accounts.length === 1 && !accountId) setAccountId(accounts[0].id);
  }, [isRelay, accounts, accountId]);

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
  const modelOptions = useMemo(() => {
    if (!isAccountToken || !selected) return [];
    if (!selected.modelLimitsEnabled) return providerModels;
    const allowed = new Set(selected.allowedModels);
    const denied = new Set(selected.deniedModels);
    return providerModels.filter(
      (candidate) => (allowed.size === 0 || allowed.has(candidate)) && !denied.has(candidate)
    );
  }, [isAccountToken, providerModels, selected]);

  useEffect(() => {
    if (!model) setContext1m(false);
  }, [model]);

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

    const accessSource: AgentAccessSource =
      selected.source.type === 'account_token'
        ? { ...selected.source, base_url: baseUrl || null }
        : selected.source;

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
              {t('agentConfig.drawerTitle', '为 {{name}} 接入 Key', { name: agent.name })}
            </DialogTitle>
            {locked && (
              <Badge variant="secondary" className="gap-1 font-normal">
                <LockKeyhole className="h-3 w-3" />
                {t('agentConfig.keyLocked', '当前 Key 已锁定')}
              </Badge>
            )}
          </div>
          <DialogDescription>
            {t(
              'agentConfig.drawerDesc',
              '选择 Key 和运行参数，再决定直接写入工具配置，还是经本地网关适配。'
            )}
          </DialogDescription>
        </DialogHeader>

        <div className="app-dialog-body flex-1 p-0">
          {loading ? (
            <div className="m-6 flex min-h-44 items-center justify-center gap-2 rounded-xl border border-dashed border-border/70 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('agentConfig.loadingKeys', '正在加载可用 Key…')}
            </div>
          ) : sources.length === 0 ? (
            <div className="m-6 rounded-xl border border-dashed border-border/70 px-6 py-10 text-center text-sm text-muted-foreground">
              {t('agentConfig.noKeys', '没有可用的 Key。先到中转站或 API Key 里添加。')}
            </div>
          ) : (
            <div className="grid min-h-full md:grid-cols-[minmax(0,0.92fr)_minmax(0,1.08fr)]">
              <div className="p-5 md:border-r md:border-border/60">
                <FormSection
                number="1"
                title={t('agentConfig.selectKeyTitle', '选择 Key')}
                description={t('agentConfig.selectKeyDesc', '只展示启用中的账号和 Key。')}
                icon={<KeyRound className="h-4 w-4" />}
              >
                <div className="grid gap-4 sm:grid-cols-2">
                  <Field label={t('agentConfig.fieldSource', '来源')}>
                    <Select value={sourceId} onValueChange={selectSource} disabled={locked}>
                      <SelectTrigger>
                        <SelectValue placeholder={t('agentConfig.pickSource', '选择来源')} />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectGroup>
                          <SelectLabel>{t('agentConfig.groupRelay', '中转站账号')}</SelectLabel>
                          {sources
                            .filter((source) => source.group === 'relay')
                            .map((source) => (
                              <SelectItem key={source.id} value={source.id}>
                                {source.label}
                              </SelectItem>
                            ))}
                        </SelectGroup>
                        {sources.some((source) => source.group === 'independent') && (
                          <SelectGroup>
                            <SelectLabel>
                              {t('agentConfig.groupIndependent', '独立 API Key')}
                            </SelectLabel>
                            {sources
                              .filter((source) => source.group === 'independent')
                              .map((source) => (
                                <SelectItem key={source.id} value={source.id}>
                                  {source.label}
                                </SelectItem>
                              ))}
                          </SelectGroup>
                        )}
                      </SelectContent>
                    </Select>
                  </Field>

                  <Field label={t('agentConfig.fieldAccount', '账号')}>
                    <Select
                      value={accountId}
                      onValueChange={selectAccount}
                      disabled={locked || !isRelay}
                    >
                      <SelectTrigger>
                        <SelectValue
                          placeholder={
                            isRelay
                              ? t('agentConfig.pickAccount', '选择账号')
                              : t('agentConfig.notApplicable', '不适用')
                          }
                        />
                      </SelectTrigger>
                      <SelectContent>
                        {accounts.map((account) => (
                          <SelectItem key={account.id} value={account.id}>
                            {account.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </Field>
                </div>

                <Field label={t('agentConfig.fieldKey', 'Key')}>
                  <Select
                    value={selectedId}
                    onValueChange={selectKey}
                    disabled={locked || !activeSource || (!!isRelay && !accountId)}
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

                {selected && (
                  <div className="grid gap-3 rounded-lg border border-border/60 bg-muted/20 p-3 text-sm sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
                    <div className="min-w-0">
                      <p className="truncate font-semibold">{selected.label}</p>
                      <p className="mt-1 truncate text-sm text-muted-foreground">
                        {[selected.providerName, selected.accountName].filter(Boolean).join(' · ')}
                      </p>
                    </div>
                    <div className="flex flex-wrap items-center gap-2 sm:justify-end">
                      <code className="rounded-md bg-background px-2 py-1 text-xs text-muted-foreground ring-1 ring-border/60">
                        {selected.masked}
                      </code>
                      {selected.balance && (
                        <Badge variant="outline">
                          {t('agentConfig.balance', '余额')} {selected.balance}
                        </Badge>
                      )}
                    </div>
                  </div>
                )}
                </FormSection>
              </div>

              <div className="space-y-4 p-5">
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
                      hint={t('agentConfig.methodDirectHint', '中转站原生支持时最简单')}
                      onClick={() => setMethod('direct')}
                    />
                    <MethodOption
                      active={method === 'gateway'}
                      icon={<ShieldCheck className="h-4 w-4" />}
                      title={t('agentConfig.methodGateway', '经本地网关')}
                      hint={t('agentConfig.methodGatewayHint', '需要协议适配 / 伪装时')}
                      onClick={() => setMethod('gateway')}
                    />
                  </div>
                </FormSection>

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
                          {modelOptions.map((modelName) => (
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

                  {method === 'gateway' && (
                  <div className="space-y-4">
                    <Field label={t('agentConfig.upstreamKind', '中转站上游协议')}>
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
                  </div>
                  )}

                  <div className={cn('grid gap-3', method === 'gateway' && 'sm:grid-cols-2')}>
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
                      <SettingRow
                        title={t('agentConfig.impersonateLabel', '伪装为 {{name}}', { name: agent.name })}
                        description={t('agentConfig.impersonateDesc', '让中转站把请求识别为标准客户端。')}
                      >
                        <Switch checked={impersonate} onCheckedChange={setImpersonate} />
                      </SettingRow>
                    )}
                  </div>
                </FormSection>
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
    <section className="app-dialog-section space-y-3">
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
  hint,
  onClick,
}: {
  active: boolean;
  icon: React.ReactNode;
  title: string;
  hint: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'flex min-h-16 w-full items-start gap-3 rounded-lg border p-3 text-left transition-colors',
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
        <span className={cn('flex items-center gap-2 text-sm font-semibold', active && 'text-primary')}>
          {icon}
          {title}
        </span>
        <span className="mt-1 block text-xs leading-4 text-muted-foreground">{hint}</span>
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
