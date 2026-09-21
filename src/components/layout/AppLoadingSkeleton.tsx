import React from 'react'
import { Skeleton } from '@/components/ui/skeleton'
import { PageSkeleton } from './PageSkeleton'

export const AppLoadingSkeleton: React.FC = () => {
  return (
    <div className="relative flex h-screen w-full overflow-hidden bg-background">
      {/* Top Hairline Indeterminate Progress Line */}
      <div className="absolute top-0 left-0 right-0 z-50 h-[2px] overflow-hidden bg-border/40">
        <div className="h-full bg-primary animate-indeterminate-bar" />
      </div>

      {/* Left Sidebar Skeleton (desktop) */}
      <aside className="hidden md:flex w-60 shrink-0 flex-col border-r border-border/70 bg-sidebar/50 backdrop-blur-xs p-4 justify-between">
        <div className="space-y-6">
          {/* Logo / Brand */}
          <div className="flex items-center gap-2.5 px-2">
            <Skeleton className="size-6 rounded-md" />
            <Skeleton className="h-4 w-20" />
          </div>

          {/* Workspace Switcher Skeleton */}
          <Skeleton className="h-9 w-full rounded-md" />

          {/* Navigation Items */}
          <div className="space-y-1.5 pt-2">
            <Skeleton className="h-2.5 w-16 px-2 mb-3" />
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="flex items-center gap-3 px-2 py-2 rounded-md">
                <Skeleton className="size-4 rounded-xs shrink-0" />
                <Skeleton className="h-3.5 flex-1" />
              </div>
            ))}
          </div>
        </div>

        {/* User Profile Area */}
        <div className="pt-4 border-t border-border/50 flex items-center gap-3 px-2">
          <Skeleton className="size-8 rounded-full shrink-0" />
          <div className="flex-1 space-y-1.5 min-w-0">
            <Skeleton className="h-3 w-24" />
            <Skeleton className="h-2.5 w-14" />
          </div>
        </div>
      </aside>

      {/* Main Content Area */}
      <div className="flex flex-1 flex-col min-w-0 overflow-hidden">
        {/* Top Header Bar */}
        <header className="flex h-12 shrink-0 items-center justify-between border-b border-border/60 bg-background/80 px-6 backdrop-blur-xs">
          <div className="flex items-center gap-2">
            <Skeleton className="size-5 rounded-md md:hidden" />
            <Skeleton className="h-3.5 w-16" />
            <span className="text-muted-foreground/30 text-xs">/</span>
            <Skeleton className="h-3.5 w-24" />
          </div>
          <div className="flex items-center gap-3">
            <Skeleton className="hidden sm:block h-8 w-44 rounded-md" />
            <Skeleton className="size-7 rounded-md" />
            <Skeleton className="size-7 rounded-full" />
          </div>
        </header>

        {/* Content Body */}
        <main className="flex-1 overflow-y-auto">
          <PageSkeleton cardsCount={3} rowsCount={4} />
        </main>
      </div>
    </div>
  )
}
