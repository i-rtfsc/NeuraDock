import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CheckInDayDto } from '@/lib/tauri-commands';
import { CheckInDayDetailDialog } from './CheckInDayDetailDialog';

const day: CheckInDayDto = {
  date: '2026-01-02',
  is_checked_in: true,
  income_increment: 23.44,
  reward_is_estimate: true,
  current_balance: 123.44,
  daily_consumed: 0,
  total_consumed: 200,
  total_quota: 323.44,
};

describe('check-in reward confirmation', () => {
  it('requires explicit confirmation before recording an external check-in', async () => {
    const user = userEvent.setup();
    const confirm = vi.fn();
    render(<CheckInDayDetailDialog open onOpenChange={vi.fn()} dayData={day} onConfirmReward={confirm} />);
    const button = screen.getByRole('button', { name: 'streaks.manualRecord' });
    expect(button).toBeDisabled();
    expect(screen.getByText('streaks.rewardEstimate')).toBeVisible();
    await user.click(button);
    expect(confirm).not.toHaveBeenCalled();
    await user.click(screen.getByRole('checkbox'));
    await user.click(button);
    expect(confirm).toHaveBeenCalledOnce();
  });

  it('keeps the confirmed reward visible when the balance changes and hides re-confirmation', () => {
    const confirmed = { ...day, income_increment: 25, reward_is_estimate: false };
    const { rerender } = render(<CheckInDayDetailDialog open onOpenChange={vi.fn()} dayData={confirmed} onConfirmReward={vi.fn()} />);
    expect(screen.getByText('$25.00')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'streaks.manualRecord' })).not.toBeInTheDocument();
    rerender(<CheckInDayDetailDialog open onOpenChange={vi.fn()} dayData={{ ...confirmed, current_balance: 10, total_consumed: 315, total_quota: 325 }} />);
    expect(screen.getByText('$25.00')).toBeVisible();
  });
});
