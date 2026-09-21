import React from 'react'
import type { LucideIcon } from 'lucide-react'

interface EmptyStateProps {
  icon: LucideIcon
  title: string
  description: string
}

export const EmptyState: React.FC<EmptyStateProps> = ({ icon: Icon, title, description }) => (
  <div className="flex h-full flex-col items-center justify-center py-12 text-muted-foreground duration-300 animate-in fade-in zoom-in">
    <Icon className="mb-4 h-12 w-12 opacity-20" />
    <h3 className="text-lg font-medium text-foreground">{title}</h3>
    <p className="mt-1 max-w-[250px] text-center text-sm">{description}</p>
  </div>
)
