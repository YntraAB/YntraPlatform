import React from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '@/hooks/useAuth'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { CheckCircle2, Clock, FileText } from 'lucide-react'
import { useReportingStats } from '@/hooks/queries/useReporting'
import type { Report } from '@/types'

export const ReportingStats: React.FC = () => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const { workspaceId } = useWorkspace()

  const isAdmin = user?.role === 'admin' || user?.role === 'platform_admin'
  const { data: reports = [] } = useReportingStats(workspaceId)

  const stats = React.useMemo(() => {
    const userReports = isAdmin ? reports : reports.filter((r: Partial<Report>) => r.user_id === user?.id)
    return {
      total: userReports.length,
      pending: userReports.filter((r: Partial<Report>) => r.status === 'pending').length,
      resolved: userReports.filter((r: Partial<Report>) => r.status === 'resolved').length
    }
  }, [reports, isAdmin, user?.id])

  return (
    <div className="grid gap-4 md:grid-cols-3">
      <Card className="border-border/50 bg-card/50 backdrop-blur-sm">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('reporting.stats.total')}</CardTitle>
          <FileText className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">{stats.total}</div>
        </CardContent>
      </Card>
      <Card className="border-border/50 bg-card/50 backdrop-blur-sm">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('reporting.status.pending')}</CardTitle>
          <Clock className="h-4 w-4 text-amber-500" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold text-amber-500">{stats.pending}</div>
        </CardContent>
      </Card>
      <Card className="border-border/50 bg-card/50 backdrop-blur-sm">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('reporting.status.resolved')}</CardTitle>
          <CheckCircle2 className="h-4 w-4 text-green-500" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold text-green-500">{stats.resolved}</div>
        </CardContent>
      </Card>
    </div>
  )
}
