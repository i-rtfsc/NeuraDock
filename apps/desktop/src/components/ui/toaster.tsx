import * as React from 'react';
import { Toaster as Sonner } from 'sonner';

type ToasterProps = React.ComponentProps<typeof Sonner>;

const Toaster = ({ ...props }: ToasterProps) => {
  return (
    <Sonner
      theme="system"
      className="toaster group"
      toastOptions={{
        classNames: {
          toast:
            'group toast group-[.toaster]:bg-[hsl(var(--popover)/var(--control-surface-alpha,1))] group-[.toaster]:text-foreground group-[.toaster]:border-[hsl(var(--popover-border,var(--border)))] group-[.toaster]:shadow-lg group-[.toaster]:rounded-[var(--radius-control)] group-[.toaster]:border group-[.toaster]:backdrop-blur-[var(--control-surface-blur,0px)]',
          description: 'group-[.toast]:text-muted-foreground',
          actionButton:
            'group-[.toast]:bg-primary group-[.toast]:text-primary-foreground',
          cancelButton:
            'group-[.toast]:bg-muted group-[.toast]:text-muted-foreground',
        },
      }}
      {...props}
    />
  );
};

export { Toaster };
