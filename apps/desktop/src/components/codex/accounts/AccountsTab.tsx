import { useMemo, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-shell';
import { toast } from 'sonner';
import {
  ArrowRightLeft,
  Copy,
  CreditCard,
  ExternalLink,
  LogOut,
  Mail,
  MoreHorizontal,
  RefreshCw,
  ShieldCheck,
  Star,
  Trash2,
} from 'lucide-react';
import type {
  CodexAccountDto,
  CodexAuthInfoDto,
  CodexInboxCodeDto,
  CodexPaymentLinkDto,
  CodexPaymentLinkRequestDto,
  CodexPaymentPlanDto,
  CodexQuotaDto,
} from '@/lib/tauri';
import {
  useCodexAccountInboxCode,
  useCodexAccounts,
  useDeleteCodexAccount,
  useRefreshCodexAccountQuota,
} from '@/hooks/codex/useCodexAccounts';
import { useGenerateCodexPaymentLink, type CodexPaymentFormState } from '@/hooks/codex/useCodexPayment';
import { useActiveCodexAuth, useLogoutCodexAuth, useRefreshActiveCodexAuthQuota, useSwitchCodexAuth } from '@/hooks/codex/useCodexAuth';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Progress } from '@/components/ui/progress';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';
import {
  formatPlanLabel,
  formatRemainingPercent,
  formatResetAt,
  formatResetHint,
  formatWindowLabel,
  getRemainingPercent,
  pickPrimaryDisplayWindow,
} from '@/components/codex/shared/quota';
import {
  buildCodexAccountList,
  hasValidCodexAuth,
  type CodexAccountSortOption,
  type CodexAccountStatusFilter,
} from './accountList';

const PAYMENT_COUNTRIES = [
  { code: 'SG', labelKey: 'codex.payment.country.sg', currency: 'SGD' },
  { code: 'US', labelKey: 'codex.payment.country.us', currency: 'USD' },
  { code: 'TR', labelKey: 'codex.payment.country.tr', currency: 'TRY' },
  { code: 'JP', labelKey: 'codex.payment.country.jp', currency: 'JPY' },
  { code: 'HK', labelKey: 'codex.payment.country.hk', currency: 'HKD' },
  { code: 'GB', labelKey: 'codex.payment.country.gb', currency: 'GBP' },
  { code: 'EU', labelKey: 'codex.payment.country.eu', currency: 'EUR' },
  { code: 'AU', labelKey: 'codex.payment.country.au', currency: 'AUD' },
  { code: 'CA', labelKey: 'codex.payment.country.ca', currency: 'CAD' },
  { code: 'IN', labelKey: 'codex.payment.country.in', currency: 'INR' },
  { code: 'BR', labelKey: 'codex.payment.country.br', currency: 'BRL' },
  { code: 'MX', labelKey: 'codex.payment.country.mx', currency: 'MXN' },
] as const;

const DEFAULT_COUNTRY = PAYMENT_COUNTRIES[0].code;
const STATIC_CARD_CLASS = 'hover:scale-100 hover:translate-y-0 hover:shadow-sm';

function buildDefaultPaymentForm(): CodexPaymentFormState {
  return {
    planType: 'plus',
    country: DEFAULT_COUNTRY,
    currency: getCurrencyForCountry(DEFAULT_COUNTRY),
    workspaceName: 'MyTeam',
    seatQuantity: '5',
    priceInterval: 'month',
  };
}

