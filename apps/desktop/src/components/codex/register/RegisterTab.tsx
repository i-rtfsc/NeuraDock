import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { History, Play, Settings2, StopCircle, Trash2, X } from 'lucide-react';
import { useCodexRegister } from '@/hooks/codex/useCodexRegister';
import { useCodexAccounts } from '@/hooks/codex/useCodexAccounts';
import type { RegisterConfig } from '@/lib/tauri';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { cn } from '@/lib/utils';

type StatusTone = 'muted' | 'info' | 'success' | 'warning';

const PANEL_CARD_CLASS =
  'border-border/40 bg-card/95 shadow-sm hover:scale-100 hover:-translate-y-0 before:hidden';

export function RegisterTab() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const locale = i18n.language === 'en-US' ? 'en-US' : 'zh-CN';
  const [mode, setMode] = useState<'single' | 'batch'>('single');
  const [count, setCount] = useState(5);
  const [concurrency, setConcurrency] = useState(2);
  const [minInterval, setMinInterval] = useState(5);
  const [maxInterval, setMaxInterval] = useState(15);
  const [isConsoleOpen, setIsConsoleOpen] = useState(false);

  const { isRunning, isCancelling, logs, progress, currentTask, startRegistration, cancelRegistration, clearLogs } =
    useCodexRegister();
  const { data: accounts } = useCodexAccounts();

  const consoleEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    consoleEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  const handleStart = () => {
    if (isRunning) return;

    const config: RegisterConfig = {
      mode,
      count: mode === 'batch' ? count : 1,
      concurrency: mode === 'batch' ? concurrency : 1,
      minIntervalSecs: minInterval,
      maxIntervalSecs: maxInterval,
    };

    setIsConsoleOpen(true);
    startRegistration(config);
  };

  const handleCloseConsole = () => {
    if (isRunning) return;
    clearLogs();
    setIsConsoleOpen(false);
  };

  const recentAccounts = [...(accounts ?? [])]
    .filter((account) => account.source === 'register')
    .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
    .slice(0, 30);

  const handleCopyLogs = async () => {
    try {
      const text = logs.map((log) => `[${log.timestamp}] ${log.message}`).join('\n');
      await navigator.clipboard.writeText(text);
      toast.success(t('codex.register.logsCopied'));
    } catch {
      toast.error(t('codex.common.copyFailed'));
    }
  };

  const runtimeLabel = isCancelling
    ? t('codex.register.runtimeStopping')
    : isRunning
      ? t('codex.register.runtimeRunning')
      : t('codex.register.runtimeIdle');
  const runtimeTone: StatusTone = isCancelling ? 'warning' : isRunning ? 'success' : 'muted';
  const progressValue = progress?.total ? Math.min(100, (progress.current / progress.total) * 100) : 0;
  const progressPercent = Math.round(progressValue);

  return (
    <div className="grid h-full min-h-0 gap-4 overflow-visible pt-2 lg:grid-cols-[296px_minmax(0,1fr)]">
      <div className="flex min-h-0 flex-col">
        <Card className={cn(PANEL_CARD_CLASS, 'flex flex-col gap-4 overflow-visible p-4')}>
          <div className="flex items-center justify-between gap-3">
            <div className="min-w-0">
              <h3 className="text-sm font-semibold text-foreground">{t('codex.register.configureTitle')}</h3>
            </div>
            <StatusPill tone={runtimeTone}>{runtimeLabel}</StatusPill>
          </div>

          <div>
            <Label className="mb-1 block text-xs text-muted-foreground">{t('codex.register.mode')}</Label>
            <Select value={mode} onValueChange={(value) => setMode(value as 'single' | 'batch')}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="single">{t('codex.register.modeSingle')}</SelectItem>
                <SelectItem value="batch">{t('codex.register.modeBatch')}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {mode === 'batch' && (
            <>
              <div>
                <Label className="mb-1 block text-xs text-muted-foreground">{t('codex.register.count')}</Label>
                <Input
                  type="number"
                  min={1}
                  max={100}
                  value={count}
                  onChange={(e) => setCount(Number(e.target.value))}
                />
              </div>

              <div>
                <Label className="mb-1 block text-xs text-muted-foreground">{t('codex.register.concurrency')}</Label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={concurrency}
                  onChange={(e) => setConcurrency(Number(e.target.value))}
                />
              </div>

              <div className="grid grid-cols-2 gap-2">
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">{t('codex.register.minInterval')}</Label>
                  <Input
                    type="number"
                    min={1}
                    value={minInterval}
                    onChange={(e) => setMinInterval(Number(e.target.value))}
                  />
                </div>
                <div>
                  <Label className="mb-1 block text-xs text-muted-foreground">{t('codex.register.maxInterval')}</Label>
                  <Input
                    type="number"
                    min={1}
                    value={maxInterval}
                    onChange={(e) => setMaxInterval(Number(e.target.value))}
                  />
                </div>
              </div>
            </>
          )}

          <div className="space-y-3 rounded-lg border border-border/40 bg-muted/10 px-3 py-3">
            <CompactStat
              icon={<History className="h-3.5 w-3.5" />}
              label={t('codex.register.recentCount')}
              value={t('codex.register.recentCountValue', { count: recentAccounts.length })}
            />

            <Button
              variant="ghost"
              size="sm"
              className="h-8 w-full justify-start px-2 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => navigate('/settings')}
            >
              <Settings2 className="h-3.5 w-3.5" />
              {t('codex.register.openProxySettings')}
            </Button>
          </div>

          <div className="flex gap-2 pt-1">
            {isRunning ? (
              <Button variant="destructive" className="flex-1" onClick={cancelRegistration} disabled={isCancelling}>
                <StopCircle className="h-4 w-4" />
                {isCancelling ? t('codex.register.cancelling') : t('codex.register.cancel')}
              </Button>
            ) : isConsoleOpen ? (
              <Button variant="outline" className="flex-1" onClick={handleCloseConsole}>
                <X className="h-4 w-4" />
                {t('codex.common.close')}
              </Button>
            ) : (
              <Button className="flex-1" onClick={handleStart}>
                <Play className="h-4 w-4" />
                {t('codex.register.start')}
              </Button>
            )}
          </div>
        </Card>
      </div>

      <div className="flex min-h-0 flex-col">
        <Card className={cn(PANEL_CARD_CLASS, 'flex min-h-0 flex-1 flex-col overflow-hidden')}>
          {isConsoleOpen ? (
            <>
              <div className="flex items-center justify-between gap-3 border-b border-border/30 bg-muted/10 px-3 py-2">
                <div className="flex min-w-0 items-center gap-2">
                  <span className="truncate font-mono text-xs text-muted-foreground">{t('codex.register.consoleTitle')}</span>
                  <StatusPill tone={runtimeTone}>{runtimeLabel}</StatusPill>
                </div>

                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-foreground"
                    title={t('codex.register.copyLogs')}
                    onClick={handleCopyLogs}
                  >
                    <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <rect x="9" y="9" width="13" height="13" rx="2" />
                      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                    </svg>
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-foreground"
                    title={t('codex.register.clearLogs')}
                    onClick={clearLogs}
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-foreground"
                    title={t('codex.common.close')}
                    aria-label={t('codex.common.close')}
                    onClick={handleCloseConsole}
                    disabled={isRunning}
                  >
                    <X className="h-3 w-3" />
                  </Button>
                </div>
              </div>

              {progress && (
                <div className="space-y-2 border-b border-border/20 bg-muted/5 px-3 py-2">
                  <div className="flex flex-wrap items-center justify-between gap-3 text-[11px] text-muted-foreground">
                    <span>{t('codex.register.progress', { current: progress.current, total: progress.total })}</span>
                    <div className="flex flex-wrap items-center gap-3">
                      <span>{t('codex.register.summary', { success: progress.successCount, failed: progress.failCount })}</span>
                      <span className="font-medium text-foreground">
                        {t('codex.register.progressPercent', { percent: progressPercent })}
                      </span>
                    </div>
                  </div>
                  <Progress value={progressValue} className="h-1.5" />
                </div>
              )}

              {currentTask && (
                <div className="grid gap-3 border-b border-border/20 bg-muted/10 px-3 py-3 md:grid-cols-2 xl:grid-cols-4">
                  <TaskInfoCard label={t('codex.register.currentTask.taskId')} value={currentTask.taskId} truncate />
                  <TaskInfoCard
                    label={t('codex.register.currentTask.email')}
                    value={currentTask.email ?? t('codex.register.fetching')}
                    truncate
                    dim={!currentTask.email}
                  />
                  <TaskInfoCard label={t('codex.register.currentTask.emailProvider')} value="Tempmail.lol" />
                  <TaskInfoCard label={t('codex.register.currentTask.status')}>
                    <TaskStatusBadge status={currentTask.status} />
                  </TaskInfoCard>
                </div>
              )}

              <div className="flex-1 overflow-y-auto bg-background/40 p-3 font-mono text-xs">
                {logs.length === 0 ? (
                  <span className="text-muted-foreground">{t('codex.register.waiting')}</span>
                ) : (
                  <div className="space-y-0.5">
                    {logs.map((log) => (
                      <div key={log.id} className={cn('leading-relaxed', logColor(log.status))}>
                        <span className="mr-2 text-muted-foreground">{log.timestamp}</span>
                        {log.message}
                      </div>
                    ))}
                  </div>
                )}
                <div ref={consoleEndRef} />
              </div>
            </>
          ) : (
            <>
              <div className="flex items-center justify-between gap-3 border-b border-border/30 bg-muted/10 px-3 py-2">
                <span className="text-xs text-muted-foreground">{t('codex.register.recentTitle')}</span>
                <StatusPill tone={recentAccounts.length > 0 ? 'info' : 'muted'}>
                  {t('codex.register.recentCountValue', { count: recentAccounts.length })}
                </StatusPill>
              </div>

              {recentAccounts.length > 0 ? (
                <div className="flex-1 overflow-x-auto bg-background">
                  <table className="w-full text-xs">
                    <thead>
                      <tr className="border-b border-border/20 text-muted-foreground">
                        <th className="px-3 py-2 text-left font-medium">{t('codex.register.recentTable.email')}</th>
                        <th className="px-3 py-2 text-left font-medium">{t('codex.register.recentTable.status')}</th>
                        <th className="px-3 py-2 text-left font-medium">{t('codex.register.recentTable.createdAt')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {recentAccounts.map((acct) => (
                        <tr key={acct.id} className="border-b border-border/10 hover:bg-muted/20">
                          <td className="px-3 py-2 font-mono">{acct.email}</td>
                          <td className="px-3 py-2">
                            <StatusBadge status={acct.status} />
                          </td>
                          <td className="px-3 py-2 text-muted-foreground">
                            {new Date(acct.createdAt).toLocaleString(locale, { hour12: false })}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div className="flex flex-1 items-center justify-center bg-background text-sm text-muted-foreground">
                  {t('codex.register.noRecent')}
                </div>
              )}
            </>
          )}
        </Card>
      </div>
    </div>
  );
}

function CompactStat({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-md border border-border/40 bg-background/70 px-2.5 py-2">
      <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
        {icon}
        <span className="truncate">{label}</span>
      </div>
      <div className="mt-1 truncate text-xs font-medium text-foreground">{value}</div>
    </div>
  );
}

function StatusPill({ tone, children }: { tone: StatusTone; children: ReactNode }) {
  const toneStyles: Record<StatusTone, string> = {
    muted: 'border-border/40 bg-background/70 text-muted-foreground',
    info: 'border-primary/20 bg-primary/5 text-primary',
    success: 'border-success-border bg-success-soft text-success-soft-foreground',
    warning: 'border-warning-border bg-warning-soft text-warning-soft-foreground',
  };

  return (
    <span className={cn('shrink-0 whitespace-nowrap rounded-full border px-2 py-0.5 text-[10px] font-medium', toneStyles[tone])}>
      {children}
    </span>
  );
}

function TaskInfoCard({
  label,
  value,
  truncate,
  dim,
  children,
}: {
  label: string;
  value?: string;
  truncate?: boolean;
  dim?: boolean;
  children?: ReactNode;
}) {
  return (
    <div className="rounded-lg border border-border/30 bg-background/70 px-3 py-2.5">
      <div className="mb-0.5 text-[10px] text-muted-foreground">{label}</div>
      {children ?? (
        <div
          className={cn(
            'text-[11px]',
            dim ? 'italic text-muted-foreground' : 'text-foreground',
            truncate && 'truncate'
          )}
          title={truncate ? value : undefined}
        >
          {value}
        </div>
      )}
    </div>
  );
}

function TaskStatusBadge({ status }: { status: string }) {
  const { t } = useTranslation();
  const map: Record<string, { cls: string; label: string }> = {
    running: { cls: 'border border-info-border bg-info-soft text-info-soft-foreground', label: t('codex.register.status.running') },
    success: { cls: 'border border-success-border bg-success-soft text-success-soft-foreground', label: t('codex.register.status.success') },
    failed: { cls: 'border border-danger-border bg-danger-soft text-danger-soft-foreground', label: t('codex.register.status.failed') },
    cancelled: { cls: 'border border-warning-border bg-warning-soft text-warning-soft-foreground', label: t('codex.register.status.cancelled') },
    pending: { cls: 'bg-muted text-muted-foreground', label: t('codex.register.status.pending') },
  };
  const { cls, label } = map[status] ?? { cls: 'bg-muted text-muted-foreground', label: status };
  return <span className={cn('inline-flex rounded px-1.5 py-0.5 text-[10px] font-medium', cls)}>{label}</span>;
}

function StatusBadge({ status }: { status: string }) {
  const { t } = useTranslation();
  const map: Record<string, { cls: string; label: string }> = {
    active: { cls: 'bg-green-500/20 text-green-500', label: t('codex.accounts.status.active') },
    expired: { cls: 'bg-yellow-500/20 text-yellow-500', label: t('codex.accounts.status.expired') },
    banned: { cls: 'bg-red-500/20 text-red-500', label: t('codex.accounts.status.banned') },
  };
  const { cls, label } = map[status] ?? { cls: 'bg-muted text-muted-foreground', label: status };
  return <span className={cn('rounded px-1.5 py-0.5 text-[10px] font-medium', cls)}>{label}</span>;
}

function logColor(status?: string) {
  switch (status) {
    case 'success':
      return 'text-success';
    case 'failed':
      return 'text-danger';
    case 'cancelled':
      return 'text-warning';
    default:
      return 'text-foreground';
  }
}
