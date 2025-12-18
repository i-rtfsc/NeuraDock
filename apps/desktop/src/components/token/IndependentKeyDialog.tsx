import { useState, useEffect } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import type { IndependentKeyDto, CreateIndependentKeyInput } from '@/types/independentKey';

interface IndependentKeyDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  keyToEdit?: IndependentKeyDto | null;
}

const PROVIDER_TYPES = [
  { value: 'openai', label: 'OpenAI', defaultUrl: 'https://api.openai.com/v1' },
  { value: 'anthropic', label: 'Anthropic (Claude)', defaultUrl: 'https://api.anthropic.com/v1' },
  { value: 'custom', label: 'Custom Provider', defaultUrl: '' },
];

export function IndependentKeyDialog({ open, onOpenChange, keyToEdit }: IndependentKeyDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [name, setName] = useState('');
  const [providerType, setProviderType] = useState('openai');
  const [customProviderName, setCustomProviderName] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [baseUrl, setBaseUrl] = useState('https://api.openai.com/v1');
  const [organizationId, setOrganizationId] = useState('');
  const [description, setDescription] = useState('');

  // Reset form when dialog opens/closes or keyToEdit changes
  useEffect(() => {
    if (open) {
      if (keyToEdit) {
        // Edit mode
        setName(keyToEdit.name);
        setProviderType(keyToEdit.provider_type);
        setCustomProviderName(keyToEdit.custom_provider_name || '');
        setApiKey(''); // Don't prefill API key for security
        setBaseUrl(keyToEdit.base_url);
        setOrganizationId(keyToEdit.organization_id || '');
        setDescription(keyToEdit.description || '');
      } else {
        // Add mode
        setName('');
        setProviderType('openai');
        setCustomProviderName('');
        setApiKey('');
        setBaseUrl('https://api.openai.com/v1');
        setOrganizationId('');
        setDescription('');
      }
    }
  }, [open, keyToEdit]);

  // Update base URL when provider type changes
  const handleProviderTypeChange = (value: string) => {
    setProviderType(value);
    const provider = PROVIDER_TYPES.find(p => p.value === value);
    if (provider && provider.defaultUrl) {
      setBaseUrl(provider.defaultUrl);
    }
  };

  const createMutation = useMutation({
    mutationFn: (input: CreateIndependentKeyInput) => invoke('create_independent_key', { input }),
    onSuccess: async () => {
      console.log('[IndependentKeyDialog] Create success, invalidating queries...');
      toast.success(t('independentKey.created', 'API Key created successfully'));

      // Invalidate and refetch to ensure fresh data
      await queryClient.invalidateQueries({ queryKey: ['independent-keys'] });
      console.log('[IndependentKeyDialog] Queries invalidated, refetching...');
      await queryClient.refetchQueries({ queryKey: ['independent-keys'] });
      console.log('[IndependentKeyDialog] Refetch complete');

      // Close dialog after data is refreshed
      onOpenChange(false);
    },
    onError: (error: Error) => {
      console.error('[IndependentKeyDialog] Create error:', error);
      toast.error(t('independentKey.createError', 'Failed to create API Key: ') + error.message);
    },
  });

  const updateMutation = useMutation({
    mutationFn: (input: any) => invoke('update_independent_key', { input }),
    onSuccess: async () => {
      toast.success(t('independentKey.updated', 'API Key updated successfully'));

      // Invalidate and refetch to ensure fresh data
      await queryClient.invalidateQueries({ queryKey: ['independent-keys'] });
      await queryClient.refetchQueries({ queryKey: ['independent-keys'] });

      // Close dialog after data is refreshed
      onOpenChange(false);
    },
    onError: (error: Error) => {
      toast.error(t('independentKey.updateError', 'Failed to update API Key: ') + error.message);
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    // Validation
    if (!name.trim()) {
      toast.error(t('independentKey.nameRequired', 'Name is required'));
      return;
    }

    if (!apiKey.trim() && !keyToEdit) {
      toast.error(t('independentKey.apiKeyRequired', 'API Key is required'));
      return;
    }

    if (providerType === 'custom' && !customProviderName.trim()) {
      toast.error(t('independentKey.customProviderNameRequired', 'Custom provider name is required'));
      return;
    }

    if (!baseUrl.trim()) {
      toast.error(t('independentKey.baseUrlRequired', 'Base URL is required'));
      return;
    }

    if (keyToEdit) {
      // Update existing key
      const updateInput: any = {
        key_id: keyToEdit.id,
        name: name.trim(),
        base_url: baseUrl.trim(),
        organization_id: organizationId.trim() || null,
        description: description.trim() || null,
      };

      // Only include API key if user entered a new one
      if (apiKey.trim()) {
        updateInput.api_key = apiKey.trim();
      }

      updateMutation.mutate(updateInput);
    } else {
      // Create new key
      const createInput: CreateIndependentKeyInput = {
        name: name.trim(),
        provider_type: providerType,
        api_key: apiKey.trim(),
        base_url: baseUrl.trim(),
      };

      if (providerType === 'custom' && customProviderName.trim()) {
        createInput.custom_provider_name = customProviderName.trim();
      }

      if (organizationId.trim()) {
        createInput.organization_id = organizationId.trim();
      }

      if (description.trim()) {
        createInput.description = description.trim();
      }

      createMutation.mutate(createInput);
    }
  };

  const isLoading = createMutation.isPending || updateMutation.isPending;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {keyToEdit
              ? t('independentKey.editTitle', 'Edit API Key')
              : t('independentKey.addTitle', 'Add API Key')}
          </DialogTitle>
          <DialogDescription>
            {keyToEdit
              ? t('independentKey.editDescription', 'Update your API key configuration.')
              : t('independentKey.addDescription', 'Add a new API key from OpenAI, Anthropic, or a custom provider.')}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Name */}
          <div className="space-y-2">
            <Label htmlFor="name">{t('independentKey.name', 'Name')} *</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t('independentKey.namePlaceholder', 'e.g., My OpenAI API Key')}
              required
            />
          </div>

          {/* Provider Type (only show when creating) */}
          {!keyToEdit && (
            <div className="space-y-2">
              <Label htmlFor="provider-type">{t('independentKey.providerType', 'Provider Type')} *</Label>
              <Select value={providerType} onValueChange={handleProviderTypeChange}>
                <SelectTrigger id="provider-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {PROVIDER_TYPES.map((provider) => (
                    <SelectItem key={provider.value} value={provider.value}>
                      {provider.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {/* Custom Provider Name (only for custom type) */}
          {providerType === 'custom' && !keyToEdit && (
            <div className="space-y-2">
              <Label htmlFor="custom-provider-name">
                {t('independentKey.customProviderName', 'Custom Provider Name')} *
              </Label>
              <Input
                id="custom-provider-name"
                value={customProviderName}
                onChange={(e) => setCustomProviderName(e.target.value)}
                placeholder={t('independentKey.customProviderNamePlaceholder', 'e.g., My Custom API')}
                required={providerType === 'custom'}
              />
            </div>
          )}

          {/* API Key */}
          <div className="space-y-2">
            <Label htmlFor="api-key">
              {t('independentKey.apiKey', 'API Key')} {keyToEdit ? '' : '*'}
            </Label>
            <Input
              id="api-key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={
                keyToEdit
                  ? t('independentKey.apiKeyPlaceholderEdit', 'Leave empty to keep current key')
                  : t('independentKey.apiKeyPlaceholder', 'sk-...')
              }
              required={!keyToEdit}
            />
            {keyToEdit && (
              <p className="text-xs text-muted-foreground">
                {t('independentKey.apiKeyEditHint', 'Current: ')} {keyToEdit.masked_key}
              </p>
            )}
          </div>

          {/* Base URL */}
          <div className="space-y-2">
            <Label htmlFor="base-url">{t('independentKey.baseUrl', 'Base URL')} *</Label>
            <Input
              id="base-url"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="https://api.example.com/v1"
              required
            />
          </div>

          {/* Organization ID (optional, for OpenAI) */}
          {providerType === 'openai' && (
            <div className="space-y-2">
              <Label htmlFor="organization-id">
                {t('independentKey.organizationId', 'Organization ID')} ({t('common.optional', 'Optional')})
              </Label>
              <Input
                id="organization-id"
                value={organizationId}
                onChange={(e) => setOrganizationId(e.target.value)}
                placeholder="org-..."
              />
              <p className="text-xs text-muted-foreground">
                {t('independentKey.organizationIdHint', 'Only needed if you are using OpenAI organization accounts')}
              </p>
            </div>
          )}

          {/* Description */}
          <div className="space-y-2">
            <Label htmlFor="description">
              {t('independentKey.description', 'Description')} ({t('common.optional', 'Optional')})
            </Label>
            <Textarea
              id="description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t('independentKey.descriptionPlaceholder', 'Add notes about this API key...')}
              rows={3}
            />
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isLoading}
            >
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading
                ? t('common.saving', 'Saving...')
                : keyToEdit
                ? t('common.update', 'Update')
                : t('common.create', 'Create')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
