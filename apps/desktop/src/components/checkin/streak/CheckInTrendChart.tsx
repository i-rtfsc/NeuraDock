import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Bar,
  CartesianGrid,
  ComposedChart,
  Line,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
  Legend,
} from 'recharts';
import { TrendDataPoint } from '@/hooks/useCheckInStreak';
import { useTranslation } from 'react-i18next';

interface CheckInTrendChartProps {
  data: TrendDataPoint[];
  title?: string;
}

export function CheckInTrendChart({ data, title }: CheckInTrendChartProps) {
  const { t, i18n } = useTranslation();
  const resolvedTitle = title ?? t('streaks.trendTitle');
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
  const formatSignedCurrency = (value: number) => {
    const formatted = currencyFormatter.format(Math.abs(value));
    if (value > 0) return `+${formatted}`;
    if (value < 0) return `-${formatted}`;
    return formatted;
  };

  const chartData = data.map((point) => ({
    date: dateFormatter.format(new Date(point.date)),
    income: point.total_income,
    increment: point.income_increment,
    balance: point.current_balance,
    checkedIn: point.is_checked_in,
  }));

  const incrementLabel = t('streaks.trendDailyIncrement');
  const totalIncomeLabel = t('streaks.trendTotalIncome');

  return (
    <Card>
      <CardHeader>
        <CardTitle>{resolvedTitle}</CardTitle>
      </CardHeader>
      <CardContent>
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height={320}>
            <ComposedChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
              <XAxis
                dataKey="date"
                className="text-xs"
                stroke="hsl(var(--muted-foreground))"
                tick={{ fill: 'hsl(var(--muted-foreground))' }}
              />
              <YAxis
                yAxisId="left"
                className="text-xs"
                stroke="hsl(var(--muted-foreground))"
                tick={{ fill: 'hsl(var(--muted-foreground))' }}
                tickFormatter={(value) => formatSignedCurrency(value as number)}
              />
              <YAxis
                yAxisId="right"
                orientation="right"
                className="text-xs"
                stroke="hsl(var(--muted-foreground))"
                tick={{ fill: 'hsl(var(--muted-foreground))' }}
                tickFormatter={(value) => currencyFormatter.format(value as number)}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: 'hsl(var(--card))',
                  border: '1px solid hsl(var(--border))',
                  borderRadius: '8px',
                }}
                labelStyle={{ color: 'hsl(var(--foreground))' }}
                formatter={(value: number, name: string) => {
                  if (name === incrementLabel) {
                    const formatted = detailedCurrencyFormatter.format(value);
                    return [value > 0 ? `+${formatted}` : formatted, incrementLabel];
                  }
                  if (name === totalIncomeLabel) {
                    return [detailedCurrencyFormatter.format(value), totalIncomeLabel];
                  }
                  return [value, name];
                }}
              />
              <Legend iconType="circle" wrapperStyle={{ fontSize: '12px' }} />
              <Bar
                yAxisId="left"
                dataKey="increment"
                name={incrementLabel}
                fill="hsl(var(--muted-foreground))"
                radius={[6, 6, 0, 0]}
              />
              <Line
                yAxisId="right"
                type="monotone"
                dataKey="income"
                name={totalIncomeLabel}
                stroke="hsl(var(--primary))"
                strokeWidth={2}
                dot={{ r: 2 }}
                activeDot={{ r: 4 }}
              />
            </ComposedChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex items-center justify-center h-[300px] text-muted-foreground">
            {t('streaks.trendNoData')}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
