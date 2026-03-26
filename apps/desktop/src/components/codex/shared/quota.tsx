import { Badge } from '@/components/ui/badge';
import type { CodexRateLimitWindowDto } from '@/lib/tauri';
import type { TFunction } from 'i18next';

type QuotaState = {
  hasCredits?: boolean | null;
  isUnlimited?: boolean | null;
};

export function QuotaBadge({ hasCredits, isUnlimited }: QuotaState) {
  if (isUnlimited) {
    return <Badge variant="soft-success" className="text-xs">无限额度</Badge>;
  }

  if (hasCredits === true) {
    return <Badge variant="soft-success" className="text-xs">有额度</Badge>;
  }

  if (hasCredits === false) {
    return <Badge variant="soft-danger" className="text-xs">无额度</Badge>;
  }

  return <Badge variant="soft-warning" className="text-xs">未查询</Badge>;
}

export function formatPlanLabel(planType?: string | null, t?: TFunction) {
  if (!planType) return t ? t('codex.common.empty') : '—';
  return planType.charAt(0).toUpperCase() + planType.slice(1);
}

export function getRemainingPercent(window?: CodexRateLimitWindowDto | null) {
  if (!window) return null;
  return Math.max(0, Math.min(100, 100 - window.usedPercent));
}

export function formatRemainingPercent(
  window?: CodexRateLimitWindowDto | null,
  isUnlimited?: boolean | null,
  t?: TFunction,
) {
  if (isUnlimited) return t ? t('codex.quota.unlimited') : 'Unlimited';
  const remaining = getRemainingPercent(window);
  if (remaining == null) return t ? t('codex.common.empty') : '—';
  const rounded = Number.isInteger(remaining) ? remaining.toFixed(0) : remaining.toFixed(1);
  return `${rounded}%`;
}

export function pickPrimaryDisplayWindow(
  secondaryWindow?: CodexRateLimitWindowDto | null,
  primaryWindow?: CodexRateLimitWindowDto | null,
) {
  return secondaryWindow ?? primaryWindow ?? null;
}

export function formatWindowLabel(window?: CodexRateLimitWindowDto | null, t?: TFunction) {
  if (!window?.windowMinutes) {
    return t ? t('codex.quota.cycleFallback') : 'Quota cycle';
  }

  const minutes = window.windowMinutes;
  if (minutes >= 60 * 24 * 28) return t ? t('codex.quota.monthlyQuota') : 'Monthly quota';
  if (minutes >= 60 * 24 * 6) return t ? t('codex.quota.weeklyQuota') : 'Weekly quota';
  if (minutes >= 60 * 24) return t ? t('codex.quota.dailyQuota') : 'Daily quota';

  const hours = Math.max(1, Math.round(minutes / 60));
  return t ? t('codex.quota.hourlyQuota', { hours }) : `${hours}h`;
}

export function formatResetAt(resetAt?: string | null, locale = 'zh-CN', emptyLabel = '—') {
  if (!resetAt) return emptyLabel;
  const date = new Date(resetAt);
  if (Number.isNaN(date.getTime())) return emptyLabel;
  return date.toLocaleString(locale, { hour12: false });
}

export function formatResetHint(resetAt?: string | null, t?: TFunction) {
  if (!resetAt) return null;
  const diffMs = new Date(resetAt).getTime() - Date.now();
  if (Number.isNaN(diffMs)) return null;
  if (diffMs <= 0) return t ? t('codex.quota.resetSoon') : 'Resets soon';

  const totalMinutes = Math.max(1, Math.floor(diffMs / 60_000));
  const days = Math.floor(totalMinutes / (60 * 24));
  const hours = Math.floor((totalMinutes % (60 * 24)) / 60);
  const minutes = totalMinutes % 60;

  if (days > 0) {
    return t ? t('codex.quota.remainingDaysHours', { days, hours }) : `${days}d ${hours}h left`;
  }
  if (hours > 0) {
    return t ? t('codex.quota.remainingHoursMinutes', { hours, minutes }) : `${hours}h ${minutes}m left`;
  }
  return t ? t('codex.quota.remainingMinutes', { minutes }) : `${minutes}m left`;
}
