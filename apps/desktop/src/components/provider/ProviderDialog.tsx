import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useTranslation } from 'react-i18next';
import { Loader2, Info, Globe, Shield, CalendarCheck, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Card } from '@/components/ui/card';

export interface ProviderFormValues {
  name: string;
  domain: string;
  needs_waf_bypass: boolean;
  supports_check_in: boolean;
  check_in_bugged: boolean;
  // Optional fields
  login_path?: string;
  sign_in_path?: string;
  user_info_path?: string;
  token_api_path?: string;
  models_path?: string;
  api_user_key?: string;
}

interface ProviderDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mode: 'create' | 'edit';
  providerId?: string;
  defaultValues?: Partial<ProviderFormValues>;
  onSubmit: (values: ProviderFormValues) => Promise<void>;
  isSubmitting?: boolean;
}

export function ProviderDialog({
  open,
  onOpenChange,
  mode,
  defaultValues,
  onSubmit,
  isSubmitting = false,
}: ProviderDialogProps) {
  const { t } = useTranslation();

  const {
    register,
    handleSubmit,
    formState: { errors },
    reset,
    watch,
    setValue,
  } = useForm<ProviderFormValues>({
    defaultValues: {
      name: '',
      domain: '',
      needs_waf_bypass: false,
      supports_check_in: true,
      check_in_bugged: false,
      login_path: '/login',
      sign_in_path: '/api/user/sign_in',
      user_info_path: '/api/user/self',
      token_api_path: '/api/token/',
      models_path: '/api/user/models',
      api_user_key: 'new-api-user',
    },
  });

  const needsWafBypass = watch('needs_waf_bypass');
  const supportsCheckIn = watch('supports_check_in');
  const checkInBugged = watch('check_in_bugged');

  useEffect(() => {
    if (open && defaultValues) {
      reset({
        name: defaultValues.name || '',
        domain: defaultValues.domain || '',
        needs_waf_bypass: defaultValues.needs_waf_bypass ?? false,
        supports_check_in: defaultValues.supports_check_in ?? true,
        check_in_bugged: defaultValues.check_in_bugged ?? false,
        login_path: defaultValues.login_path || '/login',
        sign_in_path: defaultValues.sign_in_path || '/api/user/sign_in',
        user_info_path: defaultValues.user_info_path || '/api/user/self',
        token_api_path: defaultValues.token_api_path || '/api/token/',
        models_path: defaultValues.models_path || '/api/user/models',
        api_user_key: defaultValues.api_user_key || 'new-api-user',
      });
    } else if (!open) {
      reset();
    }
  }, [open, defaultValues, reset]);

  const handleFormSubmit = async (values: ProviderFormValues) => {
    try {
      await onSubmit(values);
      onOpenChange(false);
    } catch (error) {
      console.error('Failed to save provider:', error);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {mode === 'create' ? t('providerDialog.createTitle') : t('providerDialog.editTitle')}
          </DialogTitle>
          <DialogDescription>
            {mode === 'create'
              ? t('providerDialog.createDescription')
              : t('providerDialog.editDescription')}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit(handleFormSubmit)} className="space-y-6">
          <Tabs defaultValue="basic" className="w-full">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="basic">{t('providerDialog.tabs.basic')}</TabsTrigger>
              <TabsTrigger value="advanced">{t('providerDialog.tabs.advanced')}</TabsTrigger>
            </TabsList>

            {/* Basic Tab */}
            <TabsContent value="basic" className="space-y-6 mt-6">
              {/* Core Info */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="space-y-2">
                  <Label htmlFor="name" className="flex items-center gap-2">
                    {t('providerDialog.fields.name.label')} <span className="text-destructive">*</span>
                  </Label>
                  <Input
                    id="name"
                    placeholder={t('providerDialog.fields.name.placeholder')}
                    {...register('name', {
                      required: t('providerDialog.fields.name.required'),
                    })}
                    className={cn(errors.name && 'border-destructive')}
                  />
                  {errors.name && (
                    <p className="text-sm text-destructive">{errors.name.message}</p>
                  )}
                </div>

                <div className="space-y-2">
                  <Label htmlFor="domain" className="flex items-center gap-2">
                    <Globe className="h-3.5 w-3.5 text-muted-foreground" />
                    {t('providerDialog.fields.domain.label')} <span className="text-destructive">*</span>
                  </Label>
                  <Input
                    id="domain"
                    placeholder={t('providerDialog.fields.domain.placeholder')}
                    {...register('domain', {
                      required: t('providerDialog.fields.domain.required'),
                      pattern: {
                        value: /^https?:\/\/.+/,
                        message: t('providerDialog.fields.domain.invalidFormat'),
                      },
                    })}
                    className={cn(errors.domain && 'border-destructive')}
                  />
                  {errors.domain && (
                    <p className="text-sm text-destructive">{errors.domain.message}</p>
                  )}
                </div>
              </div>

              {/* Capabilities Card */}
              <Card className="p-4 bg-muted/20 border-border/50">
                <h3 className="text-sm font-medium mb-4 text-muted-foreground">Capabilities Configuration</h3>
                <div className="space-y-4">
                  {/* WAF Bypass */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-start gap-3">
                      <div className="p-2 rounded-md bg-info/10 text-info mt-0.5">
                        <Shield className="h-4 w-4" />
                      </div>
                      <div className="space-y-0.5">
                        <Label htmlFor="needs_waf_bypass" className="text-base cursor-pointer">
                          {t('providerDialog.fields.needsWafBypass.label')}
                        </Label>
                        <p className="text-xs text-muted-foreground max-w-[300px]">
                          {t('providerDialog.fields.needsWafBypass.description')}
                        </p>
                      </div>
                    </div>
                    <Switch
                      id="needs_waf_bypass"
                      checked={needsWafBypass}
                      onCheckedChange={(checked) => setValue('needs_waf_bypass', checked)}
                    />
                  </div>

                  <div className="h-px bg-border/50" />

                  {/* Check-in Support */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-start gap-3">
                      <div className="p-2 rounded-md bg-success/10 text-success mt-0.5">
                        <CalendarCheck className="h-4 w-4" />
                      </div>
                      <div className="space-y-0.5">
                        <Label htmlFor="supports_check_in" className="text-base cursor-pointer">
                          {t('providerDialog.fields.supportsCheckIn.label')}
                        </Label>
                        <p className="text-xs text-muted-foreground max-w-[300px]">
                          {t('providerDialog.fields.supportsCheckIn.description')}
                        </p>
                      </div>
                    </div>
                    <Switch
                      id="supports_check_in"
                      checked={supportsCheckIn}
                      onCheckedChange={(checked) => {
                        setValue('supports_check_in', checked);
                        if (!checked) setValue('check_in_bugged', false);
                      }}
                    />
                  </div>

                  {/* Bugged Toggle (Conditional) */}
                  <div className={cn(
                    "flex items-center justify-between transition-all duration-base ease-smooth overflow-hidden",
                    supportsCheckIn ? "opacity-100 max-h-20 pt-4 border-t border-border/50" : "opacity-0 max-h-0"
                  )}>
                    <div className="flex items-start gap-3 pl-2 border-l-2 border-warning-border">
                      <div className="p-2 rounded-md bg-warning/10 text-warning mt-0.5">
                        <AlertTriangle className="h-4 w-4" />
                      </div>
                      <div className="space-y-0.5">
                        <Label htmlFor="check_in_bugged" className="text-base cursor-pointer">
                          {t('providerDialog.fields.checkInBugged.label')}
                        </Label>
                        <p className="text-xs text-muted-foreground max-w-[300px]">
                          {t('providerDialog.fields.checkInBugged.description')}
                        </p>
                      </div>
                    </div>
                    <Switch
                      id="check_in_bugged"
                      checked={checkInBugged}
                      disabled={!supportsCheckIn}
                      onCheckedChange={(checked) => setValue('check_in_bugged', checked)}
                    />
                  </div>
                </div>
              </Card>
            </TabsContent>

            {/* Advanced Tab */}
            <TabsContent value="advanced" className="space-y-6 mt-6">
              <div className="rounded-lg border border-border/50 bg-info-soft p-4 text-sm text-info-soft-foreground flex gap-3">
                <Info className="h-5 w-5 shrink-0 mt-0.5" />
                <div>
                  <p className="font-semibold mb-1">{t('providerDialog.advancedNote.title', 'Default Values Note')}</p>
                  <p className="opacity-90">{t('providerDialog.advancedNote.description')}</p>
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {/* Auth Group */}
                <div className="space-y-4">
                  <h4 className="text-sm font-medium text-muted-foreground border-b pb-2">Authentication</h4>
                  <div className="space-y-2">
                    <Label htmlFor="login_path" className="text-xs">{t('providerDialog.fields.loginPath.label')}</Label>
                    <Input id="login_path" placeholder="/login" {...register('login_path')} className="h-8 text-sm" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="sign_in_path" className="text-xs">{t('providerDialog.fields.signInPath.label')}</Label>
                    <Input id="sign_in_path" placeholder="/api/user/sign_in" {...register('sign_in_path')} className="h-8 text-sm" />
                  </div>
                </div>

                {/* User Data Group */}
                <div className="space-y-4">
                  <h4 className="text-sm font-medium text-muted-foreground border-b pb-2">User Data</h4>
                  <div className="space-y-2">
                    <Label htmlFor="user_info_path" className="text-xs">{t('providerDialog.fields.userInfoPath.label')}</Label>
                    <Input id="user_info_path" placeholder="/api/user/self" {...register('user_info_path')} className="h-8 text-sm" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="api_user_key" className="text-xs">{t('providerDialog.fields.apiUserKey.label')}</Label>
                    <Input id="api_user_key" placeholder="new-api-user" {...register('api_user_key')} className="h-8 text-sm" />
                  </div>
                </div>

                {/* Resources Group */}
                <div className="space-y-4 md:col-span-2">
                  <h4 className="text-sm font-medium text-muted-foreground border-b pb-2">Resources</h4>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div className="space-y-2">
                      <Label htmlFor="token_api_path" className="text-xs">{t('providerDialog.fields.tokenApiPath.label')}</Label>
                      <Input id="token_api_path" placeholder="/api/token/" {...register('token_api_path')} className="h-8 text-sm" />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="models_path" className="text-xs">{t('providerDialog.fields.modelsPath.label')}</Label>
                      <Input id="models_path" placeholder="/api/user/models" {...register('models_path')} className="h-8 text-sm" />
                    </div>
                  </div>
                </div>
              </div>
            </TabsContent>
          </Tabs>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isSubmitting}
            >
              {t('providerDialog.buttons.cancel')}
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {mode === 'create'
                ? (isSubmitting ? t('providerDialog.buttons.creating') : t('providerDialog.buttons.create'))
                : (isSubmitting ? t('providerDialog.buttons.saving') : t('providerDialog.buttons.save'))
              }
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
