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
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { Loader2, Info } from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';

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
            {mode === 'create' ? '添加中转站' : '编辑中转站'}
          </DialogTitle>
          <DialogDescription>
            {mode === 'create'
              ? '配置新的中转站信息。基本信息为必填，API配置为可选（使用new-api标准默认值）。'
              : '修改中转站配置信息。'}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit(handleFormSubmit)} className="space-y-6">
          <Tabs defaultValue="basic" className="w-full">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="basic">基本信息</TabsTrigger>
              <TabsTrigger value="advanced">API配置（可选）</TabsTrigger>
            </TabsList>

            {/* Basic Tab */}
            <TabsContent value="basic" className="space-y-4 mt-4">
              {/* Name */}
              <div className="space-y-2">
                <Label htmlFor="name" className="flex items-center gap-2">
                  中转站名称 <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="name"
                  placeholder="例如：我的中转站"
                  {...register('name', {
                    required: '请输入中转站名称',
                  })}
                  className={cn(errors.name && 'border-destructive')}
                />
                {errors.name && (
                  <p className="text-sm text-destructive">{errors.name.message}</p>
                )}
              </div>

              {/* Domain */}
              <div className="space-y-2">
                <Label htmlFor="domain" className="flex items-center gap-2">
                  域名地址 <span className="text-destructive">*</span>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                    </TooltipTrigger>
                    <TooltipContent>
                      <p>必须以 http:// 或 https:// 开头</p>
                    </TooltipContent>
                  </Tooltip>
                </Label>
                <Input
                  id="domain"
                  placeholder="https://api.example.com"
                  {...register('domain', {
                    required: '请输入域名地址',
                    pattern: {
                      value: /^https?:\/\/.+/,
                      message: '域名必须以 http:// 或 https:// 开头',
                    },
                  })}
                  className={cn(errors.domain && 'border-destructive')}
                />
                {errors.domain && (
                  <p className="text-sm text-destructive">{errors.domain.message}</p>
                )}
              </div>

              {/* WAF Bypass */}
              <div className="flex items-center justify-between space-x-2 rounded-lg border p-4">
                <div className="space-y-0.5">
                  <Label htmlFor="needs_waf_bypass" className="text-base">
                    需要WAF绕过
                  </Label>
                  <div className="text-sm text-muted-foreground">
                    如果中转站使用Cloudflare等WAF保护，请启用此选项
                  </div>
                </div>
                <Switch
                  id="needs_waf_bypass"
                  checked={needsWafBypass}
                  onCheckedChange={(checked) => setValue('needs_waf_bypass', checked)}
                />
              </div>

              {/* Supports check-in */}
              <div className="flex items-center justify-between space-x-2 rounded-lg border p-4">
                <div className="space-y-0.5">
                  <Label htmlFor="supports_check_in" className="text-base">
                    支持自动签到
                  </Label>
                  <div className="text-sm text-muted-foreground">
                    如果中转站仅提供余额查询，请关闭此选项
                  </div>
                </div>
                <Switch
                  id="supports_check_in"
                  checked={supportsCheckIn}
                  onCheckedChange={(checked) => {
                    setValue('supports_check_in', checked);
                    if (!checked) {
                      setValue('check_in_bugged', false);
                    }
                  }}
                />
              </div>

              {/* Known bug toggle */}
              <div className="flex items-center justify-between space-x-2 rounded-lg border p-4">
                <div className="space-y-0.5">
                  <Label htmlFor="check_in_bugged" className="text-base">
                    签到功能暂不可用
                  </Label>
                  <div className="text-sm text-muted-foreground">
                    如果当前版本存在已知问题，请开启该选项并提示用户刷新余额
                  </div>
                </div>
                <Switch
                  id="check_in_bugged"
                  checked={checkInBugged}
                  disabled={!supportsCheckIn}
                  onCheckedChange={(checked) => setValue('check_in_bugged', checked)}
                />
              </div>
            </TabsContent>

            {/* Advanced Tab */}
            <TabsContent value="advanced" className="space-y-4 mt-4">
              <div className="rounded-lg border border-border/50 bg-muted/30 p-4 text-sm text-muted-foreground mb-4">
                <p className="font-medium text-foreground mb-2">💡 默认值说明</p>
                <p>以下配置项都是可选的，使用new-api标准默认值。如果你的中转站遵循new-api标准，可以不填写。</p>
              </div>

              {/* Login Path */}
              <div className="space-y-2">
                <Label htmlFor="login_path" className="flex items-center gap-2">
                  登录路径
                  <span className="text-xs text-muted-foreground">(默认: /login)</span>
                </Label>
                <Input
                  id="login_path"
                  placeholder="/login"
                  {...register('login_path')}
                />
              </div>

              {/* Sign In Path */}
              <div className="space-y-2">
                <Label htmlFor="sign_in_path" className="flex items-center gap-2">
                  签到接口路径
                  <span className="text-xs text-muted-foreground">(默认: /api/user/sign_in)</span>
                </Label>
                <Input
                  id="sign_in_path"
                  placeholder="/api/user/sign_in"
                  {...register('sign_in_path')}
                />
              </div>

              {/* User Info Path */}
              <div className="space-y-2">
                <Label htmlFor="user_info_path" className="flex items-center gap-2">
                  用户信息接口路径
                  <span className="text-xs text-muted-foreground">(默认: /api/user/self)</span>
                </Label>
                <Input
                  id="user_info_path"
                  placeholder="/api/user/self"
                  {...register('user_info_path')}
                />
              </div>

              {/* Token API Path */}
              <div className="space-y-2">
                <Label htmlFor="token_api_path" className="flex items-center gap-2">
                  Token接口路径
                  <span className="text-xs text-muted-foreground">(默认: /api/token/)</span>
                </Label>
                <Input
                  id="token_api_path"
                  placeholder="/api/token/"
                  {...register('token_api_path')}
                />
              </div>

              {/* Models Path */}
              <div className="space-y-2">
                <Label htmlFor="models_path" className="flex items-center gap-2">
                  模型列表接口路径
                  <span className="text-xs text-muted-foreground">(默认: /api/user/models)</span>
                </Label>
                <Input
                  id="models_path"
                  placeholder="/api/user/models"
                  {...register('models_path')}
                />
              </div>

              {/* API User Key */}
              <div className="space-y-2">
                <Label htmlFor="api_user_key" className="flex items-center gap-2">
                  Cookie用户标识键名
                  <span className="text-xs text-muted-foreground">(默认: new-api-user)</span>
                </Label>
                <Input
                  id="api_user_key"
                  placeholder="new-api-user"
                  {...register('api_user_key')}
                />
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
              取消
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {mode === 'create' ? '添加' : '保存'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
