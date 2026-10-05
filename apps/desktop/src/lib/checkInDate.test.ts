import { describe, it, expect } from 'vitest';
import { formatDay, localDayString, parseDayString } from './checkInDate';

/**
 * Node re-reads `TZ` at runtime on POSIX but ignores it on some platforms
 * (e.g. Windows); detect that once so the cross-time-zone tests can be skipped
 * instead of producing false failures.
 */
function canSwitchTimeZoneAtRuntime(): boolean {
  const previous = process.env.TZ;
  process.env.TZ = 'Asia/Shanghai';
  const applied = new Date(2026, 0, 2, 12, 0, 0).getTimezoneOffset() === -480;
  if (previous === undefined) {
    delete process.env.TZ;
  } else {
    process.env.TZ = previous;
  }
  return applied;
}

const CAN_SWITCH_TIME_ZONE = canSwitchTimeZoneAtRuntime();

/** Run `fn` with the process time zone set to `timeZone`. */
function withTimeZone<T>(timeZone: string, fn: () => T): T {
  const previous = process.env.TZ;
  process.env.TZ = timeZone;
  try {
    return fn();
  } finally {
    if (previous === undefined) {
      delete process.env.TZ;
    } else {
      process.env.TZ = previous;
    }
  }
}

describe('parseDayString', () => {
  it('parses a day string as the same local calendar day', () => {
    const date = parseDayString('2026-01-02');
    expect(date.getFullYear()).toBe(2026);
    expect(date.getMonth()).toBe(0);
    expect(date.getDate()).toBe(2);
  });

  it('pins the value to local noon so offsets cannot shift the day', () => {
    expect(parseDayString('2026-01-02').getHours()).toBe(12);
  });

  it('round-trips with localDayString for a whole year', () => {
    for (let month = 0; month < 12; month += 1) {
      for (let day = 1; day <= 28; day += 1) {
        const dayString = localDayString(new Date(2026, month, day));
        expect(dayString).toMatch(/^\d{4}-\d{2}-\d{2}$/);
        expect(localDayString(parseDayString(dayString))).toBe(dayString);
      }
    }
  });

  it('does not drift on DST transition days', () => {
    for (const dayString of ['2026-03-08', '2026-03-09', '2026-11-01', '2026-11-02']) {
      expect(localDayString(parseDayString(dayString))).toBe(dayString);
    }
  });
});

describe('localDayString', () => {
  it('reports the local day for late and early local times', () => {
    expect(localDayString(new Date(2026, 0, 2, 23, 30))).toBe('2026-01-02');
    expect(localDayString(new Date(2026, 0, 2, 0, 30))).toBe('2026-01-02');
  });

  it('pads single-digit months and days', () => {
    expect(localDayString(new Date(2026, 0, 2))).toBe('2026-01-02');
    expect(localDayString(new Date(2026, 11, 31))).toBe('2026-12-31');
  });

  it('defaults to now', () => {
    const now = new Date();
    expect(localDayString()).toBe(localDayString(now));
  });
});

describe('formatDay', () => {
  it('formats a day string in the requested locale', () => {
    expect(formatDay('2026-01-02', 'en-US')).toBe('January 2, 2026');
    expect(formatDay('2026-01-02', 'zh-CN')).toBe('2026年1月2日');
  });
});

describe.skipIf(!CAN_SWITCH_TIME_ZONE)('cross-time-zone stability', () => {
  it('keeps the calendar day in UTC+8', () => {
    withTimeZone('Asia/Shanghai', () => {
      expect(localDayString(new Date(2026, 0, 2, 0, 30))).toBe('2026-01-02');
      expect(localDayString(new Date(2026, 0, 2, 23, 30))).toBe('2026-01-02');
      // 17:00 UTC on Jan 1 is already Jan 2 locally.
      expect(localDayString(new Date(Date.UTC(2026, 0, 1, 17, 0)))).toBe('2026-01-02');
      expect(localDayString(parseDayString('2026-01-02'))).toBe('2026-01-02');
      expect(formatDay('2026-01-02', 'en-US')).toBe('January 2, 2026');
    });
  });

  it('keeps the calendar day in UTC-8, where a UTC-midnight parse drifts', () => {
    withTimeZone('America/Los_Angeles', () => {
      // The previous implementation: `new Date('2026-01-02')` is UTC midnight,
      // i.e. Jan 1 in the afternoon locally.
      expect(new Date('2026-01-02').getDate()).toBe(1);
      expect(localDayString(new Date('2026-01-02'))).toBe('2026-01-01');

      // parseDayString / formatDay stay on the requested day.
      expect(parseDayString('2026-01-02').getDate()).toBe(2);
      expect(localDayString(parseDayString('2026-01-02'))).toBe('2026-01-02');
      expect(localDayString(new Date(2026, 0, 2, 23, 30))).toBe('2026-01-02');
      expect(formatDay('2026-01-02', 'en-US')).toBe('January 2, 2026');
    });
  });
});
