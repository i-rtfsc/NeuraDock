import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Solar, Lunar } from 'lunar-javascript';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import { getHolidayInfo, fetchHolidayYear, hasApiData } from '@/data/holidays-cn';
import { ChevronLeft, ChevronRight, X, MapPin, Settings2, Droplets, Wind, Thermometer, CloudOff } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useCalendarStore } from '@/hooks/useCalendarStore';

const WEEKDAYS_ZH = ['一', '二', '三', '四', '五', '六', '日'];
const WEEKDAYS_EN = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const MONTHS_ZH = ['一月', '二月', '三月', '四月', '五月', '六月', '七月', '八月', '九月', '十月', '十一月', '十二月'];
const MONTHS_EN = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

interface DayInfo {
  solar: Solar;
  lunar: Lunar;
  lunarText: string;
  isCurrentMonth: boolean;
  isToday: boolean;
  isWeekend: boolean;
  dateStr: string;
  holiday?: { name: string; isOffDay: boolean };
}

function formatDateStr(solar: Solar): string {
  return `${solar.getYear()}-${String(solar.getMonth()).padStart(2, '0')}-${String(solar.getDay()).padStart(2, '0')}`;
}

function getLunarDisplayText(lunar: Lunar): string {
  const jieQi = lunar.getJieQi();
  if (jieQi) return jieQi;
  const festivals = lunar.getFestivals();
  if (festivals.length > 0) return festivals[0];
  if (lunar.getDay() === 1) return lunar.getMonthInChinese() + '月';
  return lunar.getDayInChinese();
}

function buildMonthDays(year: number, month: number): DayInfo[] {
  const todayStr = formatDateStr(Solar.fromDate(new Date()));
  const firstDay = Solar.fromYmd(year, month, 1);
  let startWeekday = firstDay.getWeek();
  startWeekday = startWeekday === 0 ? 6 : startWeekday - 1;

  const daysInMonth = month === 12
    ? 31
    : Solar.fromYmd(year, month + 1 > 12 ? 1 : month + 1, 1).subtract(firstDay);

  const days: DayInfo[] = [];
  const addDay = (solar: Solar, isCurrentMonth: boolean) => {
    const lunar = Lunar.fromSolar(solar);
    const dateStr = formatDateStr(solar);
    days.push({
      solar, lunar, dateStr,
      lunarText: getLunarDisplayText(lunar),
      isCurrentMonth,
      isToday: dateStr === todayStr,
      isWeekend: solar.getWeek() === 0 || solar.getWeek() === 6,
      holiday: getHolidayInfo(dateStr),
    });
  };

  for (let i = startWeekday - 1; i >= 0; i--) addDay(firstDay.next(-i - 1), false);
  for (let d = 0; d < daysInMonth; d++) addDay(firstDay.next(d), true);
  const remaining = 42 - days.length;
  const lastDay = firstDay.next(daysInMonth - 1);
  for (let i = 1; i <= remaining; i++) addDay(lastDay.next(i), false);
  return days;
}

// --- Weather ---
interface DayWeather {
  maxTemp: string;
  minTemp: string;
  description: string;
  icon: string;
  humidity: string;
  wind: string;
}

interface WeatherResult {
  location: string;
  /** Current conditions (today only) */
  currentTemp: string;
  currentFeelsLike: string;
  /** Forecast indexed by date string */
  forecast: Record<string, DayWeather>;
  loading: boolean;
}

function weatherCodeToEmoji(code: string): string {
  const c = parseInt(code, 10);
  if (c === 113) return '☀️';
  if (c === 116) return '⛅';
  if (c === 119 || c === 122) return '☁️';
  if ([176, 263, 266, 293, 296, 299, 302, 305, 308, 311, 314, 353, 356, 359].includes(c)) return '🌧️';
  if ([179, 182, 185, 227, 230, 281, 284, 317, 320, 323, 326, 329, 332, 335, 338, 350, 362, 365, 368, 371, 374, 377, 392, 395].includes(c)) return '❄️';
  if ([200, 386, 389].includes(c)) return '⛈️';
  if ([143, 248, 260].includes(c)) return '🌫️';
  return '🌤️';
}

