import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '@/lib/tauri';
import { toast } from 'sonner';
import { ProviderDto } from './useProviders';
import { cacheInvalidators } from '@/lib/cacheInvalidators';

const unwrap = <T,>(r: { status: 'ok'; data: T } | { status: 'error'; error: { message: string } }): T => { if (r.status === 'error') throw new Error(r.error.message); return r.data; };

export function useProviderActions() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<ProviderDto | null>(null);

  const createMutation = useMutation({
    mutationFn: async (input: {
      name: string;
      domain: string;
      needs_waf_bypass: boolean;
      supports_check_in?: boolean;
      check_in_bugged?: boolean;
      check_in_mode?: string;
      check_in_day_offset_minutes?: number;
      login_path?: string;
      sign_in_path?: string;
      user_info_path?: string;
      token_api_path?: string;
      models_path?: string;
      api_user_key?: string;
    }) => {
      return unwrap(await commands.createProvider({
        ...input,
        supports_check_in: input.supports_check_in ?? null,
        check_in_bugged: input.check_in_bugged ?? null,
        // Null lets the backend classify the mode from the configuration.
        check_in_mode: input.check_in_mode ?? null,
        check_in_day_offset_minutes: input.check_in_day_offset_minutes ?? null,
        login_path: input.login_path ?? null,
        sign_in_path: input.sign_in_path ?? null,
        user_info_path: input.user_info_path ?? null,
        token_api_path: input.token_api_path ?? null,
        models_path: input.models_path ?? null,
        api_user_key: input.api_user_key ?? null,
      }));
    },
    onSuccess: () => {
      cacheInvalidators.invalidateProvider(queryClient);
      toast.success(t('providers.toast.createSuccess'));
    },
    onError: (error: unknown) => {
      toast.error(t('providers.toast.createError', {
        error: error instanceof Error ? error.message : String(error)
      }));
    },
  });

  const updateMutation = useMutation({
    mutationFn: async (input: {
      provider_id: string;
      name?: string;
      domain?: string;
      needs_waf_bypass?: boolean;
      supports_check_in?: boolean;
      check_in_bugged?: boolean;
      check_in_mode?: string;
      check_in_day_offset_minutes?: number;
      login_path?: string;
      sign_in_path?: string;
      user_info_path?: string;
      token_api_path?: string;
      models_path?: string;
      api_user_key?: string;
    }) => {
      return unwrap(await commands.updateProvider({
        ...input,
        name: input.name ?? null,
        domain: input.domain ?? null,
        needs_waf_bypass: input.needs_waf_bypass ?? null,
        supports_check_in: input.supports_check_in ?? null,
        check_in_bugged: input.check_in_bugged ?? null,
        // Null keeps the stored classification.
        check_in_mode: input.check_in_mode ?? null,
        check_in_day_offset_minutes: input.check_in_day_offset_minutes ?? null,
        login_path: input.login_path ?? null,
        sign_in_path: input.sign_in_path ?? null,
        user_info_path: input.user_info_path ?? null,
        token_api_path: input.token_api_path ?? null,
        models_path: input.models_path ?? null,
        api_user_key: input.api_user_key ?? null,
      }));
    },
    onSuccess: (_, variables) => {
      cacheInvalidators.invalidateProvider(queryClient, variables.provider_id);
      toast.success(t('providers.toast.updateSuccess'));
    },
    onError: (error: unknown) => {
      toast.error(t('providers.toast.updateError', {
        error: error instanceof Error ? error.message : String(error)
      }));
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async (providerId: string) => {
      return unwrap(await commands.deleteProvider({ provider_id: providerId }));
    },
    onSuccess: (_, providerId) => {
      cacheInvalidators.invalidateProvider(queryClient, providerId);
      toast.success(t('providers.toast.deleteSuccess'));
    },
    onError: (error: unknown) => {
      toast.error(t('providers.toast.deleteError', {
        error: error instanceof Error ? error.message : String(error)
      }));
    },
  });

  const handleCreate = () => {
    setEditingProvider(null);
    setDialogOpen(true);
  };

  const handleEdit = (provider: ProviderDto) => {
    setEditingProvider(provider);
    setDialogOpen(true);
  };

  const handleDialogClose = () => {
    setDialogOpen(false);
    setEditingProvider(null);
  };

  const handleDelete = async (providerId: string) => {
    await deleteMutation.mutateAsync(providerId);
  };

  return {
    dialogOpen,
    editingProvider,
    handleCreate,
    handleEdit,
    handleDialogClose,
    handleDelete,
    createMutation,
    updateMutation,
    deleteMutation,
  };
}
