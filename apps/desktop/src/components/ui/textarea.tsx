import * as React from "react"

import { cn } from "@/lib/utils"

export interface TextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {}

const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, ...props }, ref) => {
    return (
      <textarea
        className={cn(
          "flex min-h-[80px] w-full rounded-[var(--radius-control)] border border-[hsl(var(--control-border))] bg-[hsl(var(--control-bg))] px-3 py-2 text-sm text-[hsl(var(--control-text))] ring-offset-background placeholder:text-[hsl(var(--control-text-muted))] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--control-ring))] focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
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
