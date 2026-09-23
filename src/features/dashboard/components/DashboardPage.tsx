import React from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '@/hooks/useAuth'
import { AgendaWidget, SalaryCalculatorWidget } from './Widgets'
import { Clock } from 'lucide-react'
import { Button } from '@/components/ui/button'

export const DashboardPage: React.FC = () => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const navigate = useNavigate()

  // Format current date nicely
  const today = new Date().toLocaleDateString(undefined, {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Sticky Header */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/70 bg-background/80 px-6 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <h1 className="text-xs font-medium tracking-tight text-foreground">
            {t('dashboard.welcome', 'Välkommen tillbaka')}, {user?.name || user?.email}
          </h1>
          <span className="text-[11px] font-normal text-muted-foreground/60">
            · {today}
          </span>
        </div>
        <Button
          onClick={() => navigate('/timereports')}
          size="sm"
          className="h-8 gap-1.5 rounded-md px-3 text-xs font-medium shadow-2xs"
        >
          <Clock className="size-3.5" />
          <span>{t('timereports.report_time_btn', 'Rapportera tid')}</span>
        </Button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 items-stretch">
          <div className="col-span-1 lg:col-span-2 flex flex-col min-h-0">
            <AgendaWidget />
          </div>
          <div className="col-span-1 flex flex-col min-h-0">
            <SalaryCalculatorWidget />
          </div>
        </div>
      </div>
    </div>
  )
}