export function AccountsTab() {
  const { t, i18n } = useTranslation();
  const locale = i18n.language === 'en-US' ? 'en-US' : 'zh-CN';
  const { data: accounts, isLoading } = useCodexAccounts();
  const { data: activeAuth, isLoading: authLoading } = useActiveCodexAuth();
  const deleteAccount = useDeleteCodexAccount();
  const refreshQuota = useRefreshCodexAccountQuota();
  const refreshActiveQuota = useRefreshActiveCodexAuthQuota();
  const switchAuth = useSwitchCodexAuth();
  const logout = useLogoutCodexAuth();
  const inboxCode = useCodexAccountInboxCode();
  const generatePaymentLink = useGenerateCodexPaymentLink();
  const [refreshingId, setRefreshingId] = useState<string | null>(null);
  const [loadingInboxId, setLoadingInboxId] = useState<string | null>(null);
  const [inboxDialogOpen, setInboxDialogOpen] = useState(false);
  const [inboxResult, setInboxResult] = useState<CodexInboxCodeDto | null>(null);
  const [paymentAccount, setPaymentAccount] = useState<CodexAccountDto | null>(null);
  const [paymentForm, setPaymentForm] = useState<CodexPaymentFormState>(buildDefaultPaymentForm());
  const [generatedPaymentLink, setGeneratedPaymentLink] = useState<CodexPaymentLinkDto | null>(null);
  const [sortOption, setSortOption] = useState<CodexAccountSortOption>('remaining-desc');
  const [hideNoQuota, setHideNoQuota] = useState(false);
  const [onlyValidAuth, setOnlyValidAuth] = useState(false);
  const [onlyUnlimited, setOnlyUnlimited] = useState(false);
  const [statusFilter, setStatusFilter] = useState<CodexAccountStatusFilter>('all');

  const accountList = accounts ?? [];
  const displayAccounts = useMemo(
    () => buildCodexAccountList(accountList, { sortOption, hideNoQuota, onlyValidAuth, onlyUnlimited, statusFilter }),
    [accountList, hideNoQuota, onlyUnlimited, onlyValidAuth, sortOption, statusFilter]
  );
  const activeAccount = accountList.find((account) => isActiveAccount(activeAuth, account)) ?? null;
  const activeQuota = activeAuth?.quota ?? buildQuotaFromAccount(activeAccount);
  const validTokenCount = accountList.filter(hasValidCodexAuth).length;
  const creditsCount = accountList.filter((account) => account.isUnlimited || account.hasCredits).length;

  const copyText = async (text: string | null | undefined, successKey: string) => {
    if (!text) {
      toast.error(t('codex.common.copyUnavailable'));
      return;
    }

    try {
      await navigator.clipboard.writeText(text);
      toast.success(t(successKey));
    } catch {
      toast.error(t('codex.common.copyFailed'));
    }
  };

  const handleRefreshQuota = async (id: string) => {
    setRefreshingId(id);
    try {
      await refreshQuota.mutateAsync(id);
    } finally {
      setRefreshingId(null);
    }
  };

  const handleFetchInboxCode = async (account: CodexAccountDto) => {
    setLoadingInboxId(account.id);
    setInboxDialogOpen(true);
    setInboxResult(null);
    try {
      const result = await inboxCode.mutateAsync(account.id);
      setInboxResult(result);
    } catch {
      setInboxDialogOpen(false);
    } finally {
      setLoadingInboxId(null);
    }
  };

  const handleOpenPaymentDialog = (account: CodexAccountDto) => {
    setPaymentAccount(account);
    setPaymentForm(buildDefaultPaymentForm());
    setGeneratedPaymentLink(null);
  };

  const handlePaymentDialogChange = (openState: boolean) => {
    if (openState) return;
    setPaymentAccount(null);
    setGeneratedPaymentLink(null);
    setPaymentForm(buildDefaultPaymentForm());
  };

  const handleGeneratePaymentLink = async () => {
    if (!paymentAccount) return;

    const request: CodexPaymentLinkRequestDto = {
      planType: paymentForm.planType,
      country: paymentForm.country,
      currency: paymentForm.currency,
      workspaceName: paymentForm.planType === 'team' ? paymentForm.workspaceName.trim() || null : null,
      seatQuantity:
        paymentForm.planType === 'team'
          ? Math.max(1, Number.parseInt(paymentForm.seatQuantity || '5', 10) || 5)
          : null,
      priceInterval: paymentForm.planType === 'team' ? paymentForm.priceInterval : null,
    };

    const result = await generatePaymentLink.mutateAsync({
      accountId: paymentAccount.id,
      request,
    });
    setGeneratedPaymentLink(result);
  };

  const updatePaymentForm = (patch: Partial<CodexPaymentFormState>) => {
    setGeneratedPaymentLink(null);
    setPaymentForm((current) => ({ ...current, ...patch }));
  };

  const handleCountryChange = (country: string) => {
    updatePaymentForm({
      country,
      currency: getCurrencyForCountry(country),
    });
  };

  if (isLoading) {
    return <div className="py-8 text-center text-sm text-muted-foreground">{t('codex.common.loading')}</div>;
  }

  return (
    <>
      <div className="flex flex-col gap-4">
        <div className="grid items-start gap-4 lg:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)] lg:items-stretch">
          <ActiveAuthCard
            activeAuth={activeAuth}
            activeAccount={activeAccount}
            activeQuota={activeQuota}
            isLoading={authLoading}
            isRefreshing={refreshActiveQuota.isPending}
            isLoggingOut={logout.isPending}
            locale={locale}
            onRefresh={() => refreshActiveQuota.mutate()}
            onLogout={() => logout.mutate()}
            onCopyAccountId={(value) => copyText(value, 'codex.accounts.accountIdCopied')}
          />

          <div className="grid gap-3 sm:grid-cols-3 lg:h-full lg:grid-cols-1">
            <StatCard label={t('codex.accounts.stats.totalAccounts')} value={accountList.length} hint={t('codex.accounts.stats.totalAccountsHint')} />
            <StatCard label={t('codex.accounts.stats.validAuth')} value={validTokenCount} hint={t('codex.accounts.stats.validAuthHint')} />
            <StatCard label={t('codex.accounts.stats.accountsWithQuota')} value={creditsCount} hint={t('codex.accounts.stats.accountsWithQuotaHint')} />
          </div>
        </div>

        {accountList.length === 0 ? (
          <div className="py-16 text-center text-sm text-muted-foreground">{t('codex.accounts.empty')}</div>
        ) : (
          <>
            <div className="flex flex-col gap-3 rounded-xl border border-border/40 bg-muted/10 px-4 py-3">
              <div className="text-sm text-muted-foreground">
                {t('codex.accounts.listSummary', { visible: displayAccounts.length, total: accountList.length })}
              </div>

              <div className="flex flex-wrap items-center gap-3">
                <div className="flex items-center gap-2">
                  <Label htmlFor="codex-account-sort" className="text-xs text-muted-foreground">
                    {t('codex.accounts.controls.sortLabel')}
                  </Label>
                  <Select value={sortOption} onValueChange={(value) => setSortOption(value as CodexAccountSortOption)}>
                    <SelectTrigger id="codex-account-sort" className="h-8 w-[220px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="remaining-desc">{t('codex.accounts.controls.sortOptions.remainingDesc')}</SelectItem>
                      <SelectItem value="created-desc">{t('codex.accounts.controls.sortOptions.createdDesc')}</SelectItem>
                      <SelectItem value="created-asc">{t('codex.accounts.controls.sortOptions.createdAsc')}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="flex items-center gap-2">
                  <Label htmlFor="codex-account-status" className="text-xs text-muted-foreground">
                    {t('codex.accounts.controls.statusLabel')}
                  </Label>
                  <Select value={statusFilter} onValueChange={(value) => setStatusFilter(value as CodexAccountStatusFilter)}>
                    <SelectTrigger id="codex-account-status" className="h-8 w-[180px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">{t('codex.accounts.controls.statusOptions.all')}</SelectItem>
                      <SelectItem value="active">{t('codex.accounts.status.active')}</SelectItem>
                      <SelectItem value="expired">{t('codex.accounts.status.expired')}</SelectItem>
                      <SelectItem value="banned">{t('codex.accounts.status.banned')}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <FilterSwitch
                  id="codex-hide-no-quota"
                  checked={hideNoQuota}
                  onCheckedChange={setHideNoQuota}
                  label={t('codex.accounts.controls.hideNoQuota')}
                />
                <FilterSwitch
                  id="codex-only-valid-auth"
                  checked={onlyValidAuth}
                  onCheckedChange={setOnlyValidAuth}
                  label={t('codex.accounts.controls.onlyValidAuth')}
                />
                <FilterSwitch
                  id="codex-only-unlimited"
                  checked={onlyUnlimited}
                  onCheckedChange={setOnlyUnlimited}
                  label={t('codex.accounts.controls.onlyUnlimited')}
                />
              </div>
            </div>

            <Card className={cn('overflow-hidden', STATIC_CARD_CLASS)}>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border/50 text-xs text-muted-foreground">
                      <th className="px-4 py-3 text-left font-medium">{t('codex.accounts.table.account')}</th>
                      <th className="px-4 py-3 text-left font-medium">{t('codex.accounts.table.createdAt')}</th>
                      <th className="px-4 py-3 text-left font-medium">{t('codex.accounts.table.plan')}</th>
                      <th className="px-4 py-3 text-left font-medium">{t('codex.accounts.table.remaining')}</th>
                      <th className="px-4 py-3 text-left font-medium">{t('codex.accounts.table.reset')}</th>
                      <th className="px-4 py-3 text-right font-medium">{t('codex.accounts.table.actions')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {displayAccounts.length > 0 ? (
                      displayAccounts.map((account) => (
                        <AccountRow
                          key={account.id}
                          account={account}
                          isActive={isActiveAccount(activeAuth, account)}
                          isRefreshing={refreshingId === account.id}
                          locale={locale}
                          onRefreshQuota={() => handleRefreshQuota(account.id)}
                          isInboxLoading={loadingInboxId === account.id}
                          onSwitchAuth={() => switchAuth.mutate(account.id)}
                          onDelete={() => deleteAccount.mutate(account.id)}
                          onFetchInboxCode={() => handleFetchInboxCode(account)}
                          onGeneratePaymentLink={() => handleOpenPaymentDialog(account)}
                          onCopyAccount={(value) => copyText(value, 'codex.accounts.accountCopied')}
                          onCopyPassword={(value) => copyText(value, 'codex.accounts.passwordCopied')}
                        />
                      ))
                    ) : (
                      <tr>
                        <td colSpan={6} className="px-4 py-12 text-center text-sm text-muted-foreground">
                          {t('codex.accounts.filteredEmpty')}
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </Card>
          </>
        )}
      </div>

      <PaymentLinkDialog
        account={paymentAccount}
        form={paymentForm}
        result={generatedPaymentLink}
        isGenerating={generatePaymentLink.isPending}
        open={!!paymentAccount}
        onOpenChange={handlePaymentDialogChange}
        onCountryChange={handleCountryChange}
        onFormChange={updatePaymentForm}
        onGenerate={handleGeneratePaymentLink}
        onCopy={() => copyText(generatedPaymentLink?.url, 'codex.payment.linkCopied')}
        onOpenLink={async () => {
          if (!generatedPaymentLink?.url) return;
          try {
            await open(generatedPaymentLink.url);
          } catch (error) {
            toast.error(t('codex.payment.openFailed', { message: error instanceof Error ? error.message : String(error) }));
          }
        }}
      />

      <InboxCodeDialog
        open={inboxDialogOpen}
        result={inboxResult}
        isLoading={inboxCode.isPending}
        onOpenChange={(openState) => {
          setInboxDialogOpen(openState);
          if (!openState) {
            setInboxResult(null);
          }
        }}
        onCopy={() => copyText(inboxResult?.code, 'codex.accounts.inbox.codeCopied')}
      />
    </>
  );
}

function ActiveAuthCard({
  activeAuth,
  activeAccount,
  activeQuota,
  isLoading,
  isRefreshing,
  isLoggingOut,
  locale,
  onRefresh,
  onLogout,
  onCopyAccountId,
}: {
  activeAuth: CodexAuthInfoDto | null | undefined;
  activeAccount: CodexAccountDto | null;
  activeQuota: CodexQuotaDto | null;
  isLoading: boolean;
  isRefreshing: boolean;
  isLoggingOut: boolean;
  locale: string;
  onRefresh: () => void;
  onLogout: () => void;
  onCopyAccountId: (value: string | null | undefined) => void;
}) {
  const { t } = useTranslation();
  const displayWindow = pickPrimaryDisplayWindow(activeQuota?.secondaryWindow, activeQuota?.primaryWindow);
  const shortWindow = activeQuota?.primaryWindow ?? null;
  const weeklyWindow = activeQuota?.secondaryWindow ?? null;
  const progressValue = activeQuota?.isUnlimited ? 100 : getRemainingPercent(displayWindow) ?? 0;
  const resetHint = formatResetHint(displayWindow?.resetsAt, t);
  const remainingValue = formatRemainingPercent(displayWindow, activeQuota?.isUnlimited, t);

  return (
    <Card className={cn('h-full overflow-visible p-3', STATIC_CARD_CLASS)}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <ShieldCheck className="h-3.5 w-3.5 text-primary" />
            <span className="text-sm font-medium leading-none">{t('codex.accounts.activeAuth.title')}</span>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {activeAuth?.hasTokens && activeAuth.authMode === 'chatgpt' && (
            <Button variant="outline" size="sm" className="h-7 text-xs" onClick={onRefresh} disabled={isRefreshing}>
              <RefreshCw className={cn('h-3 w-3', isRefreshing && 'animate-spin')} />
              {t('codex.accounts.activeAuth.refreshQuota')}
            </Button>
          )}
          {activeAuth && (
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs text-destructive hover:bg-destructive/10"
              onClick={onLogout}
              disabled={isLoggingOut}
            >
              <LogOut className="h-3 w-3" />
              {t('codex.accounts.activeAuth.clear')}
            </Button>
          )}
        </div>
      </div>

      {isLoading ? (
        <div className="mt-3 text-xs text-muted-foreground">{t('codex.common.loading')}</div>
      ) : activeAuth ? (
        <div className="mt-2.5 lg:flex lg:flex-col">
          {activeAuth.authMode === 'chatgpt' ? (
            <div className="grid items-stretch gap-3 lg:grid-cols-[minmax(0,1fr)_260px]">
              <div className="rounded-xl border border-border/50 bg-muted/20 p-4 shadow-sm">
                <div className="flex flex-wrap items-start gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-1.5">
                      <span className="max-w-full truncate font-mono text-sm">{activeAuth.email ?? t('codex.common.unnamed')}</span>
                      <Badge variant="secondary" className="h-5 rounded px-1.5 text-[10px]">
                        {t('codex.accounts.activeAuth.current')}
                      </Badge>
                      <Badge variant="outline" className="h-5 rounded px-1.5 text-[10px]">
                        {activeAuth.authMode}
                      </Badge>
                      {activeQuota?.planType && (
                        <Badge variant="outline" className="h-5 rounded px-1.5 text-[10px]">
                          {formatPlanLabel(activeQuota.planType, t)}
                        </Badge>
                      )}
                    </div>
                    <p className="mt-1 text-[11px] text-muted-foreground">
                      {t('codex.accounts.activeAuth.fileSource')}
                    </p>
                  </div>
                </div>

                <div className="mt-4 grid gap-3 sm:grid-cols-2">
                  <ProfileInfoBlock
                    label={t('codex.accounts.activeAuth.sourceAccount')}
                    value={activeAccount?.email ?? t('codex.accounts.activeAuth.unlinked')}
                    mono={!!activeAccount?.email}
                  />
                  <ProfileInfoBlock
                    label={t('codex.accounts.activeAuth.lastRefresh')}
                    value={formatResetAt(activeAuth.lastRefresh, locale, t('codex.common.empty'))}
                  />
                  <ProfileInfoBlock
                    label={t('codex.accounts.activeAuth.accountId')}
                    value={activeAuth.accountId ?? t('codex.common.empty')}
                    mono={!!activeAuth.accountId}
                    action={
                      activeAuth.accountId ? (
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          className="h-5 w-5 shrink-0"
                          title={t('codex.accounts.copyAccountId')}
                          onClick={() => onCopyAccountId(activeAuth.accountId)}
                        >
                          <Copy className="h-3 w-3" />
                        </Button>
                      ) : null
                    }
                  />
                  <ProfileInfoBlock
                    label={t('codex.accounts.activeAuth.statusLabel')}
                    value={activeQuota ? t('codex.accounts.activeAuth.cachedQuota') : t('codex.accounts.activeAuth.manualRefreshRequired')}
                  />
                </div>

                {activeAuth.quotaError && (
                  <div className="mt-3 text-xs text-yellow-600">
                    {t('codex.accounts.activeAuth.quotaError', { message: activeAuth.quotaError })}
                  </div>
                )}
              </div>

              <div className="flex flex-col rounded-xl border border-primary/20 bg-primary/[0.04] p-4 shadow-sm">
                <div className="text-[11px] text-muted-foreground">{t('codex.accounts.activeAuth.remaining')}</div>
                <div className="mt-1 text-3xl font-semibold tracking-tight">
                  {activeQuota ? remainingValue : t('codex.common.empty')}
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  {activeQuota
                    ? formatResetAt(displayWindow?.resetsAt, locale, t('codex.common.empty'))
                    : t('codex.accounts.activeAuth.manualRefreshRequired')}
                </div>

                <div className="mt-4 space-y-3">
                  <QuotaInfoRow
                    label={formatWindowLabel(shortWindow, t)}
                    value={activeQuota ? formatRemainingPercent(shortWindow, false, t) : t('codex.common.empty')}
                  />
                  <QuotaInfoRow
                    label={formatWindowLabel(weeklyWindow, t)}
                    value={activeQuota ? formatRemainingPercent(weeklyWindow, activeQuota.isUnlimited, t) : t('codex.common.empty')}
                  />
                </div>

                <div className="mt-auto pt-4">
                  <Progress value={activeQuota ? progressValue : 0} className="h-1.5" />
                  {activeQuota ? (
                    resetHint && <div className="mt-2 line-clamp-2 text-[11px] text-muted-foreground">{resetHint}</div>
                  ) : (
                    <div className="mt-2 text-[11px] text-muted-foreground">
                      {t('codex.accounts.activeAuth.manualRefreshHint')}
                    </div>
                  )}
                </div>
              </div>
            </div>
          ) : (
            <div className="text-xs text-muted-foreground">{t('codex.accounts.activeAuth.nonChatgptMode')}</div>
          )}
        </div>
      ) : (
        <div className="mt-3 text-sm text-muted-foreground">{t('codex.accounts.activeAuth.empty')}</div>
      )}
    </Card>
  );
}

function AccountRow({
  account,
  isActive,
  isRefreshing,
  isInboxLoading,
  locale,
  onRefreshQuota,
  onSwitchAuth,
  onDelete,
  onFetchInboxCode,
  onGeneratePaymentLink,
  onCopyAccount,
  onCopyPassword,
}: {
  account: CodexAccountDto;
  isActive: boolean;
  isRefreshing: boolean;
  isInboxLoading: boolean;
  locale: string;
  onRefreshQuota: () => void;
  onSwitchAuth: () => void;
  onDelete: () => void;
  onFetchInboxCode: () => void;
  onGeneratePaymentLink: () => void;
  onCopyAccount: (value: string | null | undefined) => void;
  onCopyPassword: (value: string | null | undefined) => void;
}) {
  const { t } = useTranslation();
  const displayWindow = pickPrimaryDisplayWindow(account.secondaryWindow, account.primaryWindow);
  const progressValue = account.isUnlimited ? 100 : getRemainingPercent(displayWindow) ?? 0;
  const resetHint = formatResetHint(displayWindow?.resetsAt, t);
  const detailWindows: Array<{
    window: NonNullable<CodexAccountDto['primaryWindow']>;
    isUnlimited?: boolean | null;
  }> = [];

  if (account.primaryWindow && account.primaryWindow !== displayWindow) {
    detailWindows.push({ window: account.primaryWindow, isUnlimited: false });
  }

  if (account.secondaryWindow && account.secondaryWindow !== displayWindow) {
    detailWindows.push({ window: account.secondaryWindow, isUnlimited: account.isUnlimited });
  }

  return (
    <tr className={cn('border-b border-border/20 transition-colors hover:bg-muted/20', isActive && 'bg-primary/5')}>
      <td className="px-4 py-3 align-top">
        <div className="flex items-center gap-2">
          {isActive && <Star className="h-3.5 w-3.5 fill-amber-400 text-amber-500" />}
          {account.status !== 'active' && <StatusBadge status={account.status} />}
          {account.status === 'active' && account.isTokenExpired && (
            <Badge variant="outline" className="h-5 rounded px-1.5 text-[10px] text-yellow-600">
              {t('codex.accounts.tokenExpired')}
            </Badge>
          )}
          <span className="max-w-[260px] truncate font-mono text-xs">{account.email}</span>
          <Button
            variant="ghost"
            size="icon-sm"
            className="h-6 w-6 shrink-0"
            title={t('codex.accounts.copyAccount')}
            onClick={() => onCopyAccount(account.email)}
          >
            <Copy className="h-3 w-3" />
          </Button>
        </div>

        <div className="mt-2 flex flex-wrap items-center gap-2 text-[11px]">
          <span className="rounded border border-border/40 bg-muted/30 px-1.5 py-0.5 text-muted-foreground">
            {t('codex.accounts.password')}
          </span>
          <span className={cn('font-mono', account.password ? 'text-foreground' : 'text-muted-foreground')}>
            {account.password ? maskPassword(account.password) : t('codex.accounts.passwordMissing')}
          </span>
          {account.password && (
            <Button
              variant="ghost"
              size="icon-sm"
              className="h-6 w-6"
              title={t('codex.accounts.copyPassword')}
              onClick={() => onCopyPassword(account.password)}
            >
              <Copy className="h-3 w-3" />
            </Button>
          )}
        </div>
      </td>

      <td className="px-4 py-3 align-top">
        <div className="text-xs">{formatCompactDateTime(account.createdAt, locale, t('codex.common.empty'))}</div>
      </td>

      <td className="px-4 py-3 align-top">
        {account.planType ? (
          <Badge variant="outline" className="text-xs">
            {formatPlanLabel(account.planType, t)}
          </Badge>
        ) : (
          <span className="text-xs text-muted-foreground">{t('codex.common.empty')}</span>
        )}
      </td>

      <td className="px-4 py-3 align-top">
        <div className="min-w-[180px]">
          <div className="flex items-center justify-between text-xs">
            <span className="font-medium text-foreground">{formatRemainingPercent(displayWindow, account.isUnlimited, t)}</span>
            <span className="text-muted-foreground">{formatWindowLabel(displayWindow, t)}</span>
          </div>
          <Progress value={progressValue} className="mt-2 h-1.5" />
          {detailWindows.length > 0 && (
            <div className="mt-1 flex flex-wrap gap-2 text-[11px] text-muted-foreground">
              {detailWindows.map(({ window, isUnlimited }, index) => (
                <span key={`${window.windowMinutes ?? 'window'}-${window.resetsAt ?? 'na'}-${index}`}>
                  {formatWindowLabel(window, t)} {formatRemainingPercent(window, isUnlimited, t)}
                </span>
              ))}
            </div>
          )}
        </div>
      </td>

      <td className="px-4 py-3 align-top">
        <div className="text-xs">{formatResetAt(displayWindow?.resetsAt, locale, t('codex.common.empty'))}</div>
        {resetHint && <div className="mt-1 text-[11px] text-muted-foreground">{resetHint}</div>}
      </td>

      <td className="px-4 py-3 align-top">
        <div className="flex items-center justify-end gap-1.5">
          <ActionButton
            title={t('codex.accounts.actions.payment')}
            onClick={onGeneratePaymentLink}
            disabled={!account.hasTokens || account.isTokenExpired}
          >
            <CreditCard className="h-3.5 w-3.5" />
          </ActionButton>
          <ActionButton
            title={t('codex.accounts.actions.switchAuth')}
            onClick={onSwitchAuth}
            disabled={!account.hasTokens || account.isTokenExpired}
          >
            <ArrowRightLeft className="h-3.5 w-3.5" />
          </ActionButton>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="outline"
                size="icon-sm"
                className="h-7 w-7"
                title={t('codex.accounts.actions.more')}
                aria-label={t('codex.accounts.actions.more')}
              >
                <MoreHorizontal className="h-3.5 w-3.5" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={onFetchInboxCode}
                disabled={isInboxLoading || account.source !== 'register'}
              >
                <Mail className="mr-2 h-3.5 w-3.5" />
                {t('codex.accounts.actions.inbox')}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={onRefreshQuota} disabled={isRefreshing || !account.hasTokens}>
                <RefreshCw className={cn('mr-2 h-3.5 w-3.5', isRefreshing && 'animate-spin')} />
                {t('codex.accounts.actions.refreshQuota')}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={onDelete}
                className="text-destructive focus:text-destructive"
              >
                <Trash2 className="mr-2 h-3.5 w-3.5" />
                {t('codex.accounts.actions.delete')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </td>
    </tr>
  );
}

function InboxCodeDialog({
  open,
  result,
  isLoading,
  onOpenChange,
  onCopy,
}: {
  open: boolean;
  result: CodexInboxCodeDto | null;
  isLoading: boolean;
  onOpenChange: (open: boolean) => void;
  onCopy: () => void;
}) {
  const { t } = useTranslation();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t('codex.accounts.inbox.title')}</DialogTitle>
          <DialogDescription>{t('codex.accounts.inbox.description')}</DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="py-8 text-center text-sm text-muted-foreground">{t('codex.accounts.inbox.loading')}</div>
        ) : result ? (
          <div className="space-y-4">
            <div className="rounded-lg border border-border/50 bg-muted/20 p-3">
              <div className="text-xs text-muted-foreground">{t('codex.accounts.inbox.email')}</div>
              <div className="mt-1 font-mono text-sm">{result.email}</div>
            </div>
            <div className="rounded-lg border border-primary/20 bg-primary/[0.04] p-4 text-center">
              <div className="text-xs text-muted-foreground">{t('codex.accounts.inbox.code')}</div>
              <div className="mt-2 font-mono text-3xl font-semibold tracking-[0.2em]">{result.code}</div>
            </div>
            <div className="flex items-center justify-end gap-2">
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                {t('codex.common.close')}
              </Button>
              <Button onClick={onCopy}>
                <Copy className="h-4 w-4" />
                {t('codex.accounts.inbox.copy')}
              </Button>
            </div>
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

function PaymentLinkDialog({
  account,
  form,
  result,
  isGenerating,
  open,
  onOpenChange,
  onCountryChange,
  onFormChange,
  onGenerate,
  onCopy,
  onOpenLink,
}: {
  account: CodexAccountDto | null;
  form: CodexPaymentFormState;
  result: CodexPaymentLinkDto | null;
  isGenerating: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCountryChange: (country: string) => void;
  onFormChange: (patch: Partial<CodexPaymentFormState>) => void;
  onGenerate: () => void;
  onCopy: () => void;
  onOpenLink: () => void;
}) {
  const { t } = useTranslation();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t('codex.payment.dialogTitle')}</DialogTitle>
          <DialogDescription>
            {account
              ? t('codex.payment.dialogDescriptionWithAccount', { email: account.email })
              : t('codex.payment.dialogDescription')}
          </DialogDescription>
        </DialogHeader>

        {account && (
          <div className="space-y-4">
            <div className="rounded-lg border border-border/50 bg-muted/20 p-3 text-xs text-muted-foreground">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-mono text-foreground">{account.email}</span>
                {account.hasPaymentSession ? (
                  <Badge variant="outline" className="text-xs">{t('codex.accounts.paymentSessionReady')}</Badge>
                ) : (
                  <Badge variant="outline" className="text-xs text-yellow-600">{t('codex.payment.tokenOnlyHint')}</Badge>
                )}
              </div>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <Label>{t('codex.payment.plan')}</Label>
                <Select
                  value={form.planType}
                  onValueChange={(value: CodexPaymentPlanDto) => onFormChange({ planType: value })}
                >
                  <SelectTrigger>
                    <SelectValue placeholder={t('codex.payment.planPlaceholder')} />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="plus">Plus</SelectItem>
                    <SelectItem value="team">Team</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>{t('codex.payment.countryLabel')}</Label>
                <Select value={form.country} onValueChange={onCountryChange}>
                  <SelectTrigger>
                    <SelectValue placeholder={t('codex.payment.countryPlaceholder')} />
                  </SelectTrigger>
                  <SelectContent>
                    {PAYMENT_COUNTRIES.map((item) => (
                      <SelectItem key={item.code} value={item.code}>
                        {t(item.labelKey)} ({item.code})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>{t('codex.payment.currencyLabel')}</Label>
                <Input value={form.currency} readOnly />
              </div>

              {form.planType === 'team' && (
                <div className="space-y-2">
                  <Label>{t('codex.payment.intervalLabel')}</Label>
                  <Select
                    value={form.priceInterval}
                    onValueChange={(value: 'month' | 'year') => onFormChange({ priceInterval: value })}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder={t('codex.payment.intervalPlaceholder')} />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="month">{t('codex.payment.interval.month')}</SelectItem>
                      <SelectItem value="year">{t('codex.payment.interval.year')}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              )}

              {form.planType === 'team' && (
                <>
                  <div className="space-y-2">
                    <Label>{t('codex.payment.workspaceLabel')}</Label>
                    <Input
                      value={form.workspaceName}
                      onChange={(event) => onFormChange({ workspaceName: event.target.value })}
                      placeholder={t('codex.payment.workspacePlaceholder')}
                    />
                  </div>

                  <div className="space-y-2">
                    <Label>{t('codex.payment.seatsLabel')}</Label>
                    <Input
                      type="number"
                      min={1}
                      value={form.seatQuantity}
                      onChange={(event) => onFormChange({ seatQuantity: event.target.value })}
                    />
                  </div>
                </>
              )}
            </div>

            <div className="flex items-center justify-end gap-2">
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                {t('codex.common.close')}
              </Button>
              <Button onClick={onGenerate} disabled={isGenerating}>
                {isGenerating ? (
                  <>
                    <RefreshCw className="h-4 w-4 animate-spin" />
                    {t('codex.payment.generating')}
                  </>
                ) : (
                  <>
                    <CreditCard className="h-4 w-4" />
                    {t('codex.payment.generate')}
                  </>
                )}
              </Button>
            </div>

            {result && (
              <div className="rounded-lg border border-border/50 bg-muted/20 p-4">
                <div className="mb-3 flex flex-wrap items-center gap-2">
                  <Badge variant="outline" className="text-xs">
                    {result.planType === 'plus' ? 'Plus' : 'Team'}
                  </Badge>
                  <span className="text-xs text-muted-foreground">
                    {result.country} / {result.currency}
                  </span>
                </div>

                <div className="space-y-2">
                  <Label>{t('codex.payment.generatedLink')}</Label>
                  <Textarea value={result.url} readOnly className="min-h-[96px] font-mono text-xs" />
                </div>

                <div className="mt-3 flex items-center justify-end gap-2">
                  <Button variant="outline" size="sm" className="h-8 text-xs" onClick={onCopy}>
                    <Copy className="h-3 w-3" />
                    {t('codex.payment.copy')}
                  </Button>
                  <Button size="sm" className="h-8 text-xs" onClick={onOpenLink}>
                    <ExternalLink className="h-3 w-3" />
                    {t('codex.payment.open')}
                  </Button>
                </div>
              </div>
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

function ProfileInfoBlock({
  label,
  value,
  mono = false,
  action = null,
}: {
  label: string;
  value: string;
  mono?: boolean;
  action?: ReactNode;
}) {
  return (
    <div className="min-w-0 rounded-lg border border-border/40 bg-background/70 p-3">
      <div className="text-[10px] text-muted-foreground">{label}</div>
      <div className="mt-1 flex min-w-0 items-center gap-1.5">
        <div className={cn('min-w-0 flex-1 truncate text-[12px] text-foreground', mono && 'font-mono text-[11px]')}>
          {value}
        </div>
        {action}
      </div>
    </div>
  );
}

function QuotaInfoRow({ label, value }: {
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium text-foreground">{value}</span>
    </div>
  );
}

function StatCard({ label, value, hint }: { label: string; value: number; hint: string }) {
  return (
    <Card className={cn('h-full p-2.5', STATIC_CARD_CLASS)}>
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 text-xl font-semibold">{value}</div>
      <div className="mt-0.5 text-[11px] text-muted-foreground">{hint}</div>
    </Card>
  );
}

function FilterSwitch({
  id,
  checked,
  onCheckedChange,
  label,
}: {
  id: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  label: string;
}) {
  return (
    <div className="flex items-center gap-2 rounded-lg border border-border/40 bg-background/70 px-3 py-2">
      <Switch id={id} checked={checked} onCheckedChange={onCheckedChange} />
      <Label htmlFor={id} className="cursor-pointer text-xs text-muted-foreground">
        {label}
      </Label>
    </div>
  );
}

function ActionButton({
  title,
  onClick,
  disabled,
  children,
}: {
  title: string;
  onClick: () => void;
  disabled?: boolean;
  children: ReactNode;
}) {
  return (
    <Button
      variant="outline"
      size="icon-sm"
      className="h-7 w-7"
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={title}
    >
      {children}
    </Button>
  );
}

function StatusBadge({ status }: { status: string }) {
  const { t } = useTranslation();
  const styles: Record<string, string> = {
    active: 'bg-green-500/15 text-green-600 border-green-500/20',
    expired: 'bg-yellow-500/15 text-yellow-600 border-yellow-500/20',
    banned: 'bg-red-500/15 text-red-600 border-red-500/20',
  };

  const labels: Record<string, string> = {
    active: t('codex.accounts.status.active'),
    expired: t('codex.accounts.status.expired'),
    banned: t('codex.accounts.status.banned'),
  };

  return (
    <span
      className={cn(
        'inline-flex items-center rounded border px-2 py-0.5 text-[11px]',
        styles[status] ?? 'border-transparent bg-muted text-muted-foreground'
      )}
    >
      {labels[status] ?? status}
    </span>
  );
}

function isActiveAccount(activeAuth: CodexAuthInfoDto | null | undefined, account: CodexAccountDto) {
  if (!activeAuth) return false;
  if (activeAuth.accountId && account.accountId) {
    return activeAuth.accountId === account.accountId;
  }
  return activeAuth.email === account.email;
}

function buildQuotaFromAccount(account: CodexAccountDto | null): CodexQuotaDto | null {
  if (!account) return null;

  return {
    planType: account.planType,
    hasCredits: account.hasCredits,
    isUnlimited: account.isUnlimited,
    creditBalance: account.creditBalance,
    primaryWindow: account.primaryWindow,
    secondaryWindow: account.secondaryWindow,
  };
}

function getCurrencyForCountry(country: string) {
  return PAYMENT_COUNTRIES.find((item) => item.code === country)?.currency ?? 'USD';
}

function maskPassword(password: string) {
  const maskLength = Math.min(Math.max(password.length, 8), 12);
  return '•'.repeat(maskLength);
}

function formatCompactDateTime(value?: string | null, locale = 'zh-CN', emptyLabel = '—') {
  if (!value) return emptyLabel;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return emptyLabel;

  return date.toLocaleString(locale, {
    hour12: false,
    year: '2-digit',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}
