import { useQuery, useMutation, useQueryClient, QueryClient } from '@tanstack/react-query';
import { accountCommands } from '@/lib/tauri-commands';
import type { CreateAccountInput, UpdateAccountInput } from '@/lib/tauri-commands';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

export function invalidateAccountCaches(queryClient: QueryClient) {
  queryClient.invalidateQueries({ queryKey: ['accounts'], exact: false });
  queryClient.refetchQueries({ queryKey: ['accounts'], type: 'active' });
  queryClient.invalidateQueries({ queryKey: ['balance-statistics'], exact: false });
  queryClient.refetchQueries({ queryKey: ['balance-statistics'], type: 'active' });
}

// Query: Get all accounts
export function useAccounts(enabledOnly: boolean = false) {
  return useQuery({
    queryKey: ['accounts', enabledOnly],
    queryFn: () => accountCommands.getAll(enabledOnly),
  });
}

// Query: Get account detail
export function useAccountDetail(accountId: string) {
  return useQuery({
    queryKey: ['account', accountId],
    queryFn: () => accountCommands.getDetail(accountId),
    enabled: !!accountId,
  });
}

// Mutation: Create account
export function useCreateAccount() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (input: CreateAccountInput) => accountCommands.create(input),
    onSuccess: () => {
      invalidateAccountCaches(queryClient);
      toast.success(t('accounts.createSuccess', '账号创建成功'));
    },
    onError: (error: any) => {
      const message = error?.message || String(error);
      toast.error(
        t('accounts.createFailed', {
          defaultValue: '创建账号失败: {{message}}',
          message,
        })
      );
    },
  });
}

// Mutation: Update account
export function useUpdateAccount() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (input: UpdateAccountInput) => accountCommands.update(input),
    onSuccess: (_, variables) => {
      invalidateAccountCaches(queryClient);
      queryClient.invalidateQueries({ queryKey: ['account', variables.account_id] });
      toast.success(t('accounts.updateSuccess', '账号已更新'));
    },
    onError: (error: any) => {
      const message = error?.message || String(error);
      toast.error(
        t('accounts.updateFailed', {
          defaultValue: '更新账号失败: {{message}}',
          message,
        })
      );
    },
  });
}

// Mutation: Delete account
export function useDeleteAccount() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (accountId: string) => accountCommands.delete(accountId),
    onSuccess: (_, accountId) => {
      invalidateAccountCaches(queryClient);
      queryClient.invalidateQueries({ queryKey: ['account', accountId] });
      toast.success(t('accountCard.deleted', '账号已删除'));
    },
    onError: (error: any) => {
      const message = error?.message || String(error);
      toast.error(
        t('accountCard.deleteFailed', {
          defaultValue: '删除账号失败: {{message}}',
          message,
        })
      );
    },
  });
}

// Mutation: Toggle account
export function useToggleAccount() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({ accountId, enabled }: { accountId: string; enabled: boolean }) =>
      accountCommands.toggle(accountId, enabled),
    onSuccess: (_, variables) => {
      invalidateAccountCaches(queryClient);
      queryClient.invalidateQueries({ queryKey: ['account', variables.accountId] });
      toast.success(
        variables.enabled
          ? t('accountCard.enabled', '账号已启用')
          : t('accountCard.disabled', '账号已停用')
      );
    },
    onError: (error: any) => {
      const message = error?.message || String(error);
      toast.error(
        t('accountCard.toggleFailed', {
          defaultValue: '切换账号状态失败: {{message}}',
          message,
        })
      );
    },
  });
}

// Mutation: Import from JSON
export function useImportAccountFromJson() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (jsonData: string) => accountCommands.importFromJson(jsonData),
    onSuccess: () => {
      invalidateAccountCaches(queryClient);
      toast.success(t('accounts.importSuccess', '账号导入成功'));
    },
    onError: (error: any) => {
      const message = error?.message || String(error);
      toast.error(
        t('accounts.importFailed', {
          defaultValue: '导入账号失败: {{message}}',
          message,
        })
      );
    },
  });
}

// Mutation: Import batch
export function useImportAccountsBatch() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (jsonData: string) => accountCommands.importBatch(jsonData),
    onSuccess: (accountIds) => {
      invalidateAccountCaches(queryClient);
      toast.success(
        t('accounts.batchImportSuccess', {
          defaultValue: '成功导入 {{count}} 个账号',
          count: accountIds.length,
        })
      );
    },
    onError: (error: any) => {
      const message = error?.message || String(error);
      toast.error(
        t('accounts.batchImportFailed', {
          defaultValue: '批量导入账号失败: {{message}}',
          message,
        })
      );
    },
  });
}
