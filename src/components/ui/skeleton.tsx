import * as React from 'react'
import { cn } from '@/lib/utils'

export interface SkeletonProps extends React.ComponentProps<'div'> {
  shimmer?: boolean
}

function Skeleton({ className, shimmer = true, ...props }: SkeletonProps) {
  return (
    <div
      data-slot="skeleton"
      className={cn(
        'relative overflow-hidden rounded-md bg-muted/65 dark:bg-muted/35',
        shimmer &&
          'before:absolute before:inset-0 before:-translate-x-full before:animate-shimmer before:bg-gradient-to-r before:from-transparent before:via-foreground/5 before:to-transparent',
        !shimmer && 'animate-pulse',
        className,
      )}
      {...props}
    />
  )
}

export { Skeleton }

