import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
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
import { Play, StopCircle, Trash2, X } from 'lucide-react';

export function RegisterTab() {
  const { t, i18n } = useTranslation();
  const locale = i18n.language === 'en-US' ? 'en-US' : 'zh-CN';
  const [mode, setMode] = useState<'single' | 'batch'>('single');
  const [count, setCount] = useState(5);
  const [concurrency, setConcurrency] = useState(2);
  const [minInterval, setMinInterval] = useState(5);
  const [maxInterval, setMaxInterval] = useState(15);
  const [proxyUrl, setProxyUrl] = useState('');
  const [isConsoleOpen, setIsConsoleOpen] = useState(false);

  const { isRunning, isCancelling, logs, progress, currentTask, startRegistration, cancelRegistration, clearLogs } =
    useCodexRegister();
  const { data: accounts } = useCodexAccounts();

  // Auto-scroll console to bottom
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
      proxyUrl: proxyUrl.trim() || null,
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

  return (
    <div className="flex h-full min-h-0 gap-4 overflow-visible pt-2">
      {/* ── Left: Config panel ── */}
      <div className="w-72 shrink-0 flex flex-col gap-4">
        <Card className="flex flex-col gap-4 overflow-visible p-4">
          <div>
            <Label className="mb-1 block text-xs text-muted-foreground">{t('codex.register.mode')}</Label>
            <Select value={mode} onValueChange={(v) => setMode(v as 'single' | 'batch')}>
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
              <div className="flex gap-2">
                <div className="flex-1">
                  <Label className="mb-1 block text-xs text-muted-foreground">{t('codex.register.minInterval')}</Label>
                  <Input
                    type="number"
                    min={1}
                    value={minInterval}
                    onChange={(e) => setMinInterval(Number(e.target.value))}
                  />
                </div>
                <div className="flex-1">
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

          <div>
            <Label className="mb-1 block text-xs text-muted-foreground">{t('codex.register.proxy')}</Label>
            <Input
              placeholder="http://127.0.0.1:7890"
              value={proxyUrl}
              onChange={(e) => setProxyUrl(e.target.value)}
            />
          </div>

          <div className="flex gap-2 pt-1">
            {isRunning ? (
              <Button
                variant="destructive"
                className="flex-1"
                onClick={cancelRegistration}
                disabled={isCancelling}
              >
                <StopCircle className="w-4 h-4 mr-1" />
                {isCancelling ? t('codex.register.cancelling') : t('codex.register.cancel')}
              </Button>
            ) : isConsoleOpen ? (
              <Button variant="outline" className="flex-1" onClick={handleCloseConsole}>
                <X className="w-4 h-4 mr-1" />
                {t('codex.common.close')}
              </Button>
            ) : (
              <Button className="flex-1" onClick={handleStart}>
                <Play className="w-4 h-4 mr-1" />
                {t('codex.register.start')}
              </Button>
            )}
          </div>
        </Card>

      </div>

        {/* ── Right: Console while opened, otherwise recent accounts ── */}
        <div className="flex-1 min-w-0 flex flex-col gap-4">
          <Card className="flex min-h-0 flex-1 flex-col border-border/40 bg-card/95">
            {isConsoleOpen ? (
              <>
                <div className="flex items-center justify-between border-b border-border/30 bg-muted/10 px-3 py-2">
                  <span className="font-mono text-xs text-muted-foreground">{t('codex.register.consoleTitle')}</span>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 text-muted-foreground hover:text-foreground"
                      title={t('codex.register.copyLogs')}
                      onClick={handleCopyLogs}
                    >
                      <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                      </svg>
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 text-muted-foreground hover:text-foreground"
                      title={t('codex.register.clearLogs')}
                      onClick={clearLogs}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 text-muted-foreground hover:text-foreground"
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
                  <div className="border-b border-border/20 bg-muted/5 px-3 py-2">
                    <div className="mb-2 flex items-center justify-between gap-3 text-[11px] text-muted-foreground">
                      <span>{t('codex.register.progress', { current: progress.current, total: progress.total })}</span>
                      <span>{t('codex.register.summary', { success: progress.successCount, failed: progress.failCount })}</span>
                    </div>
                    <Progress
                      value={progress.total > 0 ? (progress.current / progress.total) * 100 : 0}
                      className="h-1.5"
                    />
                  </div>
                )}
                {currentTask && (
                  <div className="grid grid-cols-4 gap-0 border-b border-border/20 bg-muted/20 text-[11px] font-mono">
                    <TaskInfoCell label={t('codex.register.currentTask.taskId')} value={currentTask.taskId} truncate />
                    <TaskInfoCell
                      label={t('codex.register.currentTask.email')}
                      value={currentTask.email ?? t('codex.register.fetching')}
                      truncate
                      dim={!currentTask.email}
                    />
                    <TaskInfoCell label={t('codex.register.currentTask.emailProvider')} value="Tempmail.lol" />
                    <div className="px-3 py-2 border-l border-border/20">
                      <div className="mb-0.5 text-[10px] text-muted-foreground">{t('codex.register.currentTask.status')}</div>
                      <TaskStatusBadge status={currentTask.status} />
                    </div>
                  </div>
                )}

                <div className="flex-1 overflow-y-auto p-3 font-mono text-xs space-y-0.5">
                  {logs.length === 0 && (
                    <span className="text-muted-foreground">{t('codex.register.waiting')}</span>
                  )}
                  {logs.map((log) => (
                    <div key={log.id} className={cn('leading-relaxed', logColor(log.status))}>
                      <span className="mr-2 text-muted-foreground">{log.timestamp}</span>
                      {log.message}
                    </div>
                  ))}
                  <div ref={consoleEndRef} />
                </div>
              </>
            ) : (
              <>
                <div className="border-b border-border/30 px-3 py-2 text-xs text-muted-foreground">
                  {t('codex.register.recentTitle')}
                </div>
                {recentAccounts.length > 0 ? (
                  <div className="flex-1 overflow-x-auto bg-background">
                    <table className="w-full text-xs">
                      <thead>
                        <tr className="text-muted-foreground border-b border-border/20">
                          <th className="text-left px-3 py-2 font-medium">{t('codex.register.recentTable.email')}</th>
                          <th className="text-left px-3 py-2 font-medium">{t('codex.register.recentTable.status')}</th>
                          <th className="text-left px-3 py-2 font-medium">{t('codex.register.recentTable.createdAt')}</th>
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
                  <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground bg-background">
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

function TaskInfoCell({
  label,
  value,
  truncate,
  dim,
}: {
  label: string;
  value: string;
  truncate?: boolean;
  dim?: boolean;
}) {
  return (
    <div className="px-3 py-2 border-l border-border/20 first:border-l-0">
      <div className="mb-0.5 text-[10px] text-muted-foreground">{label}</div>
      <div
        className={cn(
          'text-[11px]',
          dim ? 'text-muted-foreground italic' : 'text-foreground',
          truncate && 'truncate max-w-[140px]'
        )}
        title={truncate ? value : undefined}
      >
        {value}
      </div>
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
  return (
    <span className={cn('px-1.5 py-0.5 rounded text-[10px] font-medium', cls)}>{label}</span>
  );
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
    case 'success': return 'text-success';
    case 'failed': return 'text-danger';
    case 'cancelled': return 'text-warning';
    default: return 'text-foreground';
  }
}
