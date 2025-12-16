import { useState } from 'react';
import { useProviders } from '@/hooks/useProviders';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { invoke } from '@tauri-apps/api/core';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { RefreshCw, AlertCircle, CheckCircle2, Loader2, Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface BatchUpdateResult {
  total: number;
  updated: number;
  created: number;
  failed: number;
  results: Array<{
    success: boolean;
    account_id: string | null;
    account_name: string;
    action: string;
    error: string | null;
  }>;
}

interface BatchUpdateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function BatchUpdateDialog({ open, onOpenChange }: BatchUpdateDialogProps) {
  const { t } = useTranslation();
  const [jsonInput, setJsonInput] = useState('');
  const [createIfNotExists, setCreateIfNotExists] = useState(false);
  const [validationResult, setValidationResult] = useState<{
    valid: boolean;
    accounts?: Array<{
      name: string;
      provider: string;
      hasCookies: boolean;
    }>;
    error?: string;
  } | null>(null);

  const { data: providers = [] } = useProviders();
  const fallbackProviderId = providers[0]?.id || '';
  const providerPlaceholder = fallbackProviderId || 'provider_id';

  const queryClient = useQueryClient();

  const updateMutation = useMutation({
    mutationFn: async (params: { jsonData: string; createIfNotExists: boolean }) => {
      return invoke<BatchUpdateResult>('update_accounts_batch', {
        jsonData: params.jsonData,
        createIfNotExists: params.createIfNotExists,
      });
    },
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });

      if (result.failed === 0) {
        toast.success(
          t('batchUpdate.successMessage', {
            updated: result.updated,
            created: result.created
          })
        );
      } else {
        toast.warning(
          t('batchUpdate.partialSuccess', {
            updated: result.updated,
            created: result.created,
            failed: result.failed,
          })
        );
      }

      onOpenChange(false);
      setJsonInput('');
      setValidationResult(null);
    },
    onError: (error) => {
      console.error('Batch update failed:', error);
      toast.error(t('batchUpdate.error'));
    },
  });

  const validateJson = () => {
    if (!jsonInput.trim()) {
      setValidationResult({
        valid: false,
        error: t('batchUpdate.emptyInput'),
      });
      return;
    }

    try {
      const parsed = JSON.parse(jsonInput);

      // Must be an array
      if (!Array.isArray(parsed)) {
        setValidationResult({
          valid: false,
          error: t('batchUpdate.mustBeArray'),
        });
        return;
      }

      const accounts = parsed.map((account: any, index: number) => ({
        name: account.name || `Account ${index + 1}`,
        provider: account.provider || account.provider_id || providerPlaceholder,
        hasCookies: !!account.cookies && typeof account.cookies === 'object' && Object.keys(account.cookies).length > 0,
      }));

      const allHaveCookies = accounts.every((a) => a.hasCookies);

      setValidationResult({
        valid: allHaveCookies,
        accounts,
        error: allHaveCookies ? undefined : t('batchUpdate.missingCookies'),
      });
    } catch (error) {
      setValidationResult({
        valid: false,
        error: error instanceof Error ? error.message : t('batchUpdate.invalidJson'),
      });
    }
  };

  const handleUpdate = async () => {
    if (!validationResult?.valid) {
      toast.error(t('batchUpdate.validateFirst'));
      return;
    }

    await updateMutation.mutateAsync({
      jsonData: jsonInput,
      createIfNotExists,
    });
  };

  const formatJson = () => {
    try {
      const parsed = JSON.parse(jsonInput);
      const formatted = JSON.stringify(parsed, null, 2);
      setJsonInput(formatted);
    } catch (error) {
      toast.error(t('batchUpdate.cannotFormat'));
    }
  };

  const exampleJson = [
    {
      name: 'account1@example.com',
      provider: providerPlaceholder,
      cookies: {
        session: 'new_session_value_1',
      },
      api_user: '12345',
    },
    {
      name: 'account2@example.com',
      provider: providerPlaceholder,
      cookies: {
        session: 'new_session_value_2',
      },
      api_user: '67890',
    },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <RefreshCw className="h-5 w-5" />
            {t('batchUpdate.title')}
          </DialogTitle>
          <DialogDescription>
            {t('batchUpdate.description')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* JSON Input */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="json-input">{t('batchUpdate.jsonData')}</Label>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={formatJson}
                  disabled={updateMutation.isPending}
                  className="rounded-full"
                >
                  {t('batchUpdate.format')}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={validateJson}
                  disabled={updateMutation.isPending}
                  className="rounded-full"
                >
                  {t('batchUpdate.validate')}
                </Button>
              </div>
            </div>
            <textarea
              id="json-input"
              rows={12}
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 font-mono"
              placeholder={t('batchUpdate.placeholder')}
              value={jsonInput}
              onChange={(e) => {
                setJsonInput(e.target.value);
                setValidationResult(null);
              }}
              disabled={updateMutation.isPending}
            />
          </div>

          {/* Create if not exists option */}
          <div className="flex items-center justify-between rounded-lg border border-border p-4">
            <div className="space-y-0.5">
              <Label htmlFor="create-if-not-exists" className="flex items-center gap-2">
                <Plus className="h-4 w-4" />
                {t('batchUpdate.createIfNotExists')}
              </Label>
              <p className="text-xs text-muted-foreground">
                {t('batchUpdate.createIfNotExistsDescription')}
              </p>
            </div>
            <Switch
              id="create-if-not-exists"
              checked={createIfNotExists}
              onCheckedChange={setCreateIfNotExists}
              disabled={updateMutation.isPending}
            />
          </div>

          {/* Validation Result */}
          {validationResult && (
            <div
              className={`rounded-lg border p-4 space-y-3 ${
                validationResult.valid
                  ? 'border-green-500/50 bg-green-500/10'
                  : 'border-red-500/50 bg-red-500/10'
              }`}
            >
              <div className="flex items-center gap-2">
                {validationResult.valid ? (
                  <CheckCircle2 className="h-5 w-5 text-green-500" />
                ) : (
                  <AlertCircle className="h-5 w-5 text-red-500" />
                )}
                <span className="font-medium">
                  {validationResult.valid ? t('batchUpdate.validJson') : t('batchUpdate.invalidJson')}
                </span>
                {validationResult.accounts && (
                  <Badge variant="secondary">
                    {validationResult.accounts.length} {t('batchUpdate.accounts')}
                  </Badge>
                )}
              </div>

              {validationResult.error && (
                <p className="text-sm text-red-500">{validationResult.error}</p>
              )}

              {validationResult.accounts && validationResult.accounts.length > 0 && (
                <div className="space-y-2">
                  <p className="text-sm font-medium">{t('batchUpdate.preview')}</p>
                  <div className="space-y-1 max-h-40 overflow-y-auto">
                    {validationResult.accounts.map((account, index) => (
                      <div
                        key={index}
                        className="flex items-center gap-2 text-sm p-2 rounded bg-background/50"
                      >
                        {account.hasCookies ? (
                          <CheckCircle2 className="h-4 w-4 text-green-500" />
                        ) : (
                          <AlertCircle className="h-4 w-4 text-red-500" />
                        )}
                        <span className="flex-1 truncate">{account.name}</span>
                        <Badge variant="outline" className="text-xs">
                          {account.provider}
                        </Badge>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Example */}
          <div className="rounded-lg border border-border bg-muted/50 p-4 space-y-3">
            <h4 className="text-sm font-medium">{t('batchUpdate.example')}</h4>
            <p className="text-xs text-muted-foreground">
              {t('batchUpdate.exampleDescription')}
            </p>
            <pre className="text-xs bg-background p-2 rounded overflow-x-auto">
              {JSON.stringify(exampleJson, null, 2)}
            </pre>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => onOpenChange(false)}
              disabled={updateMutation.isPending}
              className="rounded-full"
            >
              {t('batchUpdate.cancel')}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleUpdate}
              disabled={!validationResult?.valid || updateMutation.isPending}
              className="rounded-full"
            >
              {updateMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              <RefreshCw className="mr-2 h-4 w-4" />
              {t('batchUpdate.update')}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
