import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Account } from '@/lib/tauri-commands';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { CheckInButton } from '@/components/checkin/CheckInButton';
import { AccountBalanceDisplay } from '@/components/account/AccountBalanceDisplay';
import { useSmartAccountBalance } from '@/hooks/account/useSmartAccountBalance';
import { useAccountOperations } from '@/hooks/account/useAccountOperations';
import {
  MoreVertical,
  Edit,
  Trash2,
  Power,
  PowerOff,
  Clock,
  ChevronRight,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { cn } from '@/lib/utils';

interface AccountCardProps {
  account: Account;
  onEdit: (account: Account) => void;
}

export function AccountCard({ account, onEdit }: AccountCardProps) {
  const navigate = useNavigate();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const { t } = useTranslation();

  // Use custom hooks for logic encapsulation
  const { 
    balance, 
    isLoading: isBalanceLoading, 
    isFetching: isBalanceFetching 
  } = useSmartAccountBalance(account);
  
  const { 
    handleToggle, 
    handleRefreshBalance, 
    handleDelete, 
    isDeleting, 
    isRefreshingBalance 
  } = useAccountOperations(account);

  const confirmDelete = async () => {
    await handleDelete();
    setShowDeleteConfirm(false);
  };

  // 从account对象获取自动签到设置
  const autoCheckinEnabled = account.auto_checkin_enabled || false;
  const autoCheckinTime = `${String(account.auto_checkin_hour || 9).padStart(2, '0')}:${String(account.auto_checkin_minute || 0).padStart(2, '0')}`;

  const handleCardClick = (e: React.MouseEvent) => {
    // 忽略按钮点击事件
    if ((e.target as HTMLElement).closest('button, a')) {
      return;
    }
    // 跳转到统计详情页
    navigate(`/account/${account.id}/records`);
  };

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 15 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration: 0.2, ease: "easeOut" }}
      whileHover={{ y: -2 }}
      whileTap={{ scale: 0.98 }}
      className="h-full group"
    >
      <Card 
        className={`relative h-full transition-all duration-slow ease-smooth rounded-xl border-border/40 bg-card/50 backdrop-blur-sm cursor-pointer ${!account.enabled ? 'opacity-60 grayscale-[0.5]' : ''} hover:shadow-md hover:border-border/80 hover:bg-card/80`}
        onClick={handleCardClick}
      >
        <div className="p-4 flex flex-col h-full gap-4">
          {/* Header: Name & Status */}
          <div className="flex items-start justify-between gap-2">
            <div className="flex-1 min-w-0 space-y-1">
              <div className="flex items-center gap-1.5">
                <h3 className="font-semibold text-sm truncate" title={account.name}>
                  {account.name}
                </h3>
                <ChevronRight className="h-3.5 w-3.5 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity duration-base ease-smooth" />
              </div>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span className={cn("flex h-2 w-2 rounded-full", account.enabled ? "bg-green-500" : "bg-muted-foreground")} />
                <span className="truncate opacity-80">{account.provider_name}</span>
              </div>
            </div>

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon-sm" className="shrink-0 rounded-full opacity-0 group-hover:opacity-100 transition-opacity duration-base ease-smooth -mr-2 -mt-2">
                  <MoreVertical className="h-3.5 w-3.5" />
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

          {/* Balance Section */}
          <div className="flex-1">
            {account.enabled ? (
              <AccountBalanceDisplay
                balance={balance}
                isLoading={isBalanceLoading}
                isRefreshing={isRefreshingBalance || isBalanceFetching}
                onRefresh={handleRefreshBalance}
              />
            ) : (
              <div className="h-12 flex items-center text-muted-foreground text-sm italic">
                {t('accountCard.disabled')}
              </div>
            )}
          </div>

          {/* Footer: Auto Checkin & Manual Action */}
          {account.enabled && (
            <div className="flex items-center justify-between pt-3 mt-1">
              <button 
                onClick={() => onEdit(account)}
                className="flex items-center gap-1.5 text-[10px] text-muted-foreground hover:text-foreground bg-muted/30 hover:bg-muted/60 px-2.5 py-1 rounded-full transition-colors"
                title={t('accountCard.configureAutoCheckin')}
              >
                <Clock className="h-3 w-3" />
                {autoCheckinEnabled ? (
                  <span>{autoCheckinTime}</span>
                ) : (
                  <span className="opacity-70">{t('accountCard.autoDisabled')}</span>
                )}
              </button>

              <CheckInButton
                accountId={account.id}
                accountName={account.name}
                size="sm"
                variant="ghost"
                className="text-xs hover:bg-primary/10 hover:text-primary"
              />
            </div>
          )}
        </div>

        {/* Delete Confirm Overlay */}
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
                  onClick={confirmDelete}
                  disabled={isDeleting}
                  className="rounded-full"
                >
                  {isDeleting ? t('accountCard.deleting') : t('accountCard.delete')}
                </Button>
              </div>
            </div>
          </div>
        )}
      </Card>
    </motion.div>
  );
}
