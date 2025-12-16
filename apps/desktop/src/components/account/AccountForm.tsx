import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useEffect, useState } from 'react';
import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useProviders } from '@/hooks/useProviders';

// Validation schema
const getAccountFormSchema = (t: any) => z.object({
  name: z.string().min(1, t('accountForm.accountNameRequired')).max(100),
  provider_id: z.string().min(1, t('accountForm.providerRequired')),
  cookies_json: z.string().min(1, t('accountForm.cookiesRequired')).refine(
    (val) => {
      try {
        const parsed = JSON.parse(val);
        return typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed);
      } catch {
        return false;
      }
    },
    { message: t('accountForm.cookiesInvalidJson') }
  ),
  api_user: z.string().optional(),
  auto_checkin_enabled: z.boolean().optional(),
  auto_checkin_hour: z.number().min(0).max(23).optional(),
  auto_checkin_minute: z.number().min(0).max(59).optional(),
});

export type AccountFormValues = z.infer<ReturnType<typeof getAccountFormSchema>>;

interface AccountFormProps {
  mode: 'create' | 'edit';
  defaultValues?: Partial<AccountFormValues>;
  onSubmit: (values: AccountFormValues) => Promise<void>;
  onCancel?: () => void;
  isSubmitting?: boolean;
}

