import {
  Area,
  Bar,
  ComposedChart,
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
  const detailedCurrencyFormatter = new Intl.NumberFormat(i18n.language, {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });

  const chartData = data.map((point) => ({
    date: dateFormatter.format(new Date(point.date)),
    fullDate: new Date(point.date).toLocaleDateString(),
    total: point.total_quota,
    reward: point.income_increment,
    balance: point.current_balance,
    checkedIn: point.is_checked_in,
  }));

  if (chartData.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-[300px] text-muted-foreground/50 border border-dashed border-border/60 rounded-3xl bg-muted/5">
        <div className="mb-2 text-4xl opacity-20">📈</div>
        {t('streaks.trendNoData')}
      </div>
    );
  }

  return (
    <div className={cn("w-full h-[320px] pt-4", className)}>
      <ResponsiveContainer width="100%" height="100%">
        <ComposedChart data={chartData} margin={{ top: 10, right: 10, bottom: 0, left: 0 }}>
          <defs>
            <linearGradient id="trendGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.25}/>
              <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0}/>
            </linearGradient>
            <linearGradient id="barGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="hsl(var(--success))" stopOpacity={0.8}/>
              <stop offset="100%" stopColor="hsl(var(--success))" stopOpacity={0.4}/>
            </linearGradient>
          </defs>
          <CartesianGrid 
            strokeDasharray="4 4" 
            vertical={false} 
            stroke="hsl(var(--border))" 
            opacity={0.4}
          />
          <XAxis
            dataKey="date"
            stroke="hsl(var(--muted-foreground))"
            fontSize={11}
            fontWeight={600}
            tickLine={false}
            axisLine={false}
            minTickGap={40}
            dy={10}
          />
          <YAxis
            yAxisId="left"
            stroke="hsl(var(--muted-foreground))"
            fontSize={11}
            fontWeight={600}
            tickLine={false}
            axisLine={false}
            tickFormatter={(value) => `$${value}`}
            width={40}
          />
          <YAxis
            yAxisId="right"
            orientation="right"
            stroke="hsl(var(--success))"
            fontSize={10}
            fontWeight={700}
            tickLine={false}
            axisLine={false}
            tickFormatter={(value) => value > 0 ? `+$${value}` : ''}
            hide={true}
          />
          <Tooltip
            cursor={{ 
              stroke: 'hsl(var(--primary))', 
              strokeWidth: 2, 
              strokeDasharray: '6 6',
              opacity: 0.3
            }}
            content={({ active, payload }) => {
              if (active && payload && payload.length) {
                const item = payload[0].payload;
                return (
                  <div className="rounded-2xl border border-white/20 bg-card/80 backdrop-blur-xl shadow-2xl p-4 min-w-[180px] animate-in fade-in zoom-in duration-200">
                    <div className="flex items-center justify-between mb-3">
                      <p className="text-[10px] font-black uppercase tracking-widest text-muted-foreground/70">
                        {item.fullDate}
                      </p>
                      {item.checkedIn && (
                        <span className="flex h-2 w-2 rounded-full bg-success animate-pulse" />
                      )}
                    </div>
                    
                    <div className="space-y-3">
                      <div className="flex flex-col">
                        <span className="text-[10px] text-muted-foreground font-bold">{t('streaks.trendTotalQuota')}</span>
                        <span className="text-lg font-black tracking-tighter text-primary">
                          {detailedCurrencyFormatter.format(item.total)}
                        </span>
                      </div>
                      
                      <div className="grid grid-cols-2 gap-3 pt-2 border-t border-border/30">
                        <div className="flex flex-col">
                          <span className="text-[9px] text-muted-foreground font-bold uppercase">{t('streaks.trendDailyIncrement')}</span>
                          <span className={cn(
                            "text-xs font-black tracking-tight",
                            item.reward > 0 ? "text-success" : "text-muted-foreground/50"
                          )}>
                            {item.reward > 0 ? `+${detailedCurrencyFormatter.format(item.reward)}` : '-'}
                          </span>
                        </div>
                        <div className="flex flex-col">
                          <span className="text-[9px] text-muted-foreground font-bold uppercase">{t('streaks.trendBalance')}</span>
                          <span className="text-xs font-black tracking-tight text-foreground/80">
                            {detailedCurrencyFormatter.format(item.balance)}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              }
              return null;
            }}
          />
          
          {/* 背景面积图 - 整体趋势 */}
          <Area
            yAxisId="left"
            type="monotone"
            dataKey="total"
            stroke="hsl(var(--primary))"
            strokeWidth={4}
            fillOpacity={1}
            fill="url(#trendGradient)"
            animationDuration={1500}
            activeDot={{ 
              r: 6, 
              stroke: 'hsl(var(--background))', 
              strokeWidth: 3,
              fill: 'hsl(var(--primary))',
              className: "shadow-lg"
            }}
          />
          
          {/* 柱状图 - 每日奖励 (更加显眼) */}
          <Bar
            yAxisId="left"
            dataKey="reward"
            fill="url(#barGradient)"
            radius={[4, 4, 0, 0]}
            barSize={12}
            animationDuration={1200}
          />
        </ComposedChart>
      </ResponsiveContainer>
    </div>
  );
}

