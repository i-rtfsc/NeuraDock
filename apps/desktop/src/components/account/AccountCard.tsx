import { useState } from 'react';
import { Account } from '@/lib/tauri-commands';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useDeleteAccount, useToggleAccount } from '@/hooks/useAccounts';
import { useFetchAccountBalance } from '@/hooks/useBalance';
import { CheckInButton } from '@/components/checkin/CheckInButton';
import { toast } from 'sonner';
import {
  MoreVertical,
  Edit,
  Trash2,
  Power,
  PowerOff,
  Loader2,
  Clock,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface AccountCardProps {
  account: Account;
  onEdit: (account: Account) => void;
}

export function AccountCard({ account, onEdit }: AccountCardProps) {
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const deleteMutation = useDeleteAccount();
  const toggleMutation = useToggleAccount();
  const { t } = useTranslation();
  
  // Get cache age from settings (default to 1 hour)
  const maxCacheAgeHours = parseInt(localStorage.getItem('maxCacheAgeHours') || '1', 10);
  const maxCacheAgeMs = maxCacheAgeHours * 60 * 60 * 1000;
  
  // Only fetch fresh balance if account is enabled AND balance is stale/missing
  const shouldFetchBalance = account.enabled && (
    !account.quota || !account.used_quota || account.remaining == null ||
    !account.last_balance_check_at || 
    (new Date().getTime() - new Date(account.last_balance_check_at).getTime()) > maxCacheAgeMs
  );

  // Fetch fresh balance in background (with smart caching)
  const { data: freshBalance, isLoading: balanceLoading, error: balanceError } = useFetchAccountBalance(
    account.id, 
    shouldFetchBalance
  );

  // Debug logging for errors
  if (balanceError) {
    console.error(`Failed to fetch balance for ${account.name}:`, balanceError);
  }

  // Use cached balance from account or fresh balance from fetch
  const balance = freshBalance || (
    account.quota != null && account.used_quota != null && account.remaining != null ? {
      quota: account.quota,
      used: account.used_quota,
      remaining: account.remaining,
    } : null
  );

  // 从account对象获取自动签到设置
  const autoCheckinEnabled = account.auto_checkin_enabled || false;
  const autoCheckinTime = `${String(account.auto_checkin_hour || 9).padStart(2, '0')}:${String(account.auto_checkin_minute || 0).padStart(2, '0')}`;

  const handleToggle = async () => {
    try {
      await toggleMutation.mutateAsync({
        accountId: account.id,
        enabled: !account.enabled,
      });
      toast.success(
        account.enabled ? t('accountCard.disabled') : t('accountCard.enabled')
      );
    } catch (error) {
      console.error('Failed to toggle account:', error);
      toast.error(t('common.error'));
    }
  };

  const handleDelete = async () => {
    try {
      await deleteMutation.mutateAsync(account.id);
      toast.success(t('common.success'));
      setShowDeleteConfirm(false);
    } catch (error) {
      console.error('Failed to delete account:', error);
      toast.error(t('common.error'));
    }
  };

  return (
    <Card className={`relative hover:shadow-lg transition-all rounded-2xl ${!account.enabled ? 'opacity-60' : ''}`}>
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            {/* 邮箱单独一行 */}
            <h3 className="font-medium text-sm truncate mb-2" title={account.name}>
              {account.name}
            </h3>
            {/* 状态和中转站名称在第二行 */}
            <div className="flex items-center gap-2 text-xs">
              <Badge variant={account.enabled ? 'default' : 'secondary'} className="text-xs rounded-full">
                {account.enabled ? t('accountCard.active') : t('accountCard.disabled')}
              </Badge>
              <span className="text-muted-foreground">{account.provider_name}</span>
            </div>
          </div>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8 shrink-0 rounded-full">
                <MoreVertical className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="rounded-xl">
              <DropdownMenuItem onClick={() => onEdit(account)} className="rounded-lg">
                <Edit className="h-4 w-4 mr-2" />
                {t('accountCard.edit')}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={handleToggle} className="rounded-lg">
                {account.enabled ? (
                  <>
                    <PowerOff className="h-4 w-4 mr-2" />
                    {t('accountCard.disable')}
                  </>
                ) : (
                  <>
                    <Power className="h-4 w-4 mr-2" />
                    {t('accountCard.enable')}
                  </>
                )}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => setShowDeleteConfirm(true)}
                className="text-destructive focus:text-destructive rounded-lg"
              >
                <Trash2 className="h-4 w-4 mr-2" />
                {t('accountCard.delete')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </CardHeader>

      <CardContent className="space-y-3">
        {/* 余额信息 */}
        {account.enabled && (balance || balanceLoading) && (
          <div className="grid grid-cols-3 gap-2 text-xs bg-muted/30 rounded-xl p-3">
            {balance ? (
              <>
                <div className="text-center">
                  <p className="text-muted-foreground mb-0.5">{t('accountCard.totalIncome')}</p>
                  <p className="font-semibold text-blue-600">${(balance.quota + balance.used).toFixed(2)}</p>
                </div>
                <div className="text-center border-x">
                  <p className="text-muted-foreground mb-0.5">{t('accountCard.historicalConsumption')}</p>
                  <p className="font-semibold text-orange-600">${balance.used.toFixed(2)}</p>
                </div>
                <div className="text-center">
                  <p className="text-muted-foreground mb-0.5">{t('accountCard.currentBalance')}</p>
                  <p className="font-semibold text-green-600">${balance.quota.toFixed(2)}</p>
                </div>
              </>
            ) : balanceLoading ? (
              <div className="col-span-3 flex items-center justify-center py-2">
                <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
              </div>
            ) : null}
          </div>
        )}

        {/* 自动签到信息和手动签到按钮 */}
        {account.enabled && (
          <div className="flex items-center justify-between gap-2">
            {/* 左侧：自动签到信息 */}
            {autoCheckinEnabled ? (
              <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <Clock className="h-3.5 w-3.5" />
                <span>{t('accountCard.autoAt')} {autoCheckinTime}</span>
              </div>
            ) : (
              <div className="text-xs text-muted-foreground">
                {t('accountCard.autoDisabled')}
              </div>
            )}
            
            {/* 右侧：手动签到按钮（小圆形） */}
            <CheckInButton
              accountId={account.id}
              accountName={account.name}
              size="sm"
              variant="outline"
              className="rounded-full h-7 px-3 text-xs"
            />
          </div>
        )}
      </CardContent>

      {/* 删除确认对话框 */}
      {showDeleteConfirm && (
        <div className="absolute inset-0 bg-background/95 backdrop-blur-sm rounded-2xl flex items-center justify-center p-4 z-10">
          <div className="text-center space-y-3 w-full">
            <div>
              <h4 className="font-semibold text-sm">{t('accountCard.deleteConfirm')}</h4>
              <p className="text-xs text-muted-foreground mt-1">
                {t('accountCard.deleteWarning')}
              </p>
            </div>
            <div className="flex gap-2 justify-center">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowDeleteConfirm(false)}
                className="rounded-full"
              >
                {t('accountCard.cancel')}
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={handleDelete}
                disabled={deleteMutation.isPending}
                className="rounded-full"
              >
                {deleteMutation.isPending ? t('accountCard.deleting') : t('accountCard.delete')}
              </Button>
            </div>
          </div>
        </div>
      )}
    </Card>
  );
}
