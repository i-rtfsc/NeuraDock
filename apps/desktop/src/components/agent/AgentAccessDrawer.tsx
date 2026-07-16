import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Server, ShieldCheck, KeyRound } from 'lucide-react';

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
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/utils';
import type { ProviderNode } from '@/types/token';
import type { AgentDef, AgentAccess } from './AgentConfigGateway';

type UpstreamKind = 'openai_chat' | 'openai_responses' | 'anthropic';

/** A selectable key, from either a relay account token or an independent key. */
export interface KeyOption {
  label: string;
  masked: string;
  providerName: string;
  /** Relay provider id (for account tokens; drives node/model lookup). */
  providerId: string | null;
  /** Self-contained base URL (independent keys only). */
  baseUrl: string | null;
  /** Relay account this key belongs to (null for independent keys). */
  accountId: string | null;
  accountName: string | null;
  sourceGroup: 'relay' | 'independent';
  balance: string | null;
  source:
    | { type: 'account_token'; account_id: string; provider_id: string; token_id: number }
    | { type: 'independent_key'; key_id: number };
}

type Method = 'direct' | 'gateway';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  agent: AgentDef;
  keyOptions: KeyOption[];
  existing?: AgentAccess;
  /** When opened from a specific token (account detail), lock the key to it. */
  presetOptionId?: string;
  onDone: () => void;
}

function optionId(o: KeyOption): string {
  return o.source.type === 'account_token'
    ? `at:${o.source.account_id}:${o.source.token_id}`
    : `ik:${o.source.key_id}`;
}

