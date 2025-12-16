import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

export interface ProviderDto {
  id: string;
  name: string;
  domain: string;
  is_builtin: boolean;
  account_count: number;
  // API configuration fields
  login_path: string;
  sign_in_path: string | null;
  user_info_path: string;
  token_api_path: string | null;
  models_path: string | null;
  api_user_key: string;
  needs_waf_bypass: boolean;
}

// Query: Get all providers
export function useProviders() {
  return useQuery({
    queryKey: ['providers'],
    queryFn: () => invoke<ProviderDto[]>('get_all_providers'),
  });
}
