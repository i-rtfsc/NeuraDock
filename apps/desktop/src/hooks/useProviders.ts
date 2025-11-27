import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

export interface ProviderDto {
  id: string;
  name: string;
  domain: string;
  is_builtin: boolean;
  account_count: number;
}

// Query: Get all providers
export function useProviders() {
  return useQuery({
    queryKey: ['providers'],
    queryFn: () => invoke<ProviderDto[]>('get_all_providers'),
  });
}
