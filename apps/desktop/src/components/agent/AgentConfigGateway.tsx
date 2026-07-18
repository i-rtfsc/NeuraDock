import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import {
  Copy,
  Check,
  Power,
  Plus,
  Pencil,
  Unplug,
} from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { AgentAccessDrawer, type KeyOption } from './AgentAccessDrawer';
import type { IndependentKeyDto } from '@/types/independentKey';
import type { AccountDto, TokenDto } from '@/types/token';
import { formatQuotaUSD } from '@/lib/formatters';
import { cn } from '@/lib/utils';

/** Agents the gateway can serve. Adding one is a single entry here. */
export type AgentId = 'codex' | 'claude_code';

export interface AgentDef {
  id: AgentId;
  name: string;
  nativeKind: 'openai_responses' | 'anthropic';
  configPath: string;
}

export const AGENTS: AgentDef[] = [
  { id: 'codex', name: 'Codex', nativeKind: 'openai_responses', configPath: '~/.codex/config.toml' },
  {
    id: 'claude_code',
    name: 'Claude Code',
    nativeKind: 'anthropic',
    configPath: '~/.claude/settings.json',
  },
];

type AccessMethod = 'direct' | 'gateway';

export type AgentAccessSource =
  | {
      type: 'account_token';
      account_id: string;
      provider_id: string;
      token_id: number;
      base_url?: string | null;
    }
  | { type: 'independent_key'; key_id: number };

/** One agent's current access record (backend source of truth). */
export interface AgentAccess {
  agent: AgentId;
  method: AccessMethod;
  label: string;
  model: string | null;
  upstream_kind: string | null;
  impersonate: boolean | null;
  source?: AgentAccessSource | null;
  context1m?: boolean | null;
}

export interface GatewayUpstreamStatus {
  id: string;
  label: string;
  circuit_state: 'closed' | 'open' | 'half_open';
  consecutive_failures: number;
  total_requests: number;
  successful_requests: number;
  failed_requests: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_tokens: number;
  average_latency_ms: number;
}

export interface GatewayStatus {
  running: boolean;
  base_url: string | null;
  bindings: {
    client: AgentId;
    model?: string | null;
    context1m?: boolean;
    upstream: GatewayUpstreamStatus | null;
  }[];
}

export type AgentConfigurationKind = 'managed' | 'manual' | 'changed' | 'unconfigured';

export interface AgentConfigurationState {
  agent: AgentId;
  state: AgentConfigurationKind;
  endpoint: string | null;
  model: string | null;
  context1m: boolean;
  credential_present: boolean;
  config_paths: string[];
}

export const GATEWAY_STATUS_QUERY_KEY = ['gateway-status'];
export const AGENT_ACCESS_QUERY_KEY = ['agent-access'];
export const AGENT_CONFIGURATION_QUERY_KEY = ['agent-configuration-states'];

export function useGatewayStatus() {
  return useQuery<GatewayStatus>({
    queryKey: GATEWAY_STATUS_QUERY_KEY,
    queryFn: () => invoke<GatewayStatus>('get_gateway_status'),
    refetchInterval: 5000,
  });
}

export function useAgentAccess() {
  return useQuery<AgentAccess[]>({
    queryKey: AGENT_ACCESS_QUERY_KEY,
    queryFn: () => invoke<AgentAccess[]>('get_agent_access'),
  });
}

export function useAgentConfigurationStates() {
  return useQuery<AgentConfigurationState[]>({
    queryKey: AGENT_CONFIGURATION_QUERY_KEY,
    queryFn: () => invoke<AgentConfigurationState[]>('get_agent_configuration_states'),
    refetchInterval: 5000,
  });
}

