import React from 'react'
import { Skeleton } from '@/components/ui/skeleton'

export const MessageSkeleton: React.FC = () => {
  return (
    <div className="flex w-full flex-1 flex-col gap-1 p-4 duration-500 animate-in fade-in zoom-in-95">
      {[1, 2, 3, 4, 5, 6].map((idx) => (
        <div key={idx} className="flex items-center gap-4 border-b border-border/40 px-2 py-4">
          {/* Select Area */}
          <div className="flex w-10 shrink-0 justify-start">
            <Skeleton className="h-[18px] w-[18px] rounded-[5px]" />
          </div>

          {/* Sender & Unread Dot Area */}
          <div className="w-56 shrink-0 space-y-2">
            <Skeleton className="h-4 w-3/4" />
            <Skeleton className="h-2 w-1/2" />
          </div>

          {/* Subject & Snippet */}
          <div className="flex-1 space-y-2">
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-3 w-2/3" />
          </div>

          {/* Icons & Timestamp */}
          <div className="flex w-48 shrink-0 items-center justify-end gap-3">
            <Skeleton className="h-[18px] w-[18px] rounded" />
            <Skeleton className="h-[18px] w-[18px] rounded" />
            <Skeleton className="h-[18px] w-[18px] rounded" />
            <Skeleton className="ml-2 h-3 w-12" />
          </div>
        </div>
      ))}
    </div>
  )
}