/** A source is one relay station (grouped by provider name) or the independent-key bucket. */
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
  existing,
  presetOptionId,
  onDone,
}: Props) {
  const { t } = useTranslation();
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

  // When opened for a specific token, the key is fixed — hide the pickers.
  const locked = !!presetOptionId;

  // Group keys into selectable sources: one per relay provider, plus a single
  // "独立 API Key" bucket. The first dropdown picks a source, the second picks a
  // key within it.
  const sources = useMemo<KeySource[]>(() => {
    const relay = new Map<string, KeyOption[]>();
    const independent: KeyOption[] = [];
    for (const o of keyOptions) {
      if (o.sourceGroup === 'relay') {
        const list = relay.get(o.providerName) ?? [];
        list.push(o);
        relay.set(o.providerName, list);
      } else {
        independent.push(o);
      }
    }
    const list: KeySource[] = [];
    for (const [provider, keys] of relay) {
      list.push({ id: `relay:${provider}`, label: provider, group: 'relay', keys });
    }
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

  useEffect(() => {
    if (!open) return;
    setMethod(existing?.method ?? 'gateway');
    setKind((existing?.upstream_kind as UpstreamKind) ?? agent.nativeKind);
    setImpersonate(existing?.impersonate ?? true);
    setModel('');
    setNode('');
    setContext1m(false);
    if (presetOptionId) {
      // Locked to a specific token: derive its source/account so lookups work.
      const preset = keyOptions.find((o) => optionId(o) === presetOptionId);
      setSelectedId(presetOptionId);
      setSourceId(preset?.sourceGroup === 'relay' ? `relay:${preset.providerName}` : INDEPENDENT_SOURCE_ID);
      setAccountId(preset?.accountId ?? '');
    } else {
      setSourceId(sources[0]?.id ?? '');
      setAccountId('');
      setSelectedId('');
    }
    // sources is derived from keyOptions; intentionally not a dep to avoid
    // resetting the picker while the dialog is open and lists refetch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, agent.nativeKind, existing, presetOptionId]);

  const activeSource = sources.find((s) => s.id === sourceId);
  const isRelay = activeSource?.group === 'relay';

  // Relay sources are grouped by provider, but one provider can hold several
  // accounts. Offer an account picker between source and key. Independent keys
  // have no account level.
  const accounts = useMemo(() => {
    if (!activeSource || activeSource.group !== 'relay') return [];
    const map = new Map<string, string>();
    for (const o of activeSource.keys) {
      if (o.accountId) map.set(o.accountId, o.accountName ?? o.accountId);
    }
    return Array.from(map, ([id, name]) => ({ id, name }));
  }, [activeSource]);

  // Keys shown in the last dropdown: for relay, scoped to the chosen account;
  // for independent, the whole bucket.
  const activeKeys = useMemo(() => {
    if (!activeSource) return [];
    if (activeSource.group !== 'relay') return activeSource.keys;
    if (!accountId) return [];
    return activeSource.keys.filter((o) => o.accountId === accountId);
  }, [activeSource, accountId]);

  // Auto-select the sole account so a single-account provider needs no click.
  useEffect(() => {
    if (isRelay && accounts.length === 1 && !accountId) setAccountId(accounts[0].id);
  }, [isRelay, accounts, accountId]);

  const selected = keyOptions.find((o) => optionId(o) === selectedId);
  const isAccountToken = selected?.source.type === 'account_token';

  // Account tokens carry no base_url of their own; the relay provider exposes
  // one or more nodes to choose from. Independent keys are self-contained.
  const { data: nodes = [] } = useQuery<ProviderNode[]>({
    queryKey: ['provider-nodes', selected?.providerId],
    enabled: !!selected?.providerId,
    queryFn: () => invoke<ProviderNode[]>('get_provider_nodes', { providerId: selected!.providerId }),
  });
  const { data: providerModels = [] } = useQuery<string[]>({
    queryKey: ['provider-models', selected?.providerId],
    enabled: !!selected?.providerId,
    queryFn: async () =>
      (await invoke<string[]>('get_cached_provider_models', { providerId: selected!.providerId })) ?? [],
    staleTime: 5 * 60 * 1000,
  });

  // Auto-select the first node so a single-node provider needs no click.
  useEffect(() => {
    if (isAccountToken && nodes.length > 0 && !node) setNode(nodes[0].base_url);
  }, [isAccountToken, nodes, node]);

  // Resolved base_url for direct write: node for account tokens, own for independent.
  const baseUrl = isAccountToken ? node : selected?.baseUrl ?? '';
  const modelOptions = isAccountToken ? providerModels : [];
  // 1M only applies when a concrete model is chosen.
  useEffect(() => {
    if (!model) setContext1m(false);
  }, [model]);

  const confirm = async () => {
    if (!selected) return;
    // Account tokens need a node (base_url) resolved before either path.
    if (isAccountToken && !baseUrl) {
      toast.error(t('agentConfig.needNode', '请先选择一个节点'));
      return;
    }
    setBusy(true);
    try {
      if (method === 'gateway') {
        const source =
          selected.source.type === 'account_token'
            ? { ...selected.source, base_url: baseUrl || null }
            : selected.source;
        await invoke('set_gateway_binding', {
          client: agent.id,
          source,
          upstreamKind: kind,
          model: model || null,
          impersonate,
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
          },
        });
        toast.success(t('agentConfig.directWritten', '已直接写入配置'));
      }
      onDone();
      onOpenChange(false);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('agentConfig.drawerTitle', '为 {{name}} 接入 Key', { name: agent.name })}</DialogTitle>
          <DialogDescription>
            {t('agentConfig.drawerDesc', '选择一个 Key，然后决定直接写入工具配置，还是经本地网关适配。')}
          </DialogDescription>
        </DialogHeader>

        {sources.length === 0 ? (
          <p className="rounded-lg border border-border/60 p-6 text-sm text-muted-foreground text-center">
            {t('agentConfig.noKeys', '没有可用的 Key。先到中转站或 API Key 里添加。')}
          </p>
        ) : (
          <div className="grid gap-6 md:grid-cols-2">
            {/* ① Key selection */}
            <section className="space-y-4">
              <h4 className="text-xs font-semibold text-muted-foreground tracking-wide">
                {t('agentConfig.step1', '① 选择 Key')}
              </h4>

              <Field label={t('agentConfig.fieldSource', '来源')} hidden={locked}>
                <Select
                  value={sourceId}
                  onValueChange={(v) => {
                    setSourceId(v);
                    setAccountId('');
                    setSelectedId('');
                  }}
                >
                  <SelectTrigger className="h-9 text-sm">
                    <SelectValue placeholder={t('agentConfig.pickSource', '选择来源')} />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectLabel>{t('agentConfig.groupRelay', '中转站账号')}</SelectLabel>
                      {sources
                        .filter((s) => s.group === 'relay')
                        .map((s) => (
                          <SelectItem key={s.id} value={s.id}>
                            {s.label}
                          </SelectItem>
                        ))}
                    </SelectGroup>
                    {sources.some((s) => s.group === 'independent') && (
                      <SelectGroup>
                        <SelectLabel>{t('agentConfig.groupIndependent', '独立 API Key')}</SelectLabel>
                        {sources
                          .filter((s) => s.group === 'independent')
                          .map((s) => (
                            <SelectItem key={s.id} value={s.id}>
                              {s.label}
                            </SelectItem>
                          ))}
                      </SelectGroup>
                    )}
                  </SelectContent>
                </Select>
              </Field>

              {isRelay && (
                <Field label={t('agentConfig.fieldAccount', '账号')} hidden={locked}>
                  <Select
                    value={accountId}
                    onValueChange={(v) => {
                      setAccountId(v);
                      setSelectedId('');
                    }}
                  >
                    <SelectTrigger className="h-9 text-sm">
                      <SelectValue placeholder={t('agentConfig.pickAccount', '选择账号')} />
                    </SelectTrigger>
                    <SelectContent>
                      {accounts.map((a) => (
                        <SelectItem key={a.id} value={a.id}>
                          {a.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              )}

              <Field label={t('agentConfig.fieldKey', 'Key')} hidden={locked}>
                <Select
                  value={selectedId}
                  onValueChange={setSelectedId}
                  disabled={!activeSource || (isRelay && !accountId)}
                >
                  <SelectTrigger className="h-9 text-sm">
                    <SelectValue placeholder={t('agentConfig.pickKey', '选择 Key')} />
                  </SelectTrigger>
                  <SelectContent>
                    {activeKeys.map((o) => {
                      const id = optionId(o);
                      return (
                        <SelectItem key={id} value={id}>
                          <span className="flex items-center gap-2">
                            <span className="font-medium">{o.label}</span>
                            <code className="text-[10px] text-muted-foreground">{o.masked}</code>
                          </span>
                        </SelectItem>
                      );
                    })}
                  </SelectContent>
                </Select>
              </Field>

              {/* Selected-key preview so it's clear which key was picked */}
              {selected && (
                <div className="rounded-lg border border-border/60 bg-muted/10 p-3 space-y-1.5">
                  <div className="flex items-center gap-2">
                    <KeyRound className="h-3.5 w-3.5 text-primary shrink-0" />
                    <span className="text-sm font-medium truncate">{selected.label}</span>
                  </div>
                  <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-[11px] text-muted-foreground">
                    <dt>{t('agentConfig.previewProvider', '提供商')}</dt>
                    <dd className="truncate text-foreground/80">{selected.providerName}</dd>
                    {selected.accountName && (
                      <>
                        <dt>{t('agentConfig.previewAccount', '账号')}</dt>
                        <dd className="truncate text-foreground/80">{selected.accountName}</dd>
                      </>
                    )}
                    <dt>{t('agentConfig.previewKey', 'Key')}</dt>
                    <dd className="font-mono text-foreground/80">{selected.masked}</dd>
                    {selected.balance && (
                      <>
                        <dt>{t('agentConfig.balance', '余额')}</dt>
                        <dd className="text-foreground/80">{selected.balance}</dd>
                      </>
                    )}
                  </dl>
                </div>
              )}
            </section>

            {/* ② Method + ③ Gateway options */}
            <section className="space-y-4">
              <h4 className="text-xs font-semibold text-muted-foreground tracking-wide">
                {t('agentConfig.step2', '② 接入方式')}
              </h4>
              <div className="space-y-2">
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

              {/* Node (account tokens) + model + 1M, shared by both methods. */}
              {selected && (
                <div className="space-y-3 rounded-lg border border-border/60 bg-muted/10 p-3">
                  {isAccountToken && (
                    <Field label={t('agentConfig.fieldNode', '节点')}>
                      <Select value={node} onValueChange={setNode}>
                        <SelectTrigger className="h-9 text-sm">
                          <SelectValue placeholder={t('agentConfig.pickNode', '选择节点')} />
                        </SelectTrigger>
                        <SelectContent>
                          {nodes.map((n) => (
                            <SelectItem key={n.base_url} value={n.base_url}>
                              <span className="flex items-center gap-2">
                                <span>{n.name}</span>
                                <code className="text-[10px] text-muted-foreground">{n.base_url}</code>
                              </span>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </Field>
                  )}
                  <Field label={t('agentConfig.fieldModel', '模型（可选）')}>
                    {modelOptions.length > 0 ? (
                      <Select value={model} onValueChange={setModel}>
                        <SelectTrigger className="h-9 text-sm">
                          <SelectValue placeholder={t('agentConfig.pickModel', '默认（不指定）')} />
                        </SelectTrigger>
                        <SelectContent>
                          {modelOptions.map((m) => (
                            <SelectItem key={m} value={m}>
                              {m}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    ) : (
                      <input
                        value={model}
                        onChange={(e) => setModel(e.target.value)}
                        placeholder={t('agentConfig.modelPlaceholder', '默认（留空自动）')}
                        className="h-9 w-full rounded-md border border-border/60 bg-background px-2 text-sm"
                      />
                    )}
                  </Field>
                  <div className="flex items-center justify-between">
                    <div className="min-w-0">
                      <p className="text-xs font-medium">{t('agentConfig.context1m', '1M 上下文')}</p>
                      <p className="text-[10px] text-muted-foreground">
                        {t('agentConfig.context1mHint', '需中转站支持；请先选择模型')}
                      </p>
                    </div>
                    <Switch checked={context1m} onCheckedChange={setContext1m} disabled={!model} />
                  </div>
                </div>
              )}

              {method === 'gateway' && (
                <div className="space-y-3 rounded-lg border border-border/60 bg-muted/10 p-3">
                  <div className="space-y-1.5">
                    <label className="text-xs text-muted-foreground">
                      {t('agentConfig.upstreamKind', '中转站上游协议')}
                    </label>
                    <div className="grid grid-cols-1 gap-1.5">
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
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="min-w-0">
                      <p className="text-xs font-medium">
                        {t('agentConfig.impersonateLabel', '伪装为 {{name}}', { name: agent.name })}
                      </p>
                      <p className="text-[10px] text-muted-foreground">
                        {t('agentConfig.impersonateDesc', '让中转站把任意请求识别为标准客户端')}
                      </p>
                    </div>
                    <Switch checked={impersonate} onCheckedChange={setImpersonate} />
                  </div>
                </div>
              )}
            </section>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={busy}>
            {t('common.cancel', '取消')}
          </Button>
          <Button onClick={confirm} disabled={busy || !selected}>
            {t('agentConfig.confirmAccess', '确认接入')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Field({
  label,
  children,
  hidden,
}: {
  label: string;
  children: React.ReactNode;
  hidden?: boolean;
}) {
  if (hidden) return null;
  return (
    <div className="space-y-1.5">
      <label className="text-xs text-muted-foreground">{label}</label>
      {children}
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
      onClick={onClick}
      className={cn(
        'w-full flex items-start gap-3 rounded-lg border p-3 text-left transition-colors',
        active ? 'border-primary/50 bg-primary/5' : 'border-border/60 hover:bg-muted/40'
      )}
    >
      <span
        className={cn(
          'mt-0.5 flex h-4 w-4 items-center justify-center rounded-full border shrink-0',
          active ? 'border-primary' : 'border-muted-foreground/40'
        )}
      >
        {active && <span className="h-2 w-2 rounded-full bg-primary" />}
      </span>
      <span className="flex-1 min-w-0">
        <span className={cn('flex items-center gap-1.5', active ? 'text-primary' : 'text-foreground')}>
          {icon}
          <span className="text-sm font-medium">{title}</span>
        </span>
        <span className="block text-[11px] text-muted-foreground mt-0.5">{hint}</span>
      </span>
    </button>
  );
}

function KindRadio({ active, label, onClick }: { active: boolean; label: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'flex items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs transition-colors',
        active ? 'bg-primary/10 ring-1 ring-primary/40' : 'hover:bg-muted/50'
      )}
    >
      <span
        className={cn(
          'h-3 w-3 rounded-full border shrink-0',
          active ? 'border-primary bg-primary' : 'border-muted-foreground/40'
        )}
      />
      {label}
    </button>
  );
}

/** Direct write into the tool config, for either key type. Account tokens use
 *  the account-scoped configure command (needs the chosen node base_url);
 *  independent keys use their own command. */
async function directConfigure(
  agent: string,
  o: KeyOption,
  baseUrl: string,
  model: string | null,
  context1m: boolean
): Promise<void> {
  if (o.source.type === 'independent_key') {
    const command =
      agent === 'claude_code' ? 'configure_independent_key_claude' : 'configure_independent_key_codex';
    await invoke<string>(command, { keyId: o.source.key_id, model, context1m });
    return;
  }
  // Account token: configure_{codex,claude}_global takes the token + base_url.
  if (agent === 'claude_code') {
    await invoke<string>('configure_claude_global', {
      tokenId: o.source.token_id,
      accountId: o.source.account_id,
      baseUrl,
      model,
      context1m,
    });
  } else {
    await invoke<string>('configure_codex_global', {
      tokenId: o.source.token_id,
      accountId: o.source.account_id,
      providerId: o.source.provider_id,
      baseUrl,
      model,
      context1m,
    });
  }
}