function useWeather(city: string, isZh: boolean): WeatherResult {
  const [result, setResult] = useState<WeatherResult>({
    location: '', currentTemp: '', currentFeelsLike: '',
    forecast: {}, loading: true,
  });

  useEffect(() => {
    let cancelled = false;
    setResult(prev => ({ ...prev, loading: true }));

    const loc = city.trim() || '';
    const langParam = isZh ? '&lang=zh' : '';
    const url = loc
      ? `https://wttr.in/${encodeURIComponent(loc)}?format=j1${langParam}`
      : `https://wttr.in/?format=j1${langParam}`;

    (async () => {
      try {
        const res = await fetch(url, { headers: { Accept: 'application/json' } });
        if (!res.ok) throw new Error();
        const data = await res.json();
        if (cancelled) return;

        const current = data.current_condition?.[0];
        const area = data.nearest_area?.[0];
        const region = area?.region?.[0]?.value || '';
        const areaName = area?.areaName?.[0]?.value || '';
        const location = isZh ? (loc || region || areaName) : (region || areaName);

        // Build forecast map from weather array (3 days)
        const forecast: Record<string, DayWeather> = {};
        for (const day of data.weather || []) {
          const noon = day.hourly?.[4]; // index 4 = 1200h
          if (!noon) continue;
          const desc = noon.lang_zh?.[0]?.value || noon.weatherDesc?.[0]?.value || '';
          forecast[day.date] = {
            maxTemp: day.maxtempC,
            minTemp: day.mintempC,
            description: desc,
            icon: weatherCodeToEmoji(noon.weatherCode),
            humidity: noon.humidity,
            wind: `${noon.windspeedKmph} km/h`,
          };
        }

        setResult({
          location,
          currentTemp: current?.temp_C || '',
          currentFeelsLike: current?.FeelsLikeC || '',
          forecast,
          loading: false,
        });
      } catch {
        if (!cancelled) setResult(prev => ({ ...prev, loading: false }));
      }
    })();
    return () => { cancelled = true; };
  }, [city, isZh]);

  return result;
}

