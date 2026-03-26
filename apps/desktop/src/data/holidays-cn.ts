/**
 * 中国法定节假日数据
 *
 * 优先从 tianapi.com 获取，失败时回退到内置数据。
 * 内置数据来源:
 *   2025: https://www.gov.cn/zhengce/content/202411/content_6986382.htm
 *   2026: http://politics.people.com.cn/n1/2025/1104/c1001-40596715.html
 */

export interface HolidayInfo {
  name: string;
  /** true = 放假, false = 调休上班 */
  isOffDay: boolean;
}

// --- Built-in fallback data ---

function buildBuiltinData(): Record<string, HolidayInfo> {
  const data: Record<string, HolidayInfo> = {};

  const add = (dates: string, name: string, isOffDay: boolean) => {
    for (const d of dates.split('|')) data[d] = { name, isOffDay };
  };

  // 2025
  add('2025-01-01', '元旦', true);
  add('2025-01-26', '春节(调休)', false);
  add('2025-01-28|2025-01-29|2025-01-30|2025-01-31|2025-02-01|2025-02-02|2025-02-03|2025-02-04', '春节', true);
  add('2025-02-08', '春节(调休)', false);
  add('2025-04-04|2025-04-05|2025-04-06', '清明节', true);
  add('2025-04-27', '劳动节(调休)', false);
  add('2025-05-01|2025-05-02|2025-05-03|2025-05-04|2025-05-05', '劳动节', true);
  add('2025-05-31|2025-06-01|2025-06-02', '端午节', true);
  add('2025-09-28', '国庆节(调休)', false);
  add('2025-10-01|2025-10-02|2025-10-03|2025-10-05|2025-10-06|2025-10-07|2025-10-08', '国庆节', true);
  add('2025-10-04', '中秋节', true);
  add('2025-10-11', '国庆节(调休)', false);

  // 2026
  add('2026-01-01|2026-01-02|2026-01-03', '元旦', true);
  add('2026-01-04', '元旦(调休)', false);
  add('2026-02-14', '春节(调休)', false);
  add('2026-02-15|2026-02-16|2026-02-17|2026-02-18|2026-02-19|2026-02-20|2026-02-21|2026-02-22|2026-02-23', '春节', true);
  add('2026-02-28', '春节(调休)', false);
  add('2026-04-04|2026-04-05|2026-04-06', '清明节', true);
  add('2026-05-01|2026-05-02|2026-05-03|2026-05-04|2026-05-05', '劳动节', true);
  add('2026-05-09', '劳动节(调休)', false);
  add('2026-06-19|2026-06-20|2026-06-21', '端午节', true);
  add('2026-09-25|2026-09-26|2026-09-27', '中秋节', true);
  add('2026-09-20', '国庆节(调休)', false);
  add('2026-10-01|2026-10-02|2026-10-03|2026-10-04|2026-10-05|2026-10-06|2026-10-07', '国庆节', true);
  add('2026-10-10', '国庆节(调休)', false);

  return data;
}

const builtinHolidays = buildBuiltinData();

// --- API-fetched data cache ---

const apiCache: Record<string, Record<string, HolidayInfo>> = {};
const fetchingYears = new Set<string>();

/**
 * Parse tianapi response into HolidayInfo map
 */
function parseTianapiResponse(data: {
  code: number;
  result?: {
    list: Array<{
      name: string;
      vacation: string; // "2026-01-01|2026-01-02|..."
      remark: string;   // "2026-01-04|..." (调休上班日)
    }>;
  };
}): Record<string, HolidayInfo> | null {
  if (data.code !== 200 || !data.result?.list) return null;

  const result: Record<string, HolidayInfo> = {};
  for (const item of data.result.list) {
    // 放假日
    if (item.vacation) {
      for (const d of item.vacation.split('|')) {
        if (d.trim()) result[d.trim()] = { name: item.name, isOffDay: true };
      }
    }
    // 调休上班日
    if (item.remark) {
      for (const d of item.remark.split('|')) {
        if (d.trim()) result[d.trim()] = { name: `${item.name}(调休)`, isOffDay: false };
      }
    }
  }
  return Object.keys(result).length > 0 ? result : null;
}

/**
 * Fetch holiday data for a year from tianapi.com
 */
export async function fetchHolidayYear(year: number, apiKey: string): Promise<void> {
  const yearStr = String(year);
  if (apiCache[yearStr] || fetchingYears.has(yearStr) || !apiKey) return;

  fetchingYears.add(yearStr);
  try {
    const url = `https://apis.tianapi.com/jiejiari/index?key=${encodeURIComponent(apiKey)}&date=${year}&type=1`;
    const res = await fetch(url);
    if (!res.ok) return;
    const data = await res.json();
    const parsed = parseTianapiResponse(data);
    if (parsed) {
      apiCache[yearStr] = parsed;
    }
  } catch {
    // silently fail, use builtin data
  } finally {
    fetchingYears.delete(yearStr);
  }
}

/**
 * Get holiday info for a date string (YYYY-MM-DD)
 * Prefers API data, falls back to builtin
 */
export function getHolidayInfo(dateStr: string): HolidayInfo | undefined {
  const year = dateStr.slice(0, 4);
  return apiCache[year]?.[dateStr] ?? builtinHolidays[dateStr];
}

/**
 * Check if API data is loaded for a year
 */
export function hasApiData(year: number): boolean {
  return !!apiCache[String(year)];
}
