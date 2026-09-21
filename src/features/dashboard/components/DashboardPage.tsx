import React from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '@/hooks/useAuth'
import { AgendaWidget, SalaryCalculatorWidget } from './Widgets'
import { ReportTimeDialog } from '@/features/time/components/ReportTimeDialog'
import { Clock } from 'lucide-react'
import { Button } from '@/components/ui/button'

export const DashboardPage: React.FC = () => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const [isReportTimeOpen, setIsReportTimeOpen] = React.useState(false)

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
      <div className="sticky top-0 z-20 flex h-14 shrink-0 items-center justify-between border-b border-border/70 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex flex-col gap-0.5">
          <h1 className="text-sm font-medium tracking-tight text-foreground">
            {t('dashboard.welcome', 'Välkommen tillbaka')}, {user?.name || user?.email}
          </h1>
          <p className="text-[11px] font-normal text-muted-foreground">{today}</p>
        </div>
        <Button
          onClick={() => setIsReportTimeOpen(true)}
          size="sm"
          className="h-8 gap-2 rounded-md px-3 text-xs font-medium"
        >
          <Clock className="h-3.5 w-3.5" />
          {t('timereports.report_time_btn', 'Rapportera tid')}
        </Button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          <div className="col-span-1 lg:col-span-2">
            <AgendaWidget />
          </div>
          <div className="col-span-1">
            <SalaryCalculatorWidget />
          </div>
        </div>
      </div>

      <ReportTimeDialog open={isReportTimeOpen} onOpenChange={setIsReportTimeOpen} />
    </div>
  )
}
