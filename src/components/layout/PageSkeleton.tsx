import React from 'react'
import { Skeleton } from '@/components/ui/skeleton'
import { Card, CardContent } from '@/components/ui/card'

export const PageSkeleton: React.FC<{
  className?: string
  cardsCount?: number
  rowsCount?: number
}> = ({ className = '', cardsCount = 3, rowsCount = 5 }) => {
  return (
    <div className={`flex flex-1 flex-col gap-6 p-6 animate-fade-in ${className}`}>
      {/* Top Page Header Skeleton */}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="space-y-1.5">
          <Skeleton className="h-7 w-48" />
          <Skeleton className="h-3.5 w-72" />
        </div>
        <div className="flex items-center gap-2">
          <Skeleton className="h-8 w-24 rounded-md" />
          <Skeleton className="h-8 w-28 rounded-md" />
        </div>
      </div>

      {/* Metric Cards Skeleton */}
      {cardsCount > 0 && (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: cardsCount }).map((_, i) => (
            <Card key={i} className="rounded-lg border border-border/60 bg-card p-4 shadow-xs">
              <CardContent className="p-0 space-y-3">
                <div className="flex items-center justify-between">
                  <Skeleton className="h-3.5 w-24" />
                  <Skeleton className="size-5 rounded-md" />
                </div>
                <div className="space-y-1.5">
                  <Skeleton className="h-7 w-20" />
                  <Skeleton className="h-3 w-32" />
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Main Content Area Skeleton */}
      <div className="flex-1 rounded-lg border border-border/60 bg-card p-5 shadow-xs space-y-4">
        {/* Table / Grid Toolbar */}
        <div className="flex items-center justify-between gap-3 pb-3 border-b border-border/40">
          <div className="flex items-center gap-2">
            <Skeleton className="h-8 w-48 rounded-md" />
            <Skeleton className="h-8 w-28 rounded-md" />
          </div>
          <div className="flex items-center gap-2">
            <Skeleton className="h-8 w-20 rounded-md" />
            <Skeleton className="size-8 rounded-md" />
          </div>
        </div>

        {/* Data Rows */}
        <div className="space-y-3 pt-1">
          {Array.from({ length: rowsCount }).map((_, i) => (
            <div
              key={i}
              className="flex items-center justify-between rounded-md border border-border/30 bg-muted/20 px-4 py-3"
            >
              <div className="flex items-center gap-3">
                <Skeleton className="size-8 rounded-md" />
                <div className="space-y-1.5">
                  <Skeleton className="h-3.5 w-36 sm:w-52" />
                  <Skeleton className="h-2.5 w-24 sm:w-32" />
                </div>
              </div>
              <div className="flex items-center gap-4">
                <Skeleton className="hidden h-3 w-20 sm:block" />
                <Skeleton className="h-6 w-16 rounded-md" />
                <Skeleton className="size-6 rounded-md" />
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
