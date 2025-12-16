import { cn } from '@/lib/utils';
import { ReactNode } from 'react';

/**
 * PageContent - 统一的页面内容容器
 *
 * 提供一致的最大宽度约束、水平居中和垂直间距
 *
 * @example
 * ```tsx
 * <PageContainer title="Dashboard">
 *   <PageContent maxWidth="lg">
 *     <CardGrid variant="bento">
 *       <Card>...</Card>
 *     </CardGrid>
 *   </PageContent>
 * </PageContainer>
 * ```
 */

export type PageContentMaxWidth = 'sm' | 'md' | 'lg' | 'none';

interface PageContentProps {
  children: ReactNode;
  className?: string;
  /**
   * 最大宽度约束
   * - sm: 1024px (表单/设置页面)
   * - md: 1280px (列表页面)
   * - lg: 1600px (仪表板)
   * - none: 无限制（占满容器）
   */
  maxWidth?: PageContentMaxWidth;
  /**
   * 是否水平居中（当内容小于容器宽度时）
   */
  centered?: boolean;
  /**
   * Section之间的垂直间距
   */
  spacing?: 'normal' | 'sm' | 'lg';
}

const maxWidthClasses: Record<PageContentMaxWidth, string> = {
  sm: 'max-w-content-sm',
  md: 'max-w-content-md',
  lg: 'max-w-content-lg',
  none: '',
};

const spacingClasses: Record<'normal' | 'sm' | 'lg', string> = {
  sm: 'space-y-section-gap-sm',
  normal: 'space-y-section-gap',
  lg: 'space-y-8',
};

export function PageContent({
  children,
  className,
  maxWidth = 'none',
  centered = true,
  spacing = 'normal',
}: PageContentProps) {
  return (
    <div className={cn(
      'w-full',
      maxWidthClasses[maxWidth],
      centered && 'mx-auto',
      spacingClasses[spacing],
      className
    )}>
      {children}
    </div>
  );
}

/**
 * Section - 页面内的独立区块
 * 可以包含标题和内容
 */
interface SectionProps {
  children: ReactNode;
  className?: string;
  title?: ReactNode;
  description?: string;
  actions?: ReactNode;
}

export function Section({
  children,
  className,
  title,
  description,
  actions
}: SectionProps) {
  return (
    <section className={cn('w-full', className)}>
      {(title || description || actions) && (
        <div className="flex items-center justify-between mb-4">
          <div className="space-y-1">
            {title && (
              <h2 className="text-lg font-semibold tracking-tight">
                {typeof title === 'string' ? title : title}
              </h2>
            )}
            {description && (
              <p className="text-sm text-muted-foreground">
                {description}
              </p>
            )}
          </div>
          {actions && <div className="flex items-center gap-2">{actions}</div>}
        </div>
      )}
      {children}
    </section>
  );
}
