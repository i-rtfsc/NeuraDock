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
import { accountCommands } from '@/lib/tauri-commands';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { cacheInvalidators } from '@/lib/cacheInvalidators';
import { toast } from 'sonner';
import { Upload, FileJson, AlertCircle, CheckCircle2, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import * as z from 'zod';

// Validation schemas for imported account data
const cookiesSchema = z.union([
  z.record(z.string(), z.string()), // Object format
  z.string().refine((val) => { // String format (must be valid JSON object)
    try {
      const parsed = JSON.parse(val);
      return typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed);
    } catch {
      return false;
    }
  }, { message: 'Cookies must be a valid JSON object string' })
]);

const importedAccountSchema = z.object({
  name: z.string().min(1).max(100).optional(),
  provider: z.string().optional(),
  provider_id: z.string().optional(),
  cookies: cookiesSchema,
  api_user: z.string().optional(),
});

const singleImportSchema = importedAccountSchema;
const batchImportSchema = z.array(importedAccountSchema).min(1).max(100);

interface JsonImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function JsonImportDialog({ open, onOpenChange }: JsonImportDialogProps) {
  const { t } = useTranslation();
  const [jsonInput, setJsonInput] = useState('');
  const [validationResult, setValidationResult] = useState<{
    valid: boolean;
    accounts?: Array<{
      name?: string;
      provider?: string;
      valid: boolean;
    }>;
    error?: string;
  } | null>(null);
  const [importMode, setImportMode] = useState<'single' | 'batch'>('single');

  const { data: providers = [] } = useProviders();
  const fallbackProviderId = providers[0]?.id || '';
  const providerPlaceholder = fallbackProviderId || 'provider_id';

  const queryClient = useQueryClient();

  const importSingleMutation = useMutation({
    mutationFn: accountCommands.importFromJson,
    onSuccess: () => {
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(t('jsonImport.importSuccess'));
      onOpenChange(false);
      setJsonInput('');
      setValidationResult(null);
    },
    onError: (error) => {
      console.error('Import failed:', error);
      toast.error(t('jsonImport.importError'));
    },
  });

  const importBatchMutation = useMutation({
    mutationFn: accountCommands.importBatch,
    onSuccess: (result) => {
      cacheInvalidators.invalidateAllAccounts(queryClient);
      toast.success(t('jsonImport.importBatchSuccess', { count: result.succeeded }));
      onOpenChange(false);
      setJsonInput('');
      setValidationResult(null);
    },
    onError: (error) => {
      console.error('Batch import failed:', error);
      toast.error(t('jsonImport.importBatchError'));
    },
  });

  const isSubmitting = importSingleMutation.isPending || importBatchMutation.isPending;

  const validateJson = () => {
    if (!jsonInput.trim()) {
      setValidationResult({
        valid: false,
        error: t('jsonImport.emptyInput'),
      });
      return;
    }

    try {
      const parsed = JSON.parse(jsonInput);

      // Try batch import schema first
      const batchResult = batchImportSchema.safeParse(parsed);
      if (batchResult.success) {
        const accounts = batchResult.data.map((item, index) => ({
          name: item.name || `Account ${index + 1}`,
          provider: item.provider || item.provider_id || providerPlaceholder,
          valid: true,
        }));

        setValidationResult({
          valid: true,
          accounts,
        });
        setImportMode('batch');
        return;
      }

      // Try single import schema
      const singleResult = singleImportSchema.safeParse(parsed);
      if (singleResult.success) {
        setValidationResult({
          valid: true,
          accounts: [
            {
              name: singleResult.data.name || 'Unnamed Account',
              provider: singleResult.data.provider || singleResult.data.provider_id || providerPlaceholder,
              valid: true,
            },
          ],
        });
        setImportMode('single');
        return;
      }

      // Both schemas failed, provide detailed error message
      const error = batchResult.error || singleResult.error;
      const firstError = error.issues[0];
      setValidationResult({
        valid: false,
        error: firstError?.message || t('jsonImport.invalidFormat'),
      });
    } catch (error) {
      setValidationResult({
        valid: false,
        error: error instanceof Error ? error.message : t('jsonImport.invalidJson'),
      });
    }
  };

  const handleImport = async () => {
    if (!validationResult?.valid) {
      toast.error(t('jsonImport.validateFirst'));
      return;
    }

    if (importMode === 'single') {
      await importSingleMutation.mutateAsync(jsonInput);
    } else {
      await importBatchMutation.mutateAsync(jsonInput);
    }
  };

