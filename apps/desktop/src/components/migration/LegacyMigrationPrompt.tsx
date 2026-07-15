import { useEffect, useState } from 'react';

import { usePersistedState } from '@/hooks/usePersistedState';
import { migrationCommands } from '@/lib/tauri-commands';
import { logger } from '@/lib/logger';

import { LegacyImportDialog } from './LegacyImportDialog';

const LEGACY_MIGRATION_DISMISSED_KEY = 'legacyMigration.promptDismissed.v1';

export function LegacyMigrationPrompt() {
  const [dismissed, setDismissed] = usePersistedState<boolean>(
    LEGACY_MIGRATION_DISMISSED_KEY,
    false,
  );
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const startedAt = window.performance.now();

    migrationCommands
      .detectLegacyInstallation()
      .then((available) => {
        if (cancelled) return;
        logger.info(
          'legacy installation detection completed',
          {
            available,
            dismissed,
            elapsed_ms: Number((window.performance.now() - startedAt).toFixed(2)),
          },
          'frontend::startup',
        );
        if (available && !dismissed) {
          setOpen(true);
        }
      })
      .catch((error) => {
        logger.warn(
          'legacy installation detection failed',
          {
            error: String(error),
            elapsed_ms: Number((window.performance.now() - startedAt).toFixed(2)),
          },
          'frontend::startup',
        );
        // Ignore detection errors; migration remains accessible from Settings.
      });

    return () => {
      cancelled = true;
    };
  }, [dismissed]);

  return (
    <LegacyImportDialog
      open={open}
      onOpenChange={(nextOpen) => {
        setOpen(nextOpen);
        if (!nextOpen) {
          setDismissed(true);
        }
      }}
    />
  );
}