// --- Combined Day Info + Weather Card ---
function DayInfoCard({
  day, isZh, isSelected, onClose, weather, city, onCityChange,
}: {
  day: DayInfo; isZh: boolean; isSelected: boolean; onClose: () => void;
  weather: WeatherResult; city: string; onCityChange: (city: string) => void;
}) {
  const lunar = day.lunar;
  const solar = day.solar;
  const solarFestivals = solar.getFestivals();
  const lunarFestivals = lunar.getFestivals();
  const otherFestivals = lunar.getOtherFestivals();
  const allFestivals = [...solarFestivals, ...lunarFestivals, ...otherFestivals];
  const yi = lunar.getDayYi();
  const ji = lunar.getDayJi();

  // Weather for this specific day
  const dayWeather = weather.forecast[day.dateStr];
  const isToday = day.isToday;

  // City editing
  const [editing, setEditing] = useState(false);
  const [cityInput, setCityInput] = useState(city);
  const inputRef = useRef<HTMLInputElement>(null);
  const editAreaRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (editing) setTimeout(() => inputRef.current?.focus(), 50);
  }, [editing]);

  useEffect(() => {
    if (!editing) return;
    const handler = (e: MouseEvent) => {
      if (editAreaRef.current && !editAreaRef.current.contains(e.target as Node)) {
        onCityChange(cityInput.trim());
        setEditing(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [editing, cityInput, onCityChange]);

  return (
    <Card className="card-vivid group relative overflow-visible bg-gradient-to-br from-primary/5 via-background to-sky-500/5 dark:from-primary/10 dark:to-sky-500/10">
      <div className="p-4 flex gap-4">
        {/* Left 2/3: date info */}
        <div className="flex-[2] min-w-0 flex gap-3">
          {/* Date block - vertically centered */}
          <div className="flex items-center shrink-0">
            <div className="flex flex-col items-center justify-center w-12 h-12 rounded-xl bg-primary text-primary-foreground shadow-md shadow-primary/20">
              <span className="text-lg font-black leading-none">{solar.getDay()}</span>
              <span className="text-[9px] font-medium opacity-80 mt-0.5">
                {isZh ? `${solar.getMonth()}月` : MONTHS_EN[solar.getMonth() - 1].slice(0, 3)}
              </span>
            </div>
          </div>

          {/* All text content aligned */}
          <div className="flex-1 min-w-0 space-y-1.5">
            {/* Lunar + meta */}
            <div className="flex items-center gap-2">
              <span className="text-sm font-bold text-foreground">
                {lunar.getMonthInChinese()}月{lunar.getDayInChinese()}
              </span>
              <span className="text-[11px] text-muted-foreground">
                {lunar.getDayInGanZhi()}日 | {lunar.getYearShengXiao()}年 | {solar.getXingZuo()}{isZh ? '座' : ''}
              </span>
              {isSelected && (
                <button onClick={onClose} className="text-muted-foreground hover:text-primary transition-colors ml-auto">
                  <X className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
            {/* Tags */}
            <div className="flex items-center gap-1 flex-wrap">
                {day.isToday && isSelected && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-primary/10 text-primary font-bold border border-primary/20">
                    {isZh ? '今天' : 'Today'}
                  </span>
                )}
                {lunar.getJieQi() && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-medium border border-emerald-500/20">
                    {lunar.getJieQi()}
                  </span>
                )}
                {allFestivals.map((f) => (
                  <span key={f} className="text-[10px] px-1.5 py-0.5 rounded-full bg-red-500/10 text-red-600 dark:text-red-400 font-medium border border-red-500/20">{f}</span>
                ))}
                {day.holiday && (
                  <span className={cn(
                    'text-[10px] px-1.5 py-0.5 rounded-full font-medium border',
                    day.holiday.isOffDay
                      ? 'bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20'
                      : 'bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20'
                  )}>
                    {day.holiday.name}
                  </span>
                )}
                <span className="text-[10px] text-muted-foreground/70 ml-1">
                  {lunar.getDayTianShen()} | 冲{lunar.getDayChongDesc()} 煞{lunar.getDaySha()}
                </span>
              </div>
              {/* Yi / Ji */}
              <div className="space-y-0.5 text-[11px]">
                <div>
                  <span className="text-green-600 dark:text-green-400 font-bold mr-1">{isZh ? '宜' : 'Do'}:</span>
                  <span className="text-muted-foreground">{yi.join(' ')}</span>
                </div>
                <div>
                  <span className="text-red-500 dark:text-red-400 font-bold mr-1">{isZh ? '忌' : "Don't"}:</span>
                  <span className="text-muted-foreground">{ji.join(' ')}</span>
                </div>
              </div>
            </div>
          </div>

        {/* Divider */}
        <div className="w-px bg-border/30 shrink-0 my-1" />

        {/* Right 1/3: weather */}
        <div className="flex-1 min-w-0 flex flex-col justify-between">
          {/* Location */}
          <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
            <MapPin className="h-3 w-3 shrink-0" />
            {editing ? (
              <div ref={editAreaRef} className="flex items-center" data-tauri-no-drag>
                <form onSubmit={(e) => { e.preventDefault(); onCityChange(cityInput.trim()); setEditing(false); }}>
                  <Input
                    ref={inputRef}
                    value={cityInput}
                    onChange={(e) => setCityInput(e.target.value)}
                    placeholder={isZh ? '城市名' : 'City'}
                    className="h-5 text-[11px] w-24 px-1.5 py-0"
                    onKeyDown={(e) => { if (e.key === 'Escape') { setCityInput(city); setEditing(false); } }}
                  />
                </form>
              </div>
            ) : (
              <button
                onClick={(e) => { e.stopPropagation(); setCityInput(city); setEditing(true); }}
                className="font-medium hover:text-primary transition-colors cursor-pointer flex items-center gap-0.5"
              >
                {weather.location || city || (isZh ? '设置城市' : 'Set city')}
                <Settings2 className="h-2.5 w-2.5 opacity-0 group-hover:opacity-50 transition-opacity" />
              </button>
            )}
          </div>

          {weather.loading ? (
            <div className="space-y-2 animate-pulse mt-2">
              <div className="h-6 w-16 bg-muted/50 rounded" />
              <div className="h-3 w-20 bg-muted/30 rounded" />
            </div>
          ) : dayWeather ? (
            <div className="mt-1">
              <div className="flex items-center gap-2">
                <span className="text-2xl select-none">{dayWeather.icon}</span>
                <div>
                  <div className="flex items-baseline gap-0.5">
                    {isToday && weather.currentTemp ? (
                      <>
                        <span className="text-xl font-black tracking-tight text-foreground">{weather.currentTemp}</span>
                        <span className="text-xs text-muted-foreground">°C</span>
                      </>
                    ) : (
                      <>
                        <span className="text-base font-bold text-foreground">{dayWeather.maxTemp}</span>
                        <span className="text-[11px] text-muted-foreground">/</span>
                        <span className="text-base font-bold text-muted-foreground">{dayWeather.minTemp}</span>
                        <span className="text-xs text-muted-foreground">°C</span>
                      </>
                    )}
                  </div>
                  <div className="text-[11px] text-muted-foreground">{dayWeather.description}</div>
                </div>
              </div>
              <div className="flex items-center gap-2 text-[10px] text-muted-foreground mt-1.5">
                {isToday && weather.currentFeelsLike && (
                  <span className="flex items-center gap-0.5"><Thermometer className="h-2.5 w-2.5" />{weather.currentFeelsLike}°</span>
                )}
                <span className="flex items-center gap-0.5"><Droplets className="h-2.5 w-2.5" />{dayWeather.humidity}%</span>
                <span className="flex items-center gap-0.5"><Wind className="h-2.5 w-2.5" />{dayWeather.wind}</span>
              </div>
            </div>
          ) : (
            <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground mt-2">
              <CloudOff className="h-3.5 w-3.5 shrink-0" />
              <span>{isZh ? '暂无天气数据' : 'No weather data'}</span>
            </div>
          )}
        </div>
      </div>
    </Card>
  );
}

// --- Settings Popover ---
function CalendarSettings({ isZh }: { isZh: boolean }) {
  const { tianapiKey, setTianapiKey } = useCalendarStore();
  const [open, setOpen] = useState(false);
  const [keyInput, setKeyInput] = useState(tianapiKey);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  const handleSave = () => {
    setTianapiKey(keyInput.trim());
    setOpen(false);
  };

  return (
    <div className="relative" ref={ref}>
      <Button variant="ghost" size="icon-sm" onClick={() => setOpen(!open)} title={isZh ? '日历设置' : 'Calendar Settings'}>
        <Settings2 className="h-4 w-4" />
      </Button>
      {open && (
        <div className="absolute right-0 top-full mt-1 z-50 w-[320px] bg-card border border-border rounded-xl p-4 shadow-lg animate-in fade-in slide-in-from-top-1 duration-150 space-y-3" data-tauri-no-drag>
          <div className="text-sm font-bold">{isZh ? '日历设置' : 'Calendar Settings'}</div>

          <div className="space-y-1.5">
            <label className="text-xs font-medium text-muted-foreground">{isZh ? '天行 API Key (节假日)' : 'Tianapi Key (Holidays)'}</label>
            <Input
              type="password"
              placeholder="e1bd0fb9..."
              value={keyInput}
              onChange={(e) => setKeyInput(e.target.value)}
              className="h-8 text-xs"
            />
            <p className="text-[10px] text-muted-foreground">
              {isZh ? '用于获取准确的节假日调休数据' : 'For accurate holiday data from tianapi.com'}
            </p>
          </div>

          <Button size="sm" className="w-full h-8" onClick={handleSave}>
            {isZh ? '保存' : 'Save'}
          </Button>
        </div>
      )}
    </div>
  );
}

// --- Main Component ---
export function CalendarGrid() {
  const { i18n } = useTranslation();
  const isZh = i18n.language.startsWith('zh');
  const weekdays = isZh ? WEEKDAYS_ZH : WEEKDAYS_EN;
  const monthNames = isZh ? MONTHS_ZH : MONTHS_EN;

  const { tianapiKey, weatherCity, setWeatherCity } = useCalendarStore();
  const today = new Date();
  const [year, setYear] = useState(today.getFullYear());
  const [month, setMonth] = useState(today.getMonth() + 1);
  const [selectedDay, setSelectedDay] = useState<DayInfo | null>(null);
  const [holidayVersion, setHolidayVersion] = useState(0);

  // Fetch holiday data from API
  useEffect(() => {
    if (!tianapiKey) return;
    const yearsToFetch = new Set([year]);
    if (month === 1) yearsToFetch.add(year - 1);
    if (month === 12) yearsToFetch.add(year + 1);

    let changed = false;
    const promises = [...yearsToFetch]
      .filter(y => !hasApiData(y))
      .map(y => fetchHolidayYear(y, tianapiKey).then(() => { changed = true; }));

    if (promises.length > 0) {
      Promise.all(promises).then(() => {
        if (changed) setHolidayVersion(v => v + 1);
      });
    }
  }, [year, month, tianapiKey]);

  const days = useMemo(() => buildMonthDays(year, month), [year, month, holidayVersion]);
  const weather = useWeather(weatherCity, isZh);

  const headerLunar = useMemo(() => {
    const lunar = Lunar.fromSolar(Solar.fromYmd(year, month, 15));
    return `${lunar.getYearInGanZhi()}年 (${lunar.getYearShengXiao()})`;
  }, [year, month]);

  const goPrev = useCallback(() => {
    if (month === 1) { setYear(y => y - 1); setMonth(12); }
    else setMonth(m => m - 1);
    setSelectedDay(null);
  }, [month]);

  const goNext = useCallback(() => {
    if (month === 12) { setYear(y => y + 1); setMonth(1); }
    else setMonth(m => m + 1);
    setSelectedDay(null);
  }, [month]);

  const goToday = useCallback(() => {
    const now = new Date();
    setYear(now.getFullYear());
    setMonth(now.getMonth() + 1);
    setSelectedDay(null);
  }, []);

  const yearOptions = useMemo(() => {
    const years: number[] = [];
    for (let y = year - 10; y <= year + 10; y++) years.push(y);
    return years;
  }, [year]);

  const todayDayInfo = useMemo<DayInfo>(() => {
    const solar = Solar.fromDate(new Date());
    const lunar = Lunar.fromSolar(solar);
    const dateStr = formatDateStr(solar);
    return {
      solar, lunar, dateStr,
      lunarText: getLunarDisplayText(lunar),
      isCurrentMonth: true,
      isToday: true,
      isWeekend: solar.getWeek() === 0 || solar.getWeek() === 6,
      holiday: getHolidayInfo(dateStr),
    };
  }, [holidayVersion]);

  const displayDay = selectedDay || todayDayInfo;

  return (
    <div className="w-full space-y-5">
      {/* Day Info + Weather Card */}
      <div className="pt-1">
        <DayInfoCard
          day={displayDay}
          isZh={isZh}
          isSelected={!!selectedDay}
          onClose={() => setSelectedDay(null)}
          weather={weather}
          city={weatherCity}
          onCityChange={setWeatherCity}
        />
      </div>

      {/* Calendar Card */}
      <Card className="card-vivid overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border/30 bg-muted/20">
          <div className="flex items-center gap-2">
            <Select value={String(year)} onValueChange={(v) => { setYear(Number(v)); setSelectedDay(null); }}>
              <SelectTrigger className="w-[90px] h-8 text-sm font-bold border-border/40 bg-background/60">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {yearOptions.map((y) => (
                  <SelectItem key={y} value={String(y)}>{y}</SelectItem>
                ))}
              </SelectContent>
            </Select>

            <Select value={String(month)} onValueChange={(v) => { setMonth(Number(v)); setSelectedDay(null); }}>
              <SelectTrigger className="w-[100px] h-8 text-sm font-bold border-border/40 bg-background/60">
                <SelectValue>{monthNames[month - 1]}</SelectValue>
              </SelectTrigger>
              <SelectContent>
                {monthNames.map((name, i) => (
                  <SelectItem key={i} value={String(i + 1)}>{name}</SelectItem>
                ))}
              </SelectContent>
            </Select>

            <span className="text-xs text-muted-foreground hidden sm:inline ml-1">{headerLunar}</span>
          </div>

          <div className="flex items-center gap-1">
            <CalendarSettings isZh={isZh} />
            <Button variant="outline" size="sm" onClick={goToday} className="h-8 border-border/40">
              {isZh ? '今天' : 'Today'}
            </Button>
            <Button variant="ghost" size="icon-sm" onClick={goPrev} className="rounded-full">
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon-sm" onClick={goNext} className="rounded-full">
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Weekday headers */}
        <div className="grid grid-cols-7 bg-muted/10">
          {weekdays.map((day, i) => (
            <div
              key={day}
              className={cn(
                'text-center text-[11px] font-bold py-2.5 tracking-wide uppercase',
                i >= 5 ? 'text-red-400' : 'text-muted-foreground/60'
              )}
            >
              {day}
            </div>
          ))}
        </div>

        {/* Calendar grid */}
        <div className="grid grid-cols-7">
          {days.map((day, i) => {
            const isLunarFestival = day.lunar.getFestivals().length > 0;
            const isJieQi = !!day.lunar.getJieQi();
            const isHolidayOff = day.holiday?.isOffDay === true;
            const isWorkdaySwap = day.holiday?.isOffDay === false;
            const isSelected = selectedDay?.dateStr === day.dateStr;
            const row = Math.floor(i / 7);
            const isLastRow = row === Math.floor((days.length - 1) / 7);

            return (
              <button
                key={i}
                onClick={() => setSelectedDay(isSelected ? null : day)}
                className={cn(
                  'relative flex flex-col items-center justify-center py-2 min-h-[72px] transition-all duration-150 cursor-pointer',
                  'border-b border-r border-border/15',
                  i % 7 === 6 && 'border-r-0',
                  isLastRow && 'border-b-0',
                  !day.isCurrentMonth && 'opacity-25',
                  day.isCurrentMonth && 'hover:bg-primary/5',
                  day.isToday && !isSelected && 'bg-primary/8',
                  isSelected && 'bg-primary/12 shadow-inner',
                )}
              >
                {day.holiday && (
                  <span className={cn(
                    'absolute top-1 right-1 text-[8px] leading-none w-4 h-4 flex items-center justify-center rounded-full font-bold',
                    isHolidayOff
                      ? 'bg-green-500/20 text-green-600 dark:text-green-400'
                      : 'bg-orange-500/20 text-orange-600 dark:text-orange-400'
                  )}>
                    {isHolidayOff ? (isZh ? '休' : 'O') : (isZh ? '班' : 'W')}
                  </span>
                )}

                <span className={cn(
                  'text-base font-bold leading-none transition-colors',
                  day.isToday && 'w-7 h-7 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-sm',
                  !day.isToday && day.isWeekend && !isWorkdaySwap && 'text-red-500 dark:text-red-400',
                  !day.isToday && isHolidayOff && 'text-green-600 dark:text-green-400',
                  !day.isToday && !day.isWeekend && !isHolidayOff && 'text-foreground',
                )}>
                  {day.solar.getDay()}
                </span>

                <span className={cn(
                  'text-[10px] leading-tight mt-1 truncate max-w-full px-1',
                  isLunarFestival
                    ? 'text-red-500 dark:text-red-400 font-bold'
                    : isJieQi
                      ? 'text-emerald-600 dark:text-emerald-400 font-bold'
                      : 'text-muted-foreground/70',
                )}>
                  {day.lunarText}
                </span>
              </button>
            );
          })}
        </div>
      </Card>
    </div>
  );
}
