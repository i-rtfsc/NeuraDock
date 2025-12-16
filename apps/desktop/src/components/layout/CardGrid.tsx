import { cn } from '@/lib/utils';
import { ReactNode } from 'react';

/**
 * CardGrid - 统一的卡片网格布局组件
 *
 * 使用 design tokens 统一所有卡片网格的间距和响应式断点
 *
 * @example
 * ```tsx
 * <CardGrid variant="cards">
 *   <Card>...</Card>
 *   <Card>...</Card>
 * </CardGrid>
 * ```
 */

export type CardGridVariant = 'cards' | 'stats' | 'bento' | 'compact';

interface CardGridProps {
  children: ReactNode;
  className?: string;
  /**
   * 网格变体类型
   * - cards: 标准卡片网格 (1/2/3/4列)
   * - stats: 统计卡片网格 (1/2/3/4列)
   * - bento: Bento风格网格 (1/2/4列)
   * - compact: 紧凑网格 (更小的gap)
   */
  variant?: CardGridVariant;
  /**
   * 自定义响应式列数
   * 格式: "grid-cols-1 md:grid-cols-2 lg:grid-cols-3"
   */
  cols?: string;
}

const gridVariants: Record<CardGridVariant, string> = {
  // 标准卡片网格：mobile 1列，tablet 2列，desktop 3列，wide 4列
  cards: 'grid-cols-1 md:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4',

  // 统计卡片网格：类似cards但在xl断点就变成4列
  stats: 'grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4',

  // Bento风格网格：mobile 1列，tablet 2列，desktop 4列（适合仪表板）
  bento: 'grid-cols-1 md:grid-cols-2 lg:grid-cols-4',

  // 紧凑网格：使用更小的gap
  compact: 'grid-cols-1 md:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4',
};

export function CardGrid({
  children,
  className,
  variant = 'cards',
  cols
}: CardGridProps) {
  const gap = variant === 'compact' ? 'gap-card-gap-sm' : 'gap-card-gap';
  const gridCols = cols || gridVariants[variant];

  return (
    <div className={cn(
      'grid',
      gridCols,
      gap,
      className
    )}>
      {children}
    </div>
  );
}

/**
 * StatsCardGrid - 统计卡片专用网格
 * 预设了统一的padding和样式
 */
export function StatsCardGrid({
  children,
  className
}: { children: ReactNode; className?: string }) {
  return (
    <CardGrid variant="stats" className={className}>
      {children}
    </CardGrid>
  );
}

/**
 * BentoGrid - Bento风格网格布局
 * 适合仪表板等需要灵活布局的场景
 */
export function BentoGrid({
  children,
  className
}: { children: ReactNode; className?: string }) {
  return (
    <CardGrid variant="bento" className={className}>
      {children}
    </CardGrid>
  );
}
