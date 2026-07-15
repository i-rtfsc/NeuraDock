import { useEffect, useState } from 'react';

import { migrationCommands } from '@/lib/tauri-commands';
import { logger } from '@/lib/logger';

import { LegacyImportDialog } from './LegacyImportDialog';

export function LegacyMigrationPrompt() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const startedAt = window.performance.now();

    // Dismissal is recorded backend-side (a marker in the app data dir), so
    // detect already returns false for dismissed/completed installs and
    // wiping the data directory resets the prompt.
    migrationCommands
      .detectLegacyInstallation()
      .then((available) => {
        if (cancelled) return;
        logger.info(
          'legacy installation detection completed',
          {
            available,
            elapsed_ms: Number((window.performance.now() - startedAt).toFixed(2)),
          },
          'frontend::startup',
        );
        if (available) {
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
  }, []);

  return (
    <LegacyImportDialog
      open={open}
      onOpenChange={(nextOpen) => {
        setOpen(nextOpen);
        if (!nextOpen) {
          migrationCommands.dismissPrompt().catch((error) => {
            logger.warn(
              'failed to persist legacy import dismissal',
              { error: String(error) },
              'frontend::startup',
            );
          });
        }
      }}
    />
  );
}
