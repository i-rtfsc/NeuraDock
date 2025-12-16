export interface IndependentKeyDto {
  id: number;
  name: string;
  provider_type: string;
  provider_type_display: string;
  custom_provider_name: string | null;
  masked_key: string;
  base_url: string;
  organization_id: string | null;
  description: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateIndependentKeyInput {
  name: string;
  provider_type: string;
  custom_provider_name?: string;
  api_key: string;
  base_url?: string;
  organization_id?: string;
  description?: string;
}

export interface UpdateIndependentKeyInput {
  key_id: number;
  name?: string;
  api_key?: string;
  base_url?: string;
  organization_id?: string;
  description?: string;
}