  const formatJson = () => {
    try {
      const parsed = JSON.parse(jsonInput);
      const formatted = JSON.stringify(parsed, null, 2);
      setJsonInput(formatted);
    } catch (error) {
      toast.error(t('jsonImport.cannotFormat'));
    }
  };

  const exampleSingle = {
    name: 'My Account',
    provider: providerPlaceholder,
    cookies: {
      session: 'your_session_value',
    },
    api_user: '12345',
  };

  const exampleBatch = [
    {
      name: 'Account 1',
      provider: providerPlaceholder,
      cookies: { session: 'session_1' },
      api_user: '12345',
    },
    {
      name: 'Account 2',
      provider: providerPlaceholder,
      cookies: { session: 'session_2' },
      api_user: '67890',
    },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileJson className="h-5 w-5" />
            {t('jsonImport.title')}
          </DialogTitle>
          <DialogDescription>
            {t('jsonImport.description')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* JSON Input */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="json-input">{t('jsonImport.jsonData')}</Label>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={formatJson}
                  disabled={isSubmitting}
                  className="rounded-full"
                >
                  {t('jsonImport.format')}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={validateJson}
                  disabled={isSubmitting}
                  className="rounded-full"
                >
                  {t('jsonImport.validate')}
                </Button>
              </div>
            </div>
            <textarea
              id="json-input"
              rows={12}
              className="w-full rounded-[var(--radius-control)] border border-[hsl(var(--control-border))] bg-[hsl(var(--control-bg))] px-3 py-2 text-sm text-[hsl(var(--control-text))] ring-offset-background placeholder:text-[hsl(var(--control-text-muted))] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--control-ring))] focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 font-mono"
              placeholder={t('jsonImport.placeholder')}
              value={jsonInput}
              onChange={(e) => {
                setJsonInput(e.target.value);
                setValidationResult(null);
              }}
              disabled={isSubmitting}
            />
          </div>

          {/* Validation Result */}
          {validationResult && (
            <div
              className={`rounded-lg border p-4 space-y-3 ${
                validationResult.valid
                  ? 'border-success-border bg-success-soft'
                  : 'border-danger-border bg-danger-soft'
              }`}
            >
              <div className="flex items-center gap-2">
                {validationResult.valid ? (
                  <CheckCircle2 className="h-5 w-5 text-success" />
                ) : (
                  <AlertCircle className="h-5 w-5 text-danger" />
                )}
                <span className="font-medium">
                  {validationResult.valid ? t('jsonImport.validJson') : t('jsonImport.invalidJson')}
                </span>
                {validationResult.accounts && (
                  <Badge variant="secondary">
                    {validationResult.accounts.length}{' '}
                    {validationResult.accounts.length === 1 ? t('jsonImport.account') : t('jsonImport.accounts')}
                  </Badge>
                )}
              </div>

              {validationResult.error && (
                <p className="text-sm text-danger">{validationResult.error}</p>
              )}

              {validationResult.accounts && validationResult.accounts.length > 0 && (
                <div className="space-y-2">
                  <p className="text-sm font-medium">{t('jsonImport.preview')}</p>
                  <div className="space-y-1">
                    {validationResult.accounts.map((account, index) => (
                      <div
                        key={index}
                        className="flex items-center gap-2 text-sm p-2 rounded bg-background/50"
                      >
                        {account.valid ? (
                          <CheckCircle2 className="h-4 w-4 text-success" />
                        ) : (
                          <AlertCircle className="h-4 w-4 text-danger" />
                        )}
                        <span>{account.name}</span>
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

          {/* Examples */}
          <div className="rounded-lg border border-border bg-muted/50 p-4 space-y-3">
            <h4 className="text-sm font-medium">{t('jsonImport.examples')}</h4>
            <div className="space-y-2">
              <div>
                <p className="text-xs font-medium mb-1">{t('jsonImport.singleAccount')}</p>
                <pre className="text-xs bg-background p-2 rounded overflow-x-auto">
                  {JSON.stringify(exampleSingle, null, 2)}
                </pre>
              </div>
              <div>
                <p className="text-xs font-medium mb-1">{t('jsonImport.multipleAccounts')}</p>
                <pre className="text-xs bg-background p-2 rounded overflow-x-auto">
                  {JSON.stringify(exampleBatch, null, 2)}
                </pre>
              </div>
            </div>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => onOpenChange(false)}
              disabled={isSubmitting}
              className="rounded-full"
            >
              {t('jsonImport.cancel')}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleImport}
              disabled={!validationResult?.valid || isSubmitting}
              className="rounded-full"
            >
              {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              <Upload className="mr-2 h-4 w-4" />
              {t('jsonImport.import')} {importMode === 'batch' ? t('jsonImport.accounts') : t('jsonImport.account')}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
