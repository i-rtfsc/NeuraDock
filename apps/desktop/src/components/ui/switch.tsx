import * as React from "react"
import * as SwitchPrimitives from "@radix-ui/react-switch"

import { cn } from "@/lib/utils"

const Switch = React.forwardRef<
  React.ElementRef<typeof SwitchPrimitives.Root>,
  React.ComponentPropsWithoutRef<typeof SwitchPrimitives.Root>
>(({ className, ...props }, ref) => (
  <SwitchPrimitives.Root
    className={cn(
      "peer inline-flex h-7 w-12 shrink-0 cursor-pointer items-center rounded-full border border-[hsl(var(--control-border))] bg-[hsl(var(--control-bg))] shadow-[inset_0_1px_2px_hsl(var(--foreground)/0.12)] transition-colors duration-base ease-smooth focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--control-ring))] focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-[hsl(var(--primary))] data-[state=checked]:border-[hsl(var(--primary))] data-[state=checked]:shadow-[inset_0_1px_2px_hsl(var(--primary-foreground)/0.2)]",
      className
    )}
    {...props}
    ref={ref}
  >
    <SwitchPrimitives.Thumb
      className={cn(
        "pointer-events-none block h-6 w-6 rounded-full bg-[hsl(var(--card))] shadow-[0_2px_6px_hsl(var(--foreground)/0.18)] ring-1 ring-border/50 transition-transform duration-base ease-smooth data-[state=checked]:translate-x-5 data-[state=checked]:bg-[hsl(var(--primary-foreground))] data-[state=unchecked]:translate-x-0"
      )}
    />
  </SwitchPrimitives.Root>
))
Switch.displayName = SwitchPrimitives.Root.displayName

export { Switch }
