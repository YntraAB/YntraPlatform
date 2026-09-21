import React from 'react'
import { Skeleton } from '@/components/ui/skeleton'
import { Card, CardContent } from '@/components/ui/card'

export const ClientPageSkeleton: React.FC = () => {
  return (
    <div className="flex flex-1 flex-col gap-6 p-4 md:p-6 animate-fade-in max-w-6xl mx-auto w-full">
      {/* Welcome banner skeleton */}
      <div className="rounded-xl border border-border/60 bg-card p-6 shadow-xs space-y-3">
        <Skeleton className="h-6 w-56" />
        <Skeleton className="h-4 w-96 max-w-full" />
      </div>

      {/* Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {Array.from({ length: 3 }).map((_, i) => (
          <Card key={i} className="border border-border/60 bg-card p-5 shadow-xs">
            <CardContent className="p-0 space-y-3">
              <div className="flex items-center justify-between">
                <Skeleton className="h-4 w-28" />
                <Skeleton className="size-5 rounded-md" />
              </div>
              <Skeleton className="h-8 w-16" />
              <Skeleton className="h-3 w-36" />
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Upcoming shifts list skeleton */}
      <div className="rounded-xl border border-border/60 bg-card p-5 shadow-xs space-y-4">
        <div className="flex items-center justify-between pb-3 border-b border-border/40">
          <Skeleton className="h-5 w-40" />
          <Skeleton className="h-8 w-24 rounded-md" />
        </div>
        <div className="space-y-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <div
              key={i}
              className="flex items-center justify-between rounded-lg border border-border/30 bg-muted/20 p-4"
            >
              <div className="flex items-center gap-3">
                <Skeleton className="size-10 rounded-lg" />
                <div className="space-y-1.5">
                  <Skeleton className="h-4 w-44" />
                  <Skeleton className="h-3 w-28" />
                </div>
              </div>
              <Skeleton className="h-6 w-20 rounded-md" />
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