/** Every selectable key (relay account tokens + independent keys). */
function useKeyOptions() {
  const independent = useQuery<IndependentKeyDto[]>({
    queryKey: ['independent-keys'],
    queryFn: () => invoke<IndependentKeyDto[]>('get_all_independent_keys'),
  });
  const accounts = useQuery<AccountDto[]>({
    queryKey: ['accounts', 'gateway-picker'],
    queryFn: () => invoke<AccountDto[]>('get_all_accounts', { enabledOnly: false }),
  });
  const tokens = useQuery<KeyOption[]>({
    queryKey: [
      'gateway-account-tokens',
      'cached-session-independent',
      accounts.data?.map((account) => account.id),
    ],
    enabled: !!accounts.data && accounts.data.length > 0,
    queryFn: async () => {
      const enabledAccounts = (accounts.data ?? []).filter((account) => account.enabled);
      const groups = await Promise.all(
        enabledAccounts.map(async (account): Promise<KeyOption[]> => {
          try {
            const list = await invoke<TokenDto[]>('fetch_account_tokens', {
              accountId: account.id,
              forceRefresh: false,
            });
            const options: KeyOption[] = [];
            for (const tk of list) {
              // Disabled tokens can't be configured; leave them out of the picker.
              if (!tk.is_active) continue;
              options.push({
                label: tk.name,
                masked: tk.masked_key,
                providerName: account.provider_name,
                providerId: account.provider_id,
                baseUrl: null,
                accountId: account.id,
                accountName: account.name,
                sourceGroup: 'relay',
                balance: tk.unlimited_quota ? '∞' : formatQuotaUSD(tk.remain_quota),
                modelLimitsEnabled: tk.model_limits_enabled,
                allowedModels: tk.model_limits_allowed,
                deniedModels: tk.model_limits_denied,
                source: {
                  type: 'account_token',
                  account_id: account.id,
                  provider_id: account.provider_id,
                  token_id: tk.id,
                },
              });
            }
            return options;
          } catch {
            return [];
          }
        })
      );
      return groups.flat();
    },
  });

  const options: KeyOption[] = useMemo(() => {
    const indep: KeyOption[] = (independent.data ?? [])
      .filter((k) => k.is_active)
      .map((k) => ({
        label: k.name,
        masked: k.masked_key,
        providerName: k.provider_type_display,
        providerId: null,
        baseUrl: k.base_url,
        accountId: null,
        accountName: null,
        sourceGroup: 'independent',
        balance: null,
        modelLimitsEnabled: false,
        allowedModels: [],
        deniedModels: [],
        source: { type: 'independent_key', key_id: k.id },
      }));
    return [...(tokens.data ?? []), ...indep];
  }, [independent.data, tokens.data]);

  return {
    options,
    isLoading: independent.isLoading || accounts.isLoading || tokens.isLoading,
  };
}

