import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { CodexAccountDto, CodexInboxCodeDto } from '@/lib/tauri';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export const codexAccountKeys = {
  all: ['codex-accounts'] as const,
  detail: (id: string) => ['codex-accounts', id] as const,
};

export function useCodexAccounts() {
  return useQuery({
    queryKey: codexAccountKeys.all,
    queryFn: (): Promise<CodexAccountDto[]> => invoke('list_codex_accounts'),
  });
}

export function useCodexAccount(id: string) {
  return useQuery({
    queryKey: codexAccountKeys.detail(id),
    queryFn: (): Promise<CodexAccountDto | null> => invoke('get_codex_account', { id }),
    enabled: !!id,
  });
}

export function useDeleteCodexAccount() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (id: string) => invoke('delete_codex_account', { id }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: codexAccountKeys.all });
      toast.success(t('codex.toast.accountDeleted'));
    },
    onError: (e: any) => {
      toast.error(t('codex.toast.accountDeleteFailed', { message: e?.message ?? e }));
    },
  });
}

export function useRefreshCodexAccountQuota() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (id: string): Promise<CodexAccountDto> =>
      invoke('refresh_codex_account_quota', { id }),
    onSuccess: (data: CodexAccountDto) => {
      qc.invalidateQueries({ queryKey: codexAccountKeys.all });
      qc.setQueryData(codexAccountKeys.detail(data.id), data);
      toast.success(t('codex.toast.quotaRefreshed'));
    },
    onError: (e: any) => {
      toast.error(t('codex.toast.quotaRefreshFailed', { message: e?.message ?? e }));
    },
  });
}

export function useCodexAccountInboxCode() {
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (id: string): Promise<CodexInboxCodeDto> =>
      invoke('get_codex_account_inbox_code', { id }),
    onError: (e: any) => {
      toast.error(t('codex.toast.inboxCodeFetchFailed', { message: e?.message ?? e }));
    },
  });
}
