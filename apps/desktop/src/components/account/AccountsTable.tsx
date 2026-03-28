import React from 'react';
import { useTranslation } from 'react-i18next';
import { MoreVertical, Calendar, ArrowUpDown, ArrowUp, ArrowDown, RefreshCw, KeyRound } from 'lucide-react';
import { TableVirtuoso } from 'react-virtuoso';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Badge } from '@/components/ui/badge';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { ProviderDto } from '@/hooks/useProviders';
import { Account } from '@/lib/tauri-commands';
import { cn } from '@/lib/utils';
import { format } from 'date-fns';
import { motion } from 'framer-motion';

interface AccountsTableProps {
  accounts: Account[];
  onAccountClick: (account: Account) => void;
  onCheckIn: (accountId: string) => void;
  onEdit: (account: Account) => void;
  onToggle: (account: Account) => void;
  onDelete: (account: Account) => void;
  onRefreshBalance: (accountId: string) => void;
  checkingInIds?: Set<string>;
  sortConfig?: { key: keyof Account; direction: 'asc' | 'desc' } | null;
  onSortChange?: (config: { key: keyof Account; direction: 'asc' | 'desc' } | null) => void;
  providersById?: Record<string, ProviderDto>;
}

export function AccountsTable({
  accounts,
  onAccountClick,
  onCheckIn,
  onEdit,
  onToggle,
  onDelete,
  onRefreshBalance,
  checkingInIds = new Set(),
  sortConfig,
  onSortChange,
  providersById = {},
}: AccountsTableProps) {
  const { t } = useTranslation();

  const formatLastCheckIn = (timestamp?: string) => {
    if (!timestamp) return '-';
    try {
      const date = new Date(timestamp);
      return format(date, 'yyyy-MM-dd HH:mm:ss');
    } catch {
      return '-';
    }
  };

  const handleSort = (key: keyof Account) => {
    if (!onSortChange) return;

    let direction: 'asc' | 'desc' = 'asc';
    if (sortConfig && sortConfig.key === key && sortConfig.direction === 'asc') {
      direction = 'desc';
    }
    onSortChange({ key, direction });
  };

  const SortIcon = ({ columnKey }: { columnKey: keyof Account }) => {
    if (sortConfig?.key !== columnKey) {
      return <ArrowUpDown className="ml-2 h-3 w-3 opacity-30" />;
    }
    return sortConfig.direction === 'asc' ? (
      <ArrowUp className="ml-2 h-3 w-3 text-primary" />
    ) : (
      <ArrowDown className="ml-2 h-3 w-3 text-primary" />
    );
  };

  const SortableHeader = ({ 
    label, 
    columnKey, 
    className 
  }: { 
    label: string; 
    columnKey: keyof Account; 
    className?: string 
  }) => (
    <div 
      className={cn("flex items-center cursor-pointer select-none hover:text-foreground transition-colors", className)}
      onClick={() => handleSort(columnKey)}
    >
      {label}
      <SortIcon columnKey={columnKey} />
    </div>
  );

  return (
    <motion.div 
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
      className="h-full rounded-lg border border-border/50 shadow-sm bg-card/50 backdrop-blur-sm overflow-hidden"
    >
      {accounts.length === 0 ? (
        <div className="h-full flex flex-col items-center justify-center text-muted-foreground">
           {t('accounts.noAccounts')}
        </div>
      ) : (
        <TableVirtuoso
          className="auto-hide-scrollbar"
          style={{ height: '100%' }}
          data={accounts}
          components={{
            Table: (props) => (
              <table {...props} className="w-full caption-bottom text-sm border-collapse" />
            ),
            TableHead: React.forwardRef((props, ref) => (
              <thead {...props} ref={ref} className="bg-muted/50 sticky top-0 z-20 shadow-sm [&_tr]:border-b backdrop-blur-md" />
            )),
            TableRow: (props) => (
              <tr 
                {...props} 
                  className={cn(
                    "border-b transition-all duration-base ease-smooth relative group cursor-pointer",
                    "hover:bg-primary/[0.03] hover:shadow-md hover:-translate-y-[2px] hover:z-10",
                    "active:scale-[var(--scale-active-subtle)] active:translate-y-0 active:shadow-sm",
                    "data-[state=selected]:bg-muted"
                  )}
              />
            ),
            TableBody: React.forwardRef((props, ref) => (
              <tbody {...props} ref={ref} className="[&_tr:last-child]:border-0 bg-transparent" />
            )),
          }}
          fixedHeaderContent={() => (
            <tr className="h-10 text-left align-middle font-medium text-muted-foreground">
              <th className="px-4 py-2 pl-6 font-semibold bg-muted/50 backdrop-blur-sm">
                <SortableHeader label={t('management.accountName', 'Account Name')} columnKey="name" />
              </th>
              <th className="px-4 py-2 text-center font-semibold bg-muted/50 backdrop-blur-sm">
                {t('management.autoCheckIn', 'Auto Check-in')}
              </th>
              <th className="px-4 py-2 font-semibold bg-muted/50 backdrop-blur-sm">
                <SortableHeader label={t('management.balance', 'Balance')} columnKey="current_balance" className="justify-end" />
              </th>
              <th className="px-4 py-2 font-semibold bg-muted/50 backdrop-blur-sm">
                <SortableHeader label={t('management.totalQuota', 'Total Quota')} columnKey="total_quota" className="justify-end" />
              </th>
              <th className="px-4 py-2 font-semibold bg-muted/50 backdrop-blur-sm">
                <SortableHeader label={t('management.totalConsumed', 'Historical Consumption')} columnKey="total_consumed" className="justify-end" />
              </th>
              <th className="px-4 py-2 font-semibold bg-muted/50 backdrop-blur-sm">
                <SortableHeader label={t('management.lastCheckIn', 'Last Check-in')} columnKey="last_check_in" />
              </th>
              <th className="px-4 py-2 text-center font-semibold bg-muted/50 backdrop-blur-sm min-w-[150px]">
                {t('management.session', 'Session')}
              </th>
              <th className="px-4 py-2 text-center font-semibold pr-6 bg-muted/50 backdrop-blur-sm w-32">
                {t('management.actions', 'Actions')}
              </th>
            </tr>
          )}
          itemContent={(_, account) => {
            const isChecking = checkingInIds.has(account.id);
            return (
              <>
                <td 
                  className="p-2 pl-6 align-middle font-medium cursor-pointer"
                  onClick={() => onAccountClick(account)}
                >
                  <div className="flex flex-col gap-1">
                    <span className="text-sm font-semibold">{account.name}</span>
                    <Badge variant="outline" className="font-normal text-xs w-fit">
                      {account.provider_name}
                    </Badge>
                  </div>
                </td>
                <td 
                  className="p-2 align-middle text-center cursor-pointer"
                  onClick={() => onAccountClick(account)}
                >
                  {account.auto_checkin_enabled ? (
                    <div className="flex items-center justify-center gap-1.5">
                      <span className="inline-block w-2 h-2 rounded-full bg-success animate-pulse"></span>
                      <span className="text-xs font-medium text-success">
                        {String(account.auto_checkin_hour).padStart(2, '0')}:{String(account.auto_checkin_minute).padStart(2, '0')}
                      </span>
                    </div>
                  ) : (
                    <span className="text-xs text-muted-foreground">-</span>
                  )}
                </td>
                <td 
                  className="p-2 align-middle text-right cursor-pointer"
                  onClick={() => onAccountClick(account)}
                >
                  <span className="font-mono text-sm font-semibold text-success">
                    {account.current_balance != null
                      ? `$${account.current_balance.toFixed(2)}`
                      : '-'
                    }
                  </span>
                </td>
                <td 
                  className="p-2 align-middle text-right cursor-pointer"
                  onClick={() => onAccountClick(account)}
                >
                  <span className="font-mono text-sm font-semibold text-info">
                    {account.total_quota != null
                      ? `$${account.total_quota.toFixed(2)}`
                      : '-'
                    }
                  </span>
                </td>
                <td 
                  className="p-2 align-middle text-right cursor-pointer"
                  onClick={() => onAccountClick(account)}
                >
                  <span className="font-mono text-sm font-semibold text-warning">
                    {account.total_consumed != null
                      ? `$${account.total_consumed.toFixed(2)}`
                      : '-'
                    }
                  </span>
                </td>
                <td 
                  className="p-2 align-middle cursor-pointer"
                  onClick={() => onAccountClick(account)}
                >
                  <span className="text-xs text-muted-foreground font-mono">
                    {formatLastCheckIn(account.last_check_in ?? undefined)}
                  </span>
                </td>
                <td
                  className="p-2 align-middle text-center cursor-pointer"
                  onClick={() => onAccountClick(account)}
                >
                  <Badge
                    variant={
                      account.session_days_remaining == null
                        ? 'secondary'
                        : account.session_days_remaining <= 0
                          ? 'soft-danger'
                          : account.session_expires_soon
                            ? 'soft-warning'
                            : 'soft-success'
                    }
                    className="inline-flex items-center gap-1 whitespace-nowrap text-[10px] px-2 py-0.5"
                  >
                    <KeyRound className="h-3 w-3" />
                    {account.session_days_remaining == null
                      ? t('accountCard.sessionUnknown')
                      : account.session_days_remaining <= 0
                        ? t('accountCard.sessionExpired')
                        : t('accountCard.sessionValidDays', { days: account.session_days_remaining })
                    }
                  </Badge>
                </td>
                <td className="p-2 align-middle pr-6">
                  <div className="flex items-start justify-center gap-2">
                    <div className="flex flex-col items-center gap-1">
                      {(() => {
                        const provider = providersById[account.provider_id];
                        const supportsCheckIn = provider?.supports_check_in ?? true;
                        const checkInBugged = provider?.check_in_bugged ?? false;
                        const isBugged = supportsCheckIn && checkInBugged;
                        const buttonDisabled = !account.enabled || (supportsCheckIn && isChecking);
                        const buttonLabel = !supportsCheckIn
                          ? t('management.balance', 'Balance')
                          : isChecking
                          ? t('checkIn.checking', 'Checking in...')
                          : t('checkIn.checkIn', 'Check In');
                        const buttonIcon = supportsCheckIn ? (
                          <Calendar className="mr-1.5 h-3.5 w-3.5" />
                        ) : (
                          <RefreshCw className="mr-1.5 h-3.5 w-3.5" />
                        );
                        const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
                          e.stopPropagation();
                          if (!supportsCheckIn || isBugged) {
                            onRefreshBalance(account.id);
                            return;
                          }
                          onCheckIn(account.id);
                        };
                        const button = (
                          <Button
                            variant="default"
                            size="sm"
                            onClick={handleClick}
                            disabled={buttonDisabled}
                            className={cn(
                              "h-8 px-4 text-xs font-bold shadow-sm transition-all duration-slow ease-smooth rounded-lg border-0",
                              supportsCheckIn
                                ? 'btn-gradient-primary text-primary-foreground'
                                : 'btn-gradient-success text-white',
                              'disabled:opacity-50 disabled:cursor-not-allowed',
                              isChecking && supportsCheckIn && 'animate-pulse'
                            )}
                          >
                            {buttonIcon}
                            {buttonLabel}
                          </Button>
                        );

                        const tooltipMessage = !supportsCheckIn
                          ? t('checkIn.unsupportedProvider', 'This provider only supports balance refresh')
                          : isBugged
                          ? t('checkIn.buggedProvider', 'Check-in is temporarily unavailable. Please refresh balance')
                          : null;

                        if (tooltipMessage) {
                          return (
                            <Tooltip>
                              <TooltipTrigger asChild>{button}</TooltipTrigger>
                              <TooltipContent>
                                <p>{tooltipMessage}</p>
                              </TooltipContent>
                            </Tooltip>
                          );
                        }

                        return button;
                      })()}
                    </div>

                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="icon-sm" className="h-8 w-8 hover:bg-muted">
                          <MoreVertical className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => onRefreshBalance(account.id)}>
                          {t('accountCard.refreshBalance')}
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => onEdit(account)}>
                          {t('accountCard.edit')}
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => onToggle(account)}>
                          {account.enabled ? t('accountCard.disable') : t('accountCard.enable')}
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          onClick={() => onDelete(account)}
                          className="text-destructive"
                        >
                          {t('accountCard.delete')}
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                </td>
              </>
            );
          }}
        />
      )}
    </motion.div>
  );
}
