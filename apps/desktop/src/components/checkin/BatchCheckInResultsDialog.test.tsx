import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { BatchCheckInResultsDialog } from './BatchCheckInResultsDialog';
import type { CheckInResult } from '@/hooks/useCheckIn';

function result(overrides: Partial<CheckInResult> & Pick<CheckInResult, 'account_id'>): CheckInResult {
  return {
    account_name: `account ${overrides.account_id}`,
    provider_id: 'provider-1',
    success: false,
    outcome: 'failed_retryable',
    error_code: null,
    error: null,
    ...overrides,
  };
}

const results: CheckInResult[] = [
  result({ account_id: 'acc-1', account_name: 'Alpha', success: true, outcome: 'success' }),
  result({
    account_id: 'acc-2',
    account_name: 'Beta',
    outcome: 'failed_permanent',
    error_code: 'http_status_client',
    error: 'provider said no',
  }),
  result({
    account_id: 'acc-3',
    account_name: 'Gamma',
    outcome: 'failed_retryable',
    error_code: 'timeout',
  }),
  result({
    account_id: 'acc-4',
    account_name: 'Delta',
    outcome: 'skipped_guard',
    error: 'another attempt is already in flight',
  }),
];

describe('BatchCheckInResultsDialog', () => {
  it('lists one row per account result', () => {
    render(<BatchCheckInResultsDialog open onOpenChange={vi.fn()} results={results} />);

    expect(screen.getAllByRole('listitem')).toHaveLength(4);
    for (const name of ['Alpha', 'Beta', 'Gamma', 'Delta']) {
      expect(screen.getByText(name)).toBeVisible();
    }
  });

  it('translates known reason codes and falls back to the backend message', () => {
    render(<BatchCheckInResultsDialog open onOpenChange={vi.fn()} results={results} />);

    // i18n is mocked to return the key, so the assertion is on the key.
    expect(screen.getByText('checkIn.errorCode.http_status_client')).toBeVisible();
    expect(screen.getByText('checkIn.errorCode.timeout')).toBeVisible();
    // No reason code for the guard skip, so the provider/app sentence shows.
    expect(screen.getByText('another attempt is already in flight')).toBeVisible();
  });

  it('paints each outcome with its tone class', () => {
    render(<BatchCheckInResultsDialog open onOpenChange={vi.fn()} results={results} />);

    // Radix portals the dialog into document.body, so query the document.
    expect(document.querySelectorAll('.check-in-tone-success')).toHaveLength(1);
    expect(document.querySelectorAll('.check-in-tone-error')).toHaveLength(1);
    expect(document.querySelectorAll('.check-in-tone-warning')).toHaveLength(1);
    expect(document.querySelectorAll('.check-in-tone-info')).toHaveLength(1);
    expect(document.querySelector('[data-tone="error"]')).not.toBeNull();
  });

  it('reports the status of an already-confirmed day as checked in', () => {
    render(
      <BatchCheckInResultsDialog
        open
        onOpenChange={vi.fn()}
        results={[result({ account_id: 'acc-1', outcome: 'already_confirmed', success: true })]}
      />
    );

    expect(screen.getByText('checkIn.resultCheckedIn')).toBeVisible();
    expect(screen.getByText('checkIn.outcome.already_confirmed')).toBeVisible();
  });
});
