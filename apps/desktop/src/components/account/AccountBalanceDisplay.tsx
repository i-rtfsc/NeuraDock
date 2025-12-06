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
    <div className="relative">
      {/* Refresh Button */}
      <Button
        variant="ghost"
        size="icon"
        className="absolute -top-1 -right-1 h-6 w-6 rounded-full z-10"
        onClick={(e) => {
          e.stopPropagation();
          onRefresh();
        }}
        disabled={isRefreshing || isLoading}
        title={t('accountCard.refreshBalance') || 'Refresh balance'}
      >
        <RefreshCw className={`h-3 w-3 ${isRefreshing ? 'animate-spin' : ''}`} />
      </Button>

      <div className="grid grid-cols-3 gap-2 text-xs bg-muted/30 rounded-xl p-3">
        {balance ? (
          <>
            <div className="text-center">
              <p className="text-muted-foreground mb-0.5">{t('accountCard.totalIncome')}</p>
              <p className="font-semibold text-blue-600">{formatCurrency(balance.total_income)}</p>
            </div>
            <div className="text-center border-x">
              <p className="text-muted-foreground mb-0.5">{t('accountCard.historicalConsumption')}</p>
              <p className="font-semibold text-orange-600">{formatCurrency(balance.total_consumed)}</p>
            </div>
            <div className="text-center">
              <p className="text-muted-foreground mb-0.5">{t('accountCard.currentBalance')}</p>
              <p className="font-semibold text-green-600">{formatCurrency(balance.current_balance)}</p>
            </div>
          </>
        ) : (
          <div className="col-span-3 flex items-center justify-center py-2">
            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        )}
      </div>
    </div>
  );
}
