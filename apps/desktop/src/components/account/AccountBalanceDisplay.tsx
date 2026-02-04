import { Loader2, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { formatCurrency } from '@/lib/formatters';
import { useTranslation } from 'react-i18next';
import { BalanceDto } from '@/hooks/useBalance';

interface AccountBalanceDisplayProps {
  balance: BalanceDto | null;
  isLoading: boolean;
  isRefreshing: boolean;
  onRefresh: () => void;
}

export function AccountBalanceDisplay({
  balance,
  isLoading,
  isRefreshing,
  onRefresh,
}: AccountBalanceDisplayProps) {
  const { t } = useTranslation();

  if (!balance && !isLoading && !isRefreshing) {
    return null;
  }

  return (
    <div className="relative group/balance">
      {/* Refresh Button - Always visible */}
      <Button
        variant="ghost"
        size="icon"
        className="absolute top-0 right-0 h-6 w-6 rounded-full z-10 text-muted-foreground/50 hover:text-foreground transition-colors"
        onClick={(e) => {
          e.stopPropagation();
          onRefresh();
        }}
        disabled={isRefreshing || isLoading}
        title={t('accountCard.refreshBalance') || 'Refresh balance'}
      >
        <RefreshCw className={`h-3 w-3 ${isRefreshing ? 'animate-spin' : ''}`} />
      </Button>

      <div className="space-y-2 pt-1">
        {balance ? (
          <>
            <div className="flex justify-between items-baseline pr-7">
              <span className="text-xs text-muted-foreground">{t('accountCard.currentBalance')}</span>
              <span className="font-bold text-sm text-success truncate max-w-[120px]" title={formatCurrency(balance.current_balance)}>
                {formatCurrency(balance.current_balance)}
              </span>
            </div>
            <div className="flex justify-between items-baseline">
              <span className="text-xs text-muted-foreground">{t('accountCard.totalQuota')}</span>
              <span className="font-medium text-xs text-info truncate max-w-[120px]" title={formatCurrency(balance.total_quota)}>
                {formatCurrency(balance.total_quota)}
              </span>
            </div>
            <div className="flex justify-between items-baseline">
              <span className="text-xs text-muted-foreground">{t('accountCard.historicalConsumption')}</span>
              <span className="font-medium text-xs text-warning truncate max-w-[120px]" title={formatCurrency(balance.total_consumed)}>
                {formatCurrency(balance.total_consumed)}
              </span>
            </div>
          </>
        ) : (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
          </div>
        )}
      </div>
    </div>
  );
}
