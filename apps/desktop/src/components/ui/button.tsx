import * as React from 'react';
import { Slot } from '@radix-ui/react-slot';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '@/lib/utils';

const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[var(--radius-control)] text-sm font-medium ring-offset-background transition-all duration-base ease-smooth focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 active:scale-[var(--scale-active)]',
  {
    variants: {
      variant: {
        default: 'bg-primary text-primary-foreground shadow-sm hover:bg-primary/90 hover:shadow-hover-sm',
        destructive: 'bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90 hover:shadow-hover-sm',
        outline: 'border-2 border-[hsl(var(--control-border))] bg-[hsl(var(--control-bg))] text-[hsl(var(--control-text))] hover:bg-[hsl(var(--control-bg-hover))] hover:border-[hsl(var(--control-border-hover))] hover:shadow-sm',
        secondary: 'bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80 hover:shadow-hover-sm',
        ghost: 'hover:bg-accent hover:text-accent-foreground',
        link: 'text-primary underline-offset-4 hover:underline',
      },
      size: {
        default: 'h-btn px-[var(--control-button-padding-x-default)]',
        sm: 'h-btn-sm px-[var(--control-button-padding-x-sm)] text-xs rounded-[var(--radius-control-sm)]',
        lg: 'h-btn-lg px-[var(--control-button-padding-x-lg)] text-base rounded-[var(--radius-control-lg)]',
        icon: 'h-btn-icon w-btn-icon p-0',
        'icon-sm': 'h-btn-icon-sm w-btn-icon-sm p-0',
        'icon-lg': 'h-btn-icon-lg w-btn-icon-lg p-0',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : 'button';
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  }
);
Button.displayName = 'Button';

export { Button, buttonVariants };
