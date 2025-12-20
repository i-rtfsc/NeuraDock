import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import type { TrendDataPoint } from '@/lib/tauri-commands';
import { useTranslation } from 'react-i18next';
import { cn } from '@/lib/utils';

interface CheckInTrendChartProps {
  data: TrendDataPoint[];
  className?: string;
}

export function CheckInTrendChart({ data, className }: CheckInTrendChartProps) {
  const { t, i18n } = useTranslation();
  const dateFormatter = new Intl.DateTimeFormat(i18n.language, {
    month: 'short',
    day: 'numeric',
  });
  const currencyFormatter = new Intl.NumberFormat(i18n.language, {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 0,
  });
  const detailedCurrencyFormatter = new Intl.NumberFormat(i18n.language, {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });

  const chartData = data.map((point) => ({
    date: dateFormatter.format(new Date(point.date)),
    fullDate: new Date(point.date).toLocaleDateString(),
    income: point.total_income,
    increment: point.income_increment,
    balance: point.current_balance,
    checkedIn: point.is_checked_in,
  }));

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-[300px] text-muted-foreground">
        {t('streaks.trendNoData')}
      </div>
    );
  }

  return (
    <div className={cn("w-full h-[300px]", className)}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData}>
          <defs>
            <linearGradient id="colorIncome" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.3}/>
              <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0}/>
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="hsl(var(--border))" />
          <XAxis
            dataKey="date"
            stroke="hsl(var(--muted-foreground))"
            fontSize={12}
            tickLine={false}
            axisLine={false}
            minTickGap={30}
          />
          <YAxis
            stroke="hsl(var(--muted-foreground))"
            fontSize={12}
            tickLine={false}
            axisLine={false}
            tickFormatter={(value) => currencyFormatter.format(value)}
          />
          <Tooltip
            cursor={{ stroke: 'hsl(var(--muted-foreground))', strokeWidth: 1, strokeDasharray: '4 4' }}
            content={({ active, payload }) => {
              if (active && payload && payload.length) {
                const data = payload[0].payload;
                return (
                  <div className="rounded-xl border border-border/50 bg-popover/95 backdrop-blur-md shadow-xl p-3 text-xs">
                    <p className="font-semibold mb-2 text-foreground">{data.fullDate}</p>
                    <div className="space-y-1.5">
                      <div className="flex items-center justify-between gap-4">
                        <span className="text-muted-foreground">{t('streaks.trendTotalIncome')}:</span>
                        <span className="font-mono font-semibold text-primary">
                          {detailedCurrencyFormatter.format(data.income)}
                        </span>
                      </div>
                      {data.increment > 0 && (
                        <div className="flex items-center justify-between gap-4">
                          <span className="text-muted-foreground">{t('streaks.trendDailyIncrement')}:</span>
                          <span className="font-mono font-medium text-green-600 dark:text-green-400">
                            +{detailedCurrencyFormatter.format(data.increment)}
                          </span>
                        </div>
                      )}
                      <div className="flex items-center justify-between gap-4 pt-1 border-t border-border/50">
                        <span className="text-muted-foreground">{t('streaks.trendBalance')}:</span>
                        <span className="font-mono font-medium">
                          {detailedCurrencyFormatter.format(data.balance)}
                        </span>
                      </div>
                    </div>
                  </div>
                );
              }
              return null;
            }}
          />
          <Area
            type="monotone"
            dataKey="income"
            stroke="hsl(var(--primary))"
            strokeWidth={3}
            fillOpacity={1}
            fill="url(#colorIncome)"
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
