import {
  Area,
  Bar,
  CartesianGrid,
  ComposedChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
  Legend,
} from 'recharts';
import { TrendDataPoint } from '@/hooks/useCheckInStreak';
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

  const incrementLabel = t('streaks.trendDailyIncrement');
  const totalIncomeLabel = t('streaks.trendTotalIncome');

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-[300px] text-muted-foreground">
        {t('streaks.trendNoData')}
      </div>
    );
  }

  return (
    <div className={cn("w-full h-[350px]", className)}>
      <ResponsiveContainer width="100%" height="100%">
        <ComposedChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id="colorIncome" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#f97316" stopOpacity={0.3} />
              <stop offset="95%" stopColor="#f97316" stopOpacity={0} />
            </linearGradient>
            <linearGradient id="colorIncrement" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#22c55e" stopOpacity={0.8} />
              <stop offset="95%" stopColor="#22c55e" stopOpacity={0.4} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="hsl(var(--border))" opacity={0.4} />
          <XAxis
            dataKey="date"
            axisLine={false}
            tickLine={false}
            tick={{ fill: 'hsl(var(--muted-foreground))', fontSize: 12 }}
            dy={10}
            minTickGap={30}
          />
          <YAxis
            yAxisId="left"
            orientation="left"
            axisLine={false}
            tickLine={false}
            tick={{ fill: 'hsl(var(--muted-foreground))', fontSize: 12 }}
            tickFormatter={(value) => currencyFormatter.format(value)}
          />
          <YAxis
            yAxisId="right"
            orientation="right"
            axisLine={false}
            tickLine={false}
            tick={{ fill: 'hsl(var(--muted-foreground))', fontSize: 12 }}
            tickFormatter={(value) => currencyFormatter.format(value)}
          />
          <Tooltip
            cursor={{ stroke: 'hsl(var(--muted-foreground))', strokeWidth: 1, strokeDasharray: '4 4' }}
            content={({ active, payload }) => {
              if (active && payload && payload.length) {
                return (
                  <div className="rounded-xl border border-border/50 bg-background/95 backdrop-blur-md shadow-xl p-3 text-xs">
                    <p className="font-semibold mb-2 text-foreground">{payload[0].payload.fullDate}</p>
                    <div className="space-y-1">
                      {payload.map((entry: any) => (
                        <div key={entry.name} className="flex items-center gap-2">
                          <div 
                            className="w-2 h-2 rounded-full" 
                            style={{ backgroundColor: entry.color }}
                          />
                          <span className="text-muted-foreground">{entry.name}:</span>
                          <span className="font-mono font-medium">
                            {detailedCurrencyFormatter.format(entry.value)}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                );
              }
              return null;
            }}
          />
          <Legend 
            verticalAlign="top" 
            height={36} 
            iconType="circle"
            formatter={(value) => <span className="text-xs font-medium text-muted-foreground ml-1">{value}</span>}
          />
          <Bar
            yAxisId="left"
            dataKey="increment"
            name={incrementLabel}
            fill="url(#colorIncrement)"
            radius={[4, 4, 0, 0]}
            barSize={20}
          />
          <Area
            yAxisId="right"
            type="monotone"
            dataKey="income"
            name={totalIncomeLabel}
            stroke="#f97316"
            strokeWidth={3}
            fillOpacity={1}
            fill="url(#colorIncome)"
          />
        </ComposedChart>
      </ResponsiveContainer>
    </div>
  );
}
