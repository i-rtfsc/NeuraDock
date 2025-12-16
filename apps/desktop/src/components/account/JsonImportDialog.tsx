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
import { invalidateAccountCaches } from '@/hooks/useAccounts';
import { toast } from 'sonner';
import { Upload, FileJson, AlertCircle, CheckCircle2, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

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
      invalidateAccountCaches(queryClient);
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
    onSuccess: (accountIds) => {
      invalidateAccountCaches(queryClient);
      toast.success(t('jsonImport.importBatchSuccess', { count: accountIds.length }));
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

      // Check if it's an array (batch) or object (single)
      if (Array.isArray(parsed)) {
        // Batch mode
        const accounts = parsed.map((account: any, index: number) => ({
          name: account.name || `Account ${index + 1}`,
          provider: account.provider || account.provider_id || providerPlaceholder,
          valid: !!account.cookies && (typeof account.cookies === 'object' || typeof account.cookies === 'string'),
        }));

        setValidationResult({
          valid: accounts.every((a) => a.valid),
          accounts,
        });
        setImportMode('batch');
      } else if (typeof parsed === 'object' && parsed !== null) {
        // Single mode
        const valid = !!parsed.cookies && (typeof parsed.cookies === 'object' || typeof parsed.cookies === 'string');
        setValidationResult({
          valid,
          accounts: [
            {
              name: parsed.name || 'Unnamed Account',
              provider: parsed.provider || parsed.provider_id || providerPlaceholder,
              valid,
            },
          ],
        });
        setImportMode('single');
      } else {
        setValidationResult({
          valid: false,
          error: t('jsonImport.invalidFormat'),
        });
      }
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
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 font-mono"
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
                <p className="text-sm text-red-500">{validationResult.error}</p>
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
                          <CheckCircle2 className="h-4 w-4 text-green-500" />
                        ) : (
                          <AlertCircle className="h-4 w-4 text-red-500" />
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
