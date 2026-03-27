import type { SVGProps } from 'react';

import { cn } from '@/lib/utils';

export function CodexMarkIcon({
  className,
  strokeWidth = 1.8,
  ...props
}: SVGProps<SVGSVGElement>) {
  const petals = [0, 60, 120, 180, 240, 300];

  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
      className={cn('shrink-0', className)}
      {...props}
    >
      <g
        stroke="currentColor"
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        {petals.map((angle) => (
          <g key={angle} transform={`rotate(${angle} 12 12)`}>
            <rect x="10.15" y="3.4" width="3.7" height="8.25" rx="1.85" />
          </g>
        ))}
        <circle cx="12" cy="12" r="2.15" />
      </g>
    </svg>
  );
}
