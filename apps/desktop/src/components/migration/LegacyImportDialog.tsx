import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Loader2 } from 'lucide-react';
import { relaunch } from '@tauri-apps/plugin-process';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { migrationCommands } from '@/lib/tauri-commands';

type LegacyImportStep = 'prompt' | 'confirm-overwrite';

function getCommandErrorMessage(error: unknown): string | null {
  if (typeof error === 'string') {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (error && typeof error === 'object') {
    const maybe = error as { message?: unknown };
    if (typeof maybe.message === 'string') {
      return maybe.message;
    }
  }

  return null;
}

interface LegacyImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function LegacyImportDialog({ open, onOpenChange }: LegacyImportDialogProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<LegacyImportStep>('prompt');
  const [isImporting, setIsImporting] = useState(false);
  const relaunchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (relaunchTimerRef.current) clearTimeout(relaunchTimerRef.current);
    };
  }, []);

  const title = useMemo(() => {
    if (step === 'confirm-overwrite') {
      return t('settings.legacyImportOverwriteTitle', '覆盖当前数据？');
    }
    return t('settings.legacyImportDialogTitle', '检测到旧版本数据，是否导入？');
  }, [step, t]);

  const description = useMemo(() => {
    if (step === 'confirm-overwrite') {
      return t('settings.legacyImportOverwriteDescription', '当前安装已存在数据，导入将覆盖这些数据。');
    }
    return t('settings.legacyImportDialogDescription', '我们发现了旧版 NeuraDock 的数据。你可以将其导入到当前版本。');
  }, [step, t]);

  const closeAndReset = useCallback(() => {
    setStep('prompt');
    onOpenChange(false);
  }, [onOpenChange]);

  const runImport = useCallback(
    async (overwrite: boolean) => {
      setIsImporting(true);
      try {
        const result = await migrationCommands.importLegacyData(overwrite);

        if (result.already_staged) {
          toast.success(t('settings.legacyImportAlreadyStaged', '导入已准备完成，即将重启应用。'));
        } else {
          toast.success(t('settings.legacyImportSuccess', '导入已准备完成，即将重启应用。'));
        }
        // Give user a moment to see the message before restarting
        if (relaunchTimerRef.current) clearTimeout(relaunchTimerRef.current);
        relaunchTimerRef.current = setTimeout(() => {
          relaunch();
        }, 1000);

        closeAndReset();
      } catch (error: unknown) {
        const message = getCommandErrorMessage(error);

        if (message?.startsWith('LEGACY_IMPORT_OVERWRITE_REQUIRED:')) {
          setStep('confirm-overwrite');
          setIsImporting(false);
          return;
        }

        if (message === 'LEGACY_IMPORT_NOT_FOUND') {
          toast.error(t('settings.legacyImportNotFound', '未找到可导入的旧版本数据。'));
          setIsImporting(false);
          return;
        }

        toast.error(
          t('settings.legacyImportFailed', {
            defaultValue: '导入失败：{{message}}',
            message: message ?? String(error),
          }),
        );
        setIsImporting(false);
      }
    },
    [closeAndReset, t],
  );

  const handleOpenChange = useCallback(
    (nextOpen: boolean) => {
      if (!nextOpen) {
        setStep('prompt');
      }
      onOpenChange(nextOpen);
    },
    [onOpenChange],
  );

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        <p className="text-sm text-muted-foreground">
          {t('settings.legacyImportRestartNote', '导入完成后会自动重启 NeuraDock 以生效。')}
        </p>

        <DialogFooter>
          {step === 'prompt' ? (
            <>
              <Button variant="secondary" onClick={closeAndReset} disabled={isImporting}>
                {t('settings.legacyImportSkipButton', '跳过')}
              </Button>
              <Button onClick={() => runImport(false)} disabled={isImporting}>
                {isImporting ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {t('common.loading', '加载中...')}
                  </>
                ) : (
                  t('settings.legacyImportImportButton', '导入')
                )}
              </Button>
            </>
          ) : (
            <>
              <Button variant="secondary" onClick={() => setStep('prompt')} disabled={isImporting}>
                {t('common.cancel', '取消')}
              </Button>
              <Button variant="destructive" onClick={() => runImport(true)} disabled={isImporting}>
                {isImporting ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {t('common.loading', '加载中...')}
                  </>
                ) : (
                  t('settings.legacyImportOverwriteButton', '覆盖并导入')
                )}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
