import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { accountCommands } from '@/lib/tauri-commands';
import type { CreateAccountInput, UpdateAccountInput } from '@/lib/tauri-commands';
import { toast } from 'sonner';

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

  return useMutation({
    mutationFn: (input: CreateAccountInput) => accountCommands.create(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      toast.success('Account created successfully');
    },
    onError: (error: any) => {
      toast.error(`Failed to create account: ${error.message || error}`);
    },
  });
}

// Mutation: Update account
export function useUpdateAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: UpdateAccountInput) => accountCommands.update(input),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      queryClient.invalidateQueries({ queryKey: ['account', variables.account_id] });
      toast.success('Account updated successfully');
    },
    onError: (error: any) => {
      toast.error(`Failed to update account: ${error.message || error}`);
    },
  });
}

// Mutation: Delete account
export function useDeleteAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (accountId: string) => accountCommands.delete(accountId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      toast.success('Account deleted successfully');
    },
    onError: (error: any) => {
      toast.error(`Failed to delete account: ${error.message || error}`);
    },
  });
}

// Mutation: Toggle account
export function useToggleAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ accountId, enabled }: { accountId: string; enabled: boolean }) =>
      accountCommands.toggle(accountId, enabled),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      queryClient.invalidateQueries({ queryKey: ['account', variables.accountId] });
      toast.success(`Account ${variables.enabled ? 'enabled' : 'disabled'}`);
    },
    onError: (error: any) => {
      toast.error(`Failed to toggle account: ${error.message || error}`);
    },
  });
}

// Mutation: Import from JSON
export function useImportAccountFromJson() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (jsonData: string) => accountCommands.importFromJson(jsonData),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      toast.success('Account imported successfully');
    },
    onError: (error: any) => {
      toast.error(`Failed to import account: ${error.message || error}`);
    },
  });
}

// Mutation: Import batch
export function useImportAccountsBatch() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (jsonData: string) => accountCommands.importBatch(jsonData),
    onSuccess: (accountIds) => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      toast.success(`Successfully imported ${accountIds.length} accounts`);
    },
    onError: (error: any) => {
      toast.error(`Failed to import accounts: ${error.message || error}`);
    },
  });
}
