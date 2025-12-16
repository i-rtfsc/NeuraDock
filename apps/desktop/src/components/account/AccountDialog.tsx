import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { AccountForm, AccountFormValues } from './AccountForm';
import { CreateAccountInput, UpdateAccountInput } from '@/lib/tauri-commands';
import { useCreateAccount, useUpdateAccount } from '@/hooks/useAccounts';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

interface AccountDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mode: 'create' | 'edit';
  accountId?: string;
  defaultValues?: {
    name?: string;
    provider_id?: string;
    cookies?: Record<string, string>;
    api_user?: string;
    auto_checkin_enabled?: boolean;
    auto_checkin_hour?: number;
    auto_checkin_minute?: number;
  };
}

export function AccountDialog({
  open,
  onOpenChange,
  mode,
  accountId,
  defaultValues,
}: AccountDialogProps) {
  const { t } = useTranslation();
  const createMutation = useCreateAccount();
  const updateMutation = useUpdateAccount();

  const isSubmitting = createMutation.isPending || updateMutation.isPending;

  const handleSubmit = async (values: AccountFormValues) => {
    try {
      // Parse cookies JSON
      const cookies = JSON.parse(values.cookies_json) as Record<string, string>;

      if (mode === 'create') {
        const input: CreateAccountInput = {
          name: values.name,
          provider_id: values.provider_id,
          cookies,
          api_user: values.api_user || undefined,
          auto_checkin_enabled: values.auto_checkin_enabled,
          auto_checkin_hour: values.auto_checkin_hour,
          auto_checkin_minute: values.auto_checkin_minute,
        };

        await createMutation.mutateAsync(input);
        toast.success(t('accountDialog.createSuccess'));
        onOpenChange(false);
      } else if (mode === 'edit' && accountId) {
        // Check if cookies were actually changed
        const originalCookiesJson = defaultValues?.cookies 
          ? JSON.stringify(defaultValues.cookies, null, 2) 
          : '{"session": ""}';
        const cookiesChanged = values.cookies_json !== originalCookiesJson;
        
        const input: UpdateAccountInput = {
          account_id: accountId,
          name: values.name,
          cookies: cookiesChanged ? cookies : undefined,  // Only send cookies if changed
          api_user: values.api_user || undefined,
          auto_checkin_enabled: values.auto_checkin_enabled,
          auto_checkin_hour: values.auto_checkin_hour,
          auto_checkin_minute: values.auto_checkin_minute,
        };

        await updateMutation.mutateAsync(input);
        toast.success(t('accountDialog.updateSuccess'));
        onOpenChange(false);
      }
    } catch (error) {
      console.error('Failed to save account:', error);
      toast.error(
        mode === 'create'
          ? t('accountDialog.createError')
          : t('accountDialog.updateError')
      );
    }
  };

  const formDefaultValues = defaultValues
    ? {
        name: defaultValues.name || '',
        provider_id: defaultValues.provider_id || '',
        cookies_json: defaultValues.cookies
          ? JSON.stringify(defaultValues.cookies, null, 2)
          : '{"session": ""}',
        api_user: defaultValues.api_user || '',
        auto_checkin_enabled: defaultValues.auto_checkin_enabled ?? false,
        auto_checkin_hour: defaultValues.auto_checkin_hour ?? 9,
        auto_checkin_minute: defaultValues.auto_checkin_minute ?? 0,
      }
    : undefined;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {mode === 'create' ? t('accountDialog.createTitle') : t('accountDialog.editTitle')}
          </DialogTitle>
          <DialogDescription>
            {mode === 'create'
              ? t('accountDialog.createDescription')
              : t('accountDialog.editDescription')}
          </DialogDescription>
        </DialogHeader>

        <AccountForm
          mode={mode}
          defaultValues={formDefaultValues}
          onSubmit={handleSubmit}
          onCancel={() => onOpenChange(false)}
          isSubmitting={isSubmitting}
        />
      </DialogContent>
    </Dialog>
  );
}
