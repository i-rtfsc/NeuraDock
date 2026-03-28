import * as React from "react"

import { cn } from "@/lib/utils"

export interface TextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {}

const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, ...props }, ref) => {
    return (
      <textarea
        className={cn(
          "flex min-h-[80px] w-full rounded-[var(--radius-control)] border border-[hsl(var(--control-border))] bg-[hsl(var(--control-bg)/var(--control-surface-alpha,1))] px-3 py-2 text-sm text-[hsl(var(--control-text))] shadow-xs ring-offset-background backdrop-blur-[var(--control-surface-blur,0px)] transition-[border-color,box-shadow,background-color,backdrop-filter] duration-base ease-smooth placeholder:text-[hsl(var(--control-text-muted))] hover:border-[hsl(var(--control-border-hover)/0.85)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--control-ring))] focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
          className
        )}
        ref={ref}
        {...props}
      />
    )
  }
)
Textarea.displayName = "Textarea"

export { Textarea }
