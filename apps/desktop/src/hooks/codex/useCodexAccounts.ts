import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '@/lib/tauri';
const unwrap = <T,>(r: { status: 'ok'; data: T } | { status: 'error'; error: { message: string } }): T => { if (r.status === 'error') throw new Error(r.error.message); return r.data; };
import type { CodexAccountDto } from '@/lib/tauri';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export const codexAccountKeys = {
  all: ['codex-accounts'] as const,
  detail: (id: string) => ['codex-accounts', id] as const,
};

export function useCodexAccounts() {
  return useQuery({
    queryKey: codexAccountKeys.all,
    queryFn: () => commands.listCodexAccounts().then(unwrap),
  });
}

export function useCodexAccount(id: string) {
  return useQuery({
    queryKey: codexAccountKeys.detail(id),
    queryFn: () => commands.getCodexAccount(id).then(unwrap),
    enabled: !!id,
  });
}

export function useDeleteCodexAccount() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (id: string) => commands.deleteCodexAccount(id).then(unwrap),
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
    mutationFn: (id: string) => commands.refreshCodexAccountQuota(id).then(unwrap),
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
    mutationFn: (id: string) => commands.getCodexAccountInboxCode(id).then(unwrap),
    onError: (e: any) => {
      toast.error(t('codex.toast.inboxCodeFetchFailed', { message: e?.message ?? e }));
    },
  });
}
