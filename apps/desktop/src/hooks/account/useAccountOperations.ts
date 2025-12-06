import { useDeleteAccount, useToggleAccount } from '@/hooks/useAccounts';
import { useRefreshAccountBalance } from '@/hooks/useBalance';
import { Account } from '@/lib/tauri-commands';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

/**
 * Hook to encapsulate common account actions (delete, toggle, refresh balance).
 * Keeps the UI component clean from mutation logic.
 */
export function useAccountOperations(account: Account) {
  const { t } = useTranslation();
  const deleteMutation = useDeleteAccount();
  const toggleMutation = useToggleAccount();
  const refreshBalanceMutation = useRefreshAccountBalance();

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

  const handleRefreshBalance = async () => {
    try {
      await refreshBalanceMutation.mutateAsync(account.id);
      toast.success(t('accountCard.balanceRefreshed') || 'Balance refreshed');
    } catch (error) {
      console.error('Failed to refresh balance:', error);
      const errorMessage = error instanceof Error ? error.message : String(error);
      toast.error(errorMessage || t('common.error'));
    }
  };

  const handleDelete = async () => {
    try {
      await deleteMutation.mutateAsync(account.id);
      toast.success(t('common.success'));
      return true;
    } catch (error) {
      console.error('Failed to delete account:', error);
      toast.error(t('common.error'));
      return false;
    }
  };

  return {
    handleToggle,
    handleRefreshBalance,
    handleDelete,
    isDeleting: deleteMutation.isPending,
    isToggling: toggleMutation.isPending,
    isRefreshingBalance: refreshBalanceMutation.isPending,
  };
}
