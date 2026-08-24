import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { accountCommands } from '@/lib/tauri-commands';
import type { CreateAccountInput, UpdateAccountInput } from '@/lib/tauri-commands';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { cacheInvalidators } from '@/lib/cacheInvalidators';
import { accountKeys } from '@/lib/query-keys';

// Query: Get all accounts
export function useAccounts(enabledOnly: boolean = false) {
  return useQuery({
    queryKey: accountKeys.list(enabledOnly),
    queryFn: () => accountCommands.getAll(enabledOnly),
  });
}

// Query: Get account detail
export function useAccountDetail(accountId: string) {
  return useQuery({
    queryKey: accountKeys.detail(accountId),
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
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(t('accounts.createSuccess', '账号创建成功'));
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
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
      cacheInvalidators.invalidateAccount(queryClient, variables.account_id);
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(t('accounts.updateSuccess', '账号已更新'));
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
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
      cacheInvalidators.invalidateAccount(queryClient, accountId);
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(t('accountCard.deleted', '账号已删除'));
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
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
    onMutate: async ({ accountId, enabled }) => {
      // Cancel any outgoing refetches (so they don't overwrite our optimistic update)
      await queryClient.cancelQueries({ queryKey: accountKeys.all });

      // Snapshot the previous value
      const previousAccounts = queryClient.getQueriesData({ queryKey: accountKeys.lists() });
      const previousDetail = queryClient.getQueryData(accountKeys.detail(accountId));

      // Optimistically update to the new value
      queryClient.setQueriesData({ queryKey: accountKeys.lists() }, (old: unknown) => {
        if (!Array.isArray(old)) return old;
        return old.map((acc) =>
          acc.id === accountId ? { ...acc, enabled } : acc
        );
      });

      if (previousDetail) {
        queryClient.setQueryData(accountKeys.detail(accountId), (old) =>
          old ? { ...old, enabled } : old
        );
      }

      return { previousAccounts, previousDetail };
    },
    onError: (error: unknown, { accountId }, context) => {
      // Rollback
      if (context?.previousAccounts) {
        context.previousAccounts.forEach(([queryKey, data]) => {
          queryClient.setQueryData(queryKey, data);
        });
      }
      if (context?.previousDetail) {
        queryClient.setQueryData(accountKeys.detail(accountId), context.previousDetail);
      }

      const message = error instanceof Error ? error.message : String(error);
      toast.error(
        t('accountCard.toggleFailed', {
          defaultValue: '切换账号状态失败: {{message}}',
          message,
        })
      );
    },
    onSuccess: (_, { enabled, accountId }) => {
      // Even with optimistic updates, we should invalidate to ensure eventual consistency
      cacheInvalidators.invalidateAccount(queryClient, accountId);
      cacheInvalidators.invalidateAllAccounts(queryClient);

      toast.success(
        enabled
          ? t('accountCard.enabled', '账号已启用')
          : t('accountCard.disabled', '账号已停用')
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
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(t('accounts.importSuccess', '账号导入成功'));
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
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
    onSuccess: (result) => {
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(
        t('accounts.batchImportSuccess', {
          defaultValue: '成功导入 {{count}} 个账号',
          count: result.succeeded,
        })
      );
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(
        t('accounts.batchImportFailed', {
          defaultValue: '批量导入账号失败: {{message}}',
          message,
        })
      );
    },
  });
}
