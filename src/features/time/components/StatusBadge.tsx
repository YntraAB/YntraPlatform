import React from 'react'
import { useTranslation } from 'react-i18next'

interface StatusBadgeProps {
  status: string
}

export const StatusBadge: React.FC<StatusBadgeProps> = ({ status }) => {
  const { t } = useTranslation()

  switch (status) {
    case 'pending_attest':
      return (
        <div className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
          <span className="h-1.5 w-1.5 rounded-full bg-amber-500" />
          <span>{t('timereports.status.pending_attest')}</span>
        </div>
      )
    case 'approved':
      return (
        <div className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
          <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
          <span>{t('timereports.status.approved')}</span>
        </div>
      )
    case 'not_submitted':
      return (
        <div className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
          <span className="h-1.5 w-1.5 rounded-full bg-muted-foreground/40" />
          <span>{t('timereports.status.not_submitted')}</span>
        </div>
      )
    default:
      return (
        <div className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
          <span className="h-1.5 w-1.5 rounded-full bg-muted-foreground/30" />
          <span>{status}</span>
        </div>
      )
  }
}
