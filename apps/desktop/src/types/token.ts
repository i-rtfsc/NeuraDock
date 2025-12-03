export interface TokenDto {
  id: number;
  account_id: string;
  account_name: string;
  provider_name: string;
  name: string;
  key: string;
  masked_key: string;
  status: number;
  status_text: string;
  used_quota: number;
  remain_quota: number;
  unlimited_quota: boolean;
  usage_percentage: number;
  expired_time: number | null;
  expired_at: string | null;
  is_active: boolean;
  is_expired: boolean;
  model_limits_allowed: string[];
  model_limits_denied: string[];
  fetched_at: string;
}

export interface AccountDto {
  id: string;
  name: string;
  provider_id: string;
  provider_name: string;
  enabled: boolean;
}

export interface ProviderNode {
  id: string;
  name: string;
  base_url: string;
}
