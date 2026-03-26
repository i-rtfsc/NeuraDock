import { useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { CodexAccountDto, CodexAuthInfoDto } from '@/lib/tauri';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { codexAccountKeys } from './useCodexAccounts';

const KEYS = {
  active: ['codex-auth-active'] as const,
};

export function useActiveCodexAuth() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: KEYS.active,
    queryFn: (): Promise<CodexAuthInfoDto | null> => invoke('get_active_codex_auth'),
    refetchOnWindowFocus: true,
    staleTime: 0,
  });

  useEffect(() => {
    const activeAuth = query.data;
    if (!activeAuth?.quota || activeAuth.authMode !== 'chatgpt') {
      return;
    }

    const quotaCheckedAt = new Date().toISOString();

    qc.setQueryData<CodexAccountDto[] | undefined>(codexAccountKeys.all, (current) =>
      current?.map((account) => {
        if (!isLinkedAccount(activeAuth, account)) {
          return account;
        }

        return {
          ...account,
          planType: activeAuth.quota?.planType ?? account.planType,
          hasCredits: activeAuth.quota?.hasCredits ?? account.hasCredits,
          isUnlimited: activeAuth.quota?.isUnlimited ?? account.isUnlimited,
          creditBalance: activeAuth.quota?.creditBalance ?? account.creditBalance,
          primaryWindow: activeAuth.quota?.primaryWindow ?? account.primaryWindow,
          secondaryWindow: activeAuth.quota?.secondaryWindow ?? account.secondaryWindow,
          quotaCheckedAt,
        };
      })
    );
  }, [qc, query.data, query.dataUpdatedAt]);

  return query;
}

export function useRefreshActiveCodexAuthQuota() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (): Promise<CodexAuthInfoDto | null> => invoke('refresh_active_codex_auth_quota'),
    onSuccess: (data) => {
      qc.setQueryData(KEYS.active, data);
      toast.success(t('codex.toast.activeAuthRefreshed'));
    },
    onError: (e: any) => {
      toast.error(t('codex.toast.activeAuthRefreshFailed', { message: e?.message ?? e }));
    },
  });
}

export function useSwitchCodexAuth() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (accountId: string) => invoke('switch_codex_auth', { accountId }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: KEYS.active });
      toast.success(t('codex.toast.authSwitched'));
    },
    onError: (e: any) => {
      toast.error(t('codex.toast.authSwitchFailed', { message: e?.message ?? e }));
    },
  });
}

function isLinkedAccount(activeAuth: CodexAuthInfoDto, account: CodexAccountDto) {
  if (activeAuth.accountId && account.accountId) {
    return activeAuth.accountId === account.accountId;
  }

  return activeAuth.email === account.email;
}

export function useSetCodexApiKey() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: (key: string) => invoke('set_codex_api_key', { key }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: KEYS.active });
      toast.success(t('codex.toast.apiKeySaved'));
    },
    onError: (e: any) => {
      toast.error(t('codex.toast.apiKeySaveFailed', { message: e?.message ?? e }));
    },
  });
}

export function useLogoutCodexAuth() {
  const qc = useQueryClient();
  const { t } = useTranslation();
  return useMutation({
    mutationFn: () => invoke('logout_codex_auth'),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: KEYS.active });
      toast.success(t('codex.toast.loggedOut'));
    },
    onError: (e: any) => {
      toast.error(t('codex.toast.logoutFailed', { message: e?.message ?? e }));
    },
  });
}