function GatewayControl({ status, disabled }: { status?: GatewayStatus; disabled: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const running = status?.running ?? false;

  const toggle = async () => {
    setBusy(true);
    try {
      if (running) {
        await invoke('stop_gateway_server');
        toast.success(t('agentConfig.gatewayStopped', '网关已停止，工具配置已还原'));
      } else {
        await invoke('start_gateway_server');
        toast.success(t('agentConfig.gatewayStarted', '网关已启动'));
      }
      queryClient.invalidateQueries({ queryKey: GATEWAY_STATUS_QUERY_KEY });
      queryClient.invalidateQueries({ queryKey: AGENT_CONFIGURATION_QUERY_KEY });
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(false);
    }
  };

  const copy = async () => {
    if (!status?.base_url) return;
    await navigator.clipboard.writeText(status.base_url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="flex min-w-0 items-center gap-1.5">
      <Button
        onClick={toggle}
        disabled={busy || (!running && disabled)}
        variant={running ? 'outline' : 'default'}
        size="sm"
      >
        <Power className="h-3.5 w-3.5" />
        {running ? t('agentConfig.stop', '停止') : t('agentConfig.start', '启动')}
      </Button>
      {running && status?.base_url && (
        <Button
          variant="ghost"
          size="sm"
          onClick={copy}
          className="min-w-0 max-w-52 px-2 font-normal text-muted-foreground hover:text-foreground"
        >
          <code className="truncate font-mono text-xs">{status.base_url}</code>
          {copied ? <Check className="h-3 w-3 shrink-0" /> : <Copy className="h-3 w-3 shrink-0" />}
        </Button>
      )}
    </div>
  );
}

function kindLabel(kind: string | null, t: ReturnType<typeof useTranslation>['t']): string {
  switch (kind) {
    case 'openai_responses':
      return t('agentConfig.kindResponses', 'OpenAI Responses（原生透传）');
    case 'anthropic':
      return t('agentConfig.kindAnthropic', 'Anthropic（原生透传）');
    case 'openai_chat':
      return t('agentConfig.kindChat', 'OpenAI Chat（网关转换）');
    default:
      return '—';
  }
}

function AccessStatusCard({
  agent,
  access,
  configuration,
  sourceOption,
  upstream,
  running,
  onEdit,
  onRemove,
  onStartGateway,
}: {
  agent: AgentDef;
  access?: AgentAccess;
  configuration?: AgentConfigurationState;
  sourceOption?: KeyOption;
  upstream?: GatewayUpstreamStatus | null;
  running: boolean;
  onEdit: () => void;
  onRemove: () => void;
  onStartGateway: () => void;
}) {
  const { t } = useTranslation();

  if (!access && configuration?.state === 'manual') {
    return (
      <div className="space-y-4">
        <section className="card-vivid cursor-default space-y-3 rounded-xl p-4">
          <div className="flex items-center justify-between gap-3">
            <h3 className="text-sm font-semibold">{t('agentConfig.sourceSectionTitle')}</h3>
            <Button variant="outline" size="sm" onClick={onEdit}>
              <Pencil className="h-3.5 w-3.5" />
              {t('agentConfig.manageManualConfig')}
            </Button>
          </div>
          <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_minmax(280px,0.9fr)]">
            <div className="rounded-lg bg-muted/25 p-2.5">
              <div className="flex min-w-0 items-start gap-3">
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-semibold">
                    {configuration.endpoint ?? t('agentConfig.manualConfig')}
                  </p>
                  <p className="mt-0.5 truncate text-xs text-muted-foreground">
                    {configuration.model ?? t('agentConfig.modelNotDetected')}
                  </p>
                </div>
                <Badge variant="soft-warning" className="shrink-0 text-[10px]">
                  {t('agentConfig.manualConfig')}
                </Badge>
              </div>
            </div>
            <div className="rounded-lg bg-muted/20 p-2.5">
              <dl className="grid grid-cols-2 gap-1.5 text-xs">
                <ReliabilityMetric
                  label={t('agentConfig.detectedModel')}
                  value={configuration.model ?? '—'}
                />
                <ReliabilityMetric
                  label={t('agentConfig.detectedCredential')}
                  value={
                    configuration.credential_present
                      ? t('agentConfig.credentialPresent')
                      : t('agentConfig.credentialMissing')
                  }
                />
                <div className="col-span-2 rounded-md bg-background/55 px-2 py-1.5">
                  <dt className="text-muted-foreground">{t('agentConfig.configFiles')}</dt>
                  <dd className="mt-0.5 truncate font-mono text-[11px]">
                    {configuration.config_paths.join(' / ') || '—'}
                  </dd>
                </div>
              </dl>
            </div>
          </div>
        </section>
      </div>
    );
  }

  if (!access) {
    return (
      <div className="flex min-h-80 flex-col items-center justify-center px-6 py-12 text-center">
        <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-muted text-muted-foreground">
          <Plus className="h-5 w-5" />
        </div>
        <h3 className="mt-4 text-base font-semibold">
          {t('agentConfig.notBoundTitle', '{{name}} 尚未接入任何 Key', { name: agent.name })}
        </h3>
        <p className="mt-2 max-w-md text-sm leading-6 text-muted-foreground">
          {t('agentConfig.notBoundHint', '选择一个 Key，可直接写入 {{path}}，或经本地网关适配。', {
            path: agent.configPath,
          })}
        </p>
        <Button className="mt-5" onClick={onEdit}>
          <Plus className="h-4 w-4" />
          {t('agentConfig.access', '接入 Key')}
        </Button>
      </div>
    );
  }

  const viaGateway = access.method === 'gateway';
  const configurationChanged = configuration?.state === 'changed';
  const sourceTitle = sourceOption
    ? sourceOption.sourceGroup === 'relay'
      ? sourceOption.providerName
      : t('agentConfig.groupIndependent', '独立 API Key')
    : access.label;
  const sourceDetail = sourceOption
    ? sourceOption.sourceGroup === 'relay'
      ? [sourceOption.accountName, sourceOption.label].filter(Boolean).join(' / ')
      : sourceOption.label
    : '';
  const hasRequestData = !!upstream && upstream.total_requests > 0;
  const statusVariant = !viaGateway || !upstream
    ? 'outline'
    : upstream.circuit_state === 'open'
      ? 'soft-danger'
      : upstream.circuit_state === 'half_open'
        ? 'soft-warning'
        : !running || !hasRequestData
          ? 'secondary'
          : 'soft-success';
  const statusLabel = !viaGateway
    ? t('agentConfig.directBadge', '直连')
    : upstream?.circuit_state === 'open'
      ? t('agentConfig.circuitOpen')
      : upstream?.circuit_state === 'half_open'
        ? t('agentConfig.circuitHalfOpen')
        : !running
          ? t('agentConfig.stopped', '已停止')
          : !hasRequestData
            ? t('agentConfig.circuitIdle')
            : t('agentConfig.circuitClosed');

  return (
    <div className="space-y-4">
      <section className="card-vivid cursor-default space-y-3 rounded-xl p-4">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-sm font-semibold">{t('agentConfig.sourceSectionTitle')}</h3>
          <div className="flex items-center gap-1.5">
            <Button variant="outline" size="sm" onClick={onEdit}>
              <Pencil className="h-3.5 w-3.5" />
              {t('agentConfig.edit', '编辑')}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="text-destructive hover:bg-destructive/10 hover:text-destructive"
              onClick={onRemove}
            >
              <Unplug className="h-3.5 w-3.5" />
              {t('agentConfig.remove', '解除')}
            </Button>
          </div>
        </div>
        <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_minmax(280px,0.9fr)]">
          <div className="rounded-lg bg-muted/25 p-2.5">
            <div className="flex min-w-0 items-start gap-3">
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-semibold">{sourceTitle}</p>
                {sourceDetail && (
                  <p className="mt-0.5 truncate text-xs text-muted-foreground">{sourceDetail}</p>
                )}
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <Badge
                  variant={configurationChanged ? 'soft-warning' : 'soft-primary'}
                  className="text-[10px]"
                >
                  {configurationChanged
                    ? t('agentConfig.configurationChanged')
                    : t('agentConfig.managedConfig')}
                </Badge>
                <Badge variant={statusVariant} className="text-[10px]">
                  {statusLabel}
                </Badge>
              </div>
            </div>
            {configurationChanged && (
              <div className="mt-2 rounded-md bg-warning/10 px-2 py-1.5 text-[11px] text-warning">
                {t('agentConfig.configurationChangedHint')}
                {(configuration.endpoint || configuration.model) && (
                  <span className="ml-1 font-mono">
                    {[configuration.endpoint, configuration.model].filter(Boolean).join(' · ')}
                  </span>
                )}
              </div>
            )}
            {viaGateway && (
              <div className="mt-2 flex items-center gap-2 text-[11px] text-muted-foreground">
                <span
                  className={cn(
                    'h-2 w-2 rounded-full',
                    running ? 'bg-success' : 'bg-muted-foreground/30'
                  )}
                />
                <span>
                  {running
                    ? t('agentConfig.gatewayRunningShort')
                    : t('agentConfig.gatewayStoppedShort')}
                </span>
                {!running && (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="ml-auto h-7 px-2"
                    onClick={onStartGateway}
                  >
                    {t('agentConfig.startNow', '立即启动')}
                  </Button>
                )}
              </div>
            )}
          </div>

          <div className="rounded-lg bg-muted/20 p-2.5">
            <div className="mb-2 flex items-start justify-between gap-2">
              <div>
                <p className="text-xs font-medium">{t('agentConfig.reliabilityTitle')}</p>
                <p className="mt-0.5 text-[10px] text-muted-foreground">
                  {viaGateway
                    ? t('agentConfig.reliabilityHint')
                    : t('agentConfig.directStatsUnavailable')}
                </p>
              </div>
              {!hasRequestData && (
                <Badge variant="outline" className="shrink-0 px-1.5 text-[9px] font-medium">
                  {viaGateway
                    ? t('agentConfig.noRequestData')
                    : t('agentConfig.notApplicable', '不适用')}
                </Badge>
              )}
            </div>
            <dl className="grid grid-cols-2 gap-1.5 text-xs">
              <ReliabilityMetric
                label={t('agentConfig.requestCount')}
                value={viaGateway ? (upstream?.total_requests ?? 0).toLocaleString() : '—'}
              />
              <ReliabilityMetric
                label={t('agentConfig.successRate')}
                value={
                  hasRequestData && upstream
                    ? String(Math.round((upstream.successful_requests / upstream.total_requests) * 100)) + '%'
                    : '—'
                }
              />
              <ReliabilityMetric
                label={t('agentConfig.tokenCount')}
                value={
                  hasRequestData && upstream
                    ? (upstream.input_tokens + upstream.output_tokens).toLocaleString()
                    : '—'
                }
              />
              <ReliabilityMetric
                label={t('agentConfig.averageLatency')}
                value={
                  hasRequestData && upstream
                    ? String(upstream.average_latency_ms) + ' ms'
                    : '—'
                }
              />
            </dl>
          </div>
        </div>
      </section>

      <section className="card-vivid cursor-default space-y-3 rounded-xl p-4 sm:p-5">
        <h3 className="text-sm font-semibold">{t('agentConfig.runtimeSectionTitle', '运行配置')}</h3>
        <dl className="grid gap-2 text-sm">
          <DetailRow label={t('agentConfig.rowModel', '模型')}>
            <span className="flex min-w-0 items-center gap-2">
              <span className="truncate font-mono text-xs">{access.model ?? '—'}</span>
              {access.context1m && (
                <Badge variant="outline" className="h-5 shrink-0 px-1.5 text-[10px]">1M</Badge>
              )}
            </span>
          </DetailRow>
          <DetailRow label={t('agentConfig.rowMethod', '方式')}>
            {viaGateway
              ? t('agentConfig.methodGatewayShort', '经本地网关')
              : t('agentConfig.methodDirectShort', '直接写入 {{path}}', { path: agent.configPath })}
          </DetailRow>
          {viaGateway && (
            <>
              <DetailRow label={t('agentConfig.rowUpstream', '上游')}>
                {kindLabel(access.upstream_kind, t)}
              </DetailRow>
              <DetailRow label={t('agentConfig.rowCompatibility', '客户端兼容')}>
                {access.impersonate
                  ? t('agentConfig.impersonateOn', '已伪装为 {{name}}', { name: agent.name })
                  : t('agentConfig.impersonateOff', '不伪装')}
              </DetailRow>
            </>
          )}
        </dl>
      </section>
    </div>
  );
}

function ReliabilityMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md bg-background/55 px-2 py-1.5">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="mt-0.5 font-mono font-semibold tabular-nums">{value}</dd>
    </div>
  );
}

function DetailRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid min-h-11 grid-cols-[88px_minmax(0,1fr)] items-center gap-4 rounded-lg bg-muted/20 px-3 py-2.5">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0">{children}</dd>
    </div>
  );
}

const AGENT_HEADER_TAB_CLASS =
  'h-8 shrink-0 rounded-md px-4 text-sm font-medium transition-all duration-base ease-smooth data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm';

export function AgentConfigHeaderControls({
  selectedAgent,
  onSelectAgent,
}: {
  selectedAgent: AgentId;
  onSelectAgent: (agent: AgentId) => void;
}) {
  return (
    <Tabs
      value={selectedAgent}
      onValueChange={(value) => onSelectAgent(value as AgentId)}
      className="min-w-0"
    >
      <TabsList className="inline-flex h-10 max-w-[min(38vw,480px)] shrink-0 items-center justify-start overflow-x-auto whitespace-nowrap rounded-[var(--radius-control)] border border-border/50 bg-muted/50 p-1">
        {AGENTS.map((agent) => (
          <TabsTrigger key={agent.id} value={agent.id} className={AGENT_HEADER_TAB_CLASS}>
            {agent.name}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}

export function AgentGatewayHeaderControl() {
  const { data: status } = useGatewayStatus();
  const { data: accessList } = useAgentAccess();
  const anyGatewayAccess = (accessList ?? []).some((access) => access.method === 'gateway');

  return <GatewayControl status={status} disabled={!anyGatewayAccess} />;
}

export function AgentConfigGateway({
  selectedAgent,
}: {
  selectedAgent: AgentId;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data: status } = useGatewayStatus();
  const { data: accessList } = useAgentAccess();
  const { data: configurationStates } = useAgentConfigurationStates();
  const { options: keyOptions, isLoading: keyOptionsLoading } = useKeyOptions();

  const [drawerOpen, setDrawerOpen] = useState(false);

  const agent = AGENTS.find((a) => a.id === selectedAgent)!;
  const accessByAgent = useMemo(() => {
    const map = new Map<AgentId, AgentAccess>();
    accessList?.forEach((a) => map.set(a.agent, a));
    return map;
  }, [accessList]);
  const access = accessByAgent.get(selectedAgent);
  const configuration = configurationStates?.find((item) => item.agent === selectedAgent);
  const gatewayBinding = status?.bindings.find((binding) => binding.client === selectedAgent);
  const sourceOption = useMemo(() => {
    if (!access?.source) return undefined;
    return keyOptions.find((option) => {
      if (access.source?.type !== option.source.type) return false;
      if (access.source.type === 'account_token' && option.source.type === 'account_token') {
        return (
          access.source.account_id === option.source.account_id &&
          access.source.token_id === option.source.token_id
        );
      }
      return (
        access.source.type === 'independent_key' &&
        option.source.type === 'independent_key' &&
        access.source.key_id === option.source.key_id
      );
    });
  }, [access, keyOptions]);

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: GATEWAY_STATUS_QUERY_KEY });
    queryClient.invalidateQueries({ queryKey: AGENT_ACCESS_QUERY_KEY });
    queryClient.invalidateQueries({ queryKey: AGENT_CONFIGURATION_QUERY_KEY });
  };

  const remove = async () => {
    try {
      if (access?.method === 'gateway') {
        await invoke('clear_gateway_binding', { client: selectedAgent });
      } else {
        const command = selectedAgent === 'claude_code' ? 'clear_claude_global' : 'clear_codex_global';
        await invoke<string>(command);
        await invoke('clear_agent_access', { agent: selectedAgent });
      }
      toast.success(t('agentConfig.unbound', '已解除接入并还原配置'));
      refresh();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const startGateway = async () => {
    try {
      await invoke('start_gateway_server');
      toast.success(t('agentConfig.gatewayStarted', '网关已启动'));
      refresh();
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <div className="space-y-4">
      <AccessStatusCard
        agent={agent}
        access={access}
        configuration={configuration}
        sourceOption={sourceOption}
        upstream={gatewayBinding?.upstream}
        running={status?.running ?? false}
        onEdit={() => setDrawerOpen(true)}
        onRemove={remove}
        onStartGateway={startGateway}
      />

      <AgentAccessDrawer
        open={drawerOpen}
        onOpenChange={setDrawerOpen}
        agent={agent}
        keyOptions={keyOptions}
        loading={keyOptionsLoading}
        existing={access}
        onDone={refresh}
      />
    </div>
  );
}
