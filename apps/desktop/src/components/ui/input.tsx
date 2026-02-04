import * as React from 'react';
import { cn } from '@/lib/utils';

export interface InputProps
  extends React.InputHTMLAttributes<HTMLInputElement> {}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          'flex h-input w-full rounded-[var(--radius-control)] border border-[hsl(var(--control-border))] bg-[hsl(var(--control-bg))] px-3 py-2 text-sm text-[hsl(var(--control-text))] ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-[hsl(var(--control-text-muted))] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--control-ring))] focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50',
          className
        )}
        ref={ref}
        {...props}
      />
    );
  }
);
Input.displayName = 'Input';

export { Input };
