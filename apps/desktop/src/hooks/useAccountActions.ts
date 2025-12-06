import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { Account, AccountDetail } from '@/lib/tauri-commands';

export function useAccountActions() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [editingAccount, setEditingAccount] = useState<AccountDetail | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  // Fetch account detail
  const fetchAccountDetail = async (accountId: string): Promise<AccountDetail> => {
    return await invoke<AccountDetail>('get_account_detail', { accountId });
  };

  // Edit account
  const handleEdit = async (account: Account) => {
    try {
      const accountDetail = await fetchAccountDetail(account.id);
      setEditingAccount(accountDetail);
      setDialogOpen(true);
    } catch (error) {
      console.error('Failed to fetch account details:', error);
      toast.error(t('common.error'));
    }
  };

  // Create new account
  const handleCreate = () => {
    setEditingAccount(null);
    setDialogOpen(true);
  };

  // Close dialog
  const handleDialogClose = () => {
    setDialogOpen(false);
    setEditingAccount(null);
  };

  // Delete account mutation
  const deleteAccountMutation = useMutation({
    mutationFn: async (accountId: string) => {
      await invoke('delete_account', { accountId });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      toast.success(t('accounts.deleteSuccess') || 'Account deleted successfully');
    },
    onError: (error) => {
      console.error('Failed to delete account:', error);
      toast.error(t('common.error'));
    },
  });

  // Toggle account enabled status
  const toggleAccountMutation = useMutation({
    mutationFn: async ({ accountId, enabled }: { accountId: string; enabled: boolean }) => {
      await invoke('update_account_status', { accountId, enabled });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
    },
    onError: (error) => {
      console.error('Failed to toggle account:', error);
      toast.error(t('common.error'));
    },
  });

  // Batch enable accounts
  const batchEnableMutation = useMutation({
    mutationFn: async (accountIds: string[]) => {
      await Promise.all(
        accountIds.map(id => invoke('update_account_status', { accountId: id, enabled: true }))
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      toast.success(t('accounts.batchEnableSuccess') || 'Accounts enabled successfully');
    },
    onError: (error) => {
      console.error('Failed to enable accounts:', error);
      toast.error(t('common.error'));
    },
  });

  // Batch disable accounts
  const batchDisableMutation = useMutation({
    mutationFn: async (accountIds: string[]) => {
      await Promise.all(
        accountIds.map(id => invoke('update_account_status', { accountId: id, enabled: false }))
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      toast.success(t('accounts.batchDisableSuccess') || 'Accounts disabled successfully');
    },
    onError: (error) => {
      console.error('Failed to disable accounts:', error);
      toast.error(t('common.error'));
    },
  });

  return {
    // State
    editingAccount,
    dialogOpen,
    
    // Actions
    handleEdit,
    handleCreate,
    handleDialogClose,
    deleteAccount: deleteAccountMutation.mutate,
    toggleAccount: toggleAccountMutation.mutate,
    batchEnable: batchEnableMutation.mutate,
    batchDisable: batchDisableMutation.mutate,
    
    // Loading states
    isDeleting: deleteAccountMutation.isPending,
    isToggling: toggleAccountMutation.isPending,
    isBatchEnabling: batchEnableMutation.isPending,
    isBatchDisabling: batchDisableMutation.isPending,
  };
}