export function AccountForm({
  mode,
  defaultValues,
  onSubmit,
  onCancel,
  isSubmitting = false,
}: AccountFormProps) {
  const [cookiesError, setCookiesError] = useState<string | null>(null);
  const { t } = useTranslation();
  const { data: providers = [], isLoading: isLoadingProviders } = useProviders();

  const accountFormSchema = getAccountFormSchema(t);
  const initialProviderId =
    defaultValues?.provider_id || (providers.length > 0 ? providers[0].id : '');

  const {
    register,
    handleSubmit,
    formState: { errors },
    setValue,
    watch,
  } = useForm<AccountFormValues>({
    resolver: zodResolver(accountFormSchema),
    defaultValues: {
      name: defaultValues?.name || '',
      provider_id: initialProviderId,
      cookies_json: defaultValues?.cookies_json || '{"session": ""}',
      api_user: defaultValues?.api_user || '',
      auto_checkin_enabled: defaultValues?.auto_checkin_enabled ?? false,
      auto_checkin_hour: defaultValues?.auto_checkin_hour ?? 9,
      auto_checkin_minute: defaultValues?.auto_checkin_minute ?? 0,
    },
  });

  useEffect(() => {
    if (!defaultValues?.provider_id && providers.length > 0) {
      setValue('provider_id', providers[0].id);
    }
  }, [defaultValues?.provider_id, providers, setValue]);

  const provider_id = watch('provider_id');
  const autoCheckinEnabled = watch('auto_checkin_enabled');

  const handleFormSubmit = async (data: AccountFormValues) => {
    setCookiesError(null);
    try {
      // Validate JSON one more time before submit
      JSON.parse(data.cookies_json);
      await onSubmit(data);
    } catch (error) {
      if (error instanceof SyntaxError) {
        setCookiesError(t('accountForm.invalidJsonFormat'));
      } else {
        throw error;
      }
    }
  };

  const formatJson = () => {
    try {
      const cookies_json = watch('cookies_json');
      const parsed = JSON.parse(cookies_json);
      const formatted = JSON.stringify(parsed, null, 2);
      setValue('cookies_json', formatted);
      setCookiesError(null);
    } catch (error) {
      setCookiesError(t('accountForm.cannotFormatInvalidJson'));
    }
  };

  return (
    <form onSubmit={handleSubmit(handleFormSubmit)} className="space-y-6">
      {/* Account Name */}
      <div className="space-y-2">
        <Label htmlFor="name">
          {t('accountForm.accountName')} <span className="text-destructive">*</span>
        </Label>
        <Input
          id="name"
          placeholder={t('accountForm.accountNamePlaceholder')}
          {...register('name')}
          disabled={isSubmitting}
        />
        {errors.name && (
          <p className="text-sm text-destructive">{errors.name.message}</p>
        )}
      </div>

      {/* Provider Selection */}
      <div className="space-y-2">
        <Label htmlFor="provider">
          {t('accountForm.provider')} <span className="text-destructive">*</span>
        </Label>
        <Select
          value={provider_id}
          onValueChange={(value) => setValue('provider_id', value)}
          disabled={isSubmitting || isLoadingProviders}
        >
          <SelectTrigger id="provider">
            <SelectValue placeholder={t('accountForm.selectProvider')} />
          </SelectTrigger>
          <SelectContent>
            {isLoadingProviders ? (
              <SelectItem value="loading" disabled>加载中...</SelectItem>
            ) : providers.length === 0 ? (
              <SelectItem value="none" disabled>暂无可用中转站</SelectItem>
            ) : (
              providers.map((provider) => (
                <SelectItem key={provider.id} value={provider.id}>
                  {provider.name}
                  {provider.is_builtin && <span className="text-xs text-muted-foreground ml-2">(内置)</span>}
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
        {errors.provider_id && (
          <p className="text-sm text-destructive">{errors.provider_id.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          {t('accountForm.chooseProvider')}
        </p>
      </div>

      {/* Cookies (JSON) */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label htmlFor="cookies">
            {t('accountForm.cookies')} <span className="text-destructive">*</span>
          </Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={formatJson}
            disabled={isSubmitting}
            className="rounded-full"
          >
            {t('accountForm.formatJson')}
          </Button>
        </div>
        <textarea
          id="cookies"
          rows={6}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 font-mono"
          placeholder={t('accountForm.cookiesPlaceholder')}
          {...register('cookies_json')}
          disabled={isSubmitting}
        />
        {(errors.cookies_json || cookiesError) && (
          <p className="text-sm text-destructive">
            {errors.cookies_json?.message || cookiesError}
          </p>
        )}
        <p className="text-xs text-muted-foreground">
          {t('accountForm.cookiesDescription')}
        </p>
      </div>

      {/* API User */}
      <div className="space-y-2">
        <Label htmlFor="api_user">{t('accountForm.apiUser')}</Label>
        <Input
          id="api_user"
          placeholder={t('accountForm.apiUserPlaceholder')}
          {...register('api_user')}
          disabled={isSubmitting}
        />
        {errors.api_user && (
          <p className="text-sm text-destructive">{errors.api_user.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          {t('accountForm.apiUserDescription')}
        </p>
      </div>

      {/* Auto Check-in Settings */}
      <div className="space-y-4 rounded-lg border border-border bg-muted/30 p-4">
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label htmlFor="auto_checkin">{t('settings.autoCheckin')}</Label>
            <p className="text-xs text-muted-foreground">
              {t('settings.autoCheckinDaily')}
            </p>
          </div>
          <Switch
            id="auto_checkin"
            checked={autoCheckinEnabled || false}
            onCheckedChange={(checked) => setValue('auto_checkin_enabled', checked)}
            disabled={isSubmitting}
          />
        </div>

        {autoCheckinEnabled && (
          <div className="grid grid-cols-2 gap-4 pt-2">
            <div className="space-y-2">
              <Label htmlFor="auto_checkin_hour">{t('settings.hour')}</Label>
              <Select
                value={watch('auto_checkin_hour')?.toString() || '9'}
                onValueChange={(value) => setValue('auto_checkin_hour', parseInt(value))}
                disabled={isSubmitting}
              >
                <SelectTrigger id="auto_checkin_hour">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Array.from({ length: 24 }, (_, i) => (
                    <SelectItem key={i} value={i.toString()}>
                      {i.toString().padStart(2, '0')}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="auto_checkin_minute">{t('settings.minute')}</Label>
              <Select
                value={watch('auto_checkin_minute')?.toString() || '0'}
                onValueChange={(value) => setValue('auto_checkin_minute', parseInt(value))}
                disabled={isSubmitting}
              >
                <SelectTrigger id="auto_checkin_minute">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Array.from({ length: 60 }, (_, i) => (
                    <SelectItem key={i} value={i.toString()}>
                      {i.toString().padStart(2, '0')}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        )}
      </div>

      {/* Help Text */}
      <div className="rounded-lg border border-border bg-muted/50 p-4 space-y-2">
        <h4 className="text-sm font-medium">{t('accountForm.howToGetValues')}</h4>
        <ol className="text-xs text-muted-foreground space-y-1 list-decimal list-inside">
          <li>
            <strong>Cookies:</strong> {t('accountForm.cookiesHelp')}
          </li>
          <li>
            <strong>API User:</strong> {t('accountForm.apiUserHelp')}
          </li>
        </ol>
      </div>

      {/* Actions */}
      <div className="flex justify-end gap-3 pt-4">
        {onCancel && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onCancel}
            disabled={isSubmitting}
            className="rounded-full"
          >
            {t('accountForm.cancel')}
          </Button>
        )}
        <Button 
          type="submit" 
          variant="outline" 
          size="sm" 
          disabled={isSubmitting} 
          className="rounded-full"
        >
          {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
          {mode === 'create' ? t('accountForm.create') : t('accountForm.save')}
        </Button>
      </div>
    </form>
  );
}
