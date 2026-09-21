import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '@/hooks/useAuth'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Loader2, MessageSquare, ShieldAlert, Eye, Filter, RefreshCw } from 'lucide-react'
import { format } from 'date-fns'
import { sv, enUS } from 'date-fns/locale'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { toast } from 'sonner'

interface ReportContent {
  subject: string
  description: string
  date_of_incident?: string
  [key: string]: unknown
}

interface Report {
  id: string
  type: string
  content: ReportContent
  status: string
  is_anonymous: boolean
  created_at: string
  user?: {
    full_name: string
  }
}

import { useReports, useUpdateReportStatus } from '@/hooks/queries/useReporting'

export const ReportList: React.FC = () => {
  const { t, i18n } = useTranslation()
  const { user } = useAuth()
  const { workspaceId } = useWorkspace()
  const [selectedReport, setSelectedReport] = useState<Report | null>(null)
  const [statusFilter, setStatusFilter] = useState<string>('all')
  const [typeFilter, setTypeFilter] = useState<string>('all')

  const isAdmin = user?.role === 'admin' || user?.role === 'platform_admin'

  const { data, isLoading: loading, refetch: fetchReports } = useReports(workspaceId, {
    isAdmin,
    userId: user?.id || '',
    status: statusFilter,
    type: typeFilter
  })

  const reports = (data || []) as Report[]

  const updateStatusMutation = useUpdateReportStatus()

  const handleStatusChange = async (reportId: string, newStatus: string) => {
    try {
      await updateStatusMutation.mutateAsync({ reportId, status: newStatus })
      toast.success(t('reporting.status_updated'))
      if (selectedReport?.id === reportId) {
        setSelectedReport(prev => prev ? { ...prev, status: newStatus } : null)
      }
    } catch (error) {
      console.error('Error updating status:', error)
      toast.error(t('reporting.status_error'))
    }
  }

  const getStatusBadge = (status: string) => {
    const dotColors: Record<string, string> = {
      pending: 'bg-amber-500',
      reviewed: 'bg-blue-500',
      resolved: 'bg-emerald-500',
    }
    const dotColor = dotColors[status] || 'bg-muted-foreground'
    const label = t(`reporting.status.${status}`, status)

    return (
      <span className="inline-flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
        <span className={`h-1.5 w-1.5 rounded-full ${dotColor}`} />
        {label}
      </span>
    )
  }

  const locale = i18n.language === 'sv' ? sv : enUS

  return (
    <div className="space-y-4">
      {/* Filters */}
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between bg-muted/30 p-4 rounded-lg border border-border/50">
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center gap-2">
            <Filter className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">{t('common.filter')}:</span>
          </div>

          <Select value={statusFilter} onValueChange={setStatusFilter}>
            <SelectTrigger className="w-[150px] bg-background/50">
              <SelectValue placeholder={t('reporting.list.status')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t('common.all_teams')}</SelectItem>
              <SelectItem value="pending">{t('reporting.status.pending')}</SelectItem>
              <SelectItem value="reviewed">{t('reporting.status.reviewed')}</SelectItem>
              <SelectItem value="resolved">{t('reporting.status.resolved')}</SelectItem>
            </SelectContent>
          </Select>

          <Select value={typeFilter} onValueChange={setTypeFilter}>
            <SelectTrigger className="w-[200px] bg-background/50">
              <SelectValue placeholder={t('reporting.list.type')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t('common.all_teams')}</SelectItem>
              <SelectItem value="complaint">{t('reporting.types.complaint')}</SelectItem>
              <SelectItem value="work_injury">{t('reporting.types.work_injury')}</SelectItem>
              <SelectItem value="incident">{t('reporting.types.incident')}</SelectItem>
              <SelectItem value="deviation">{t('reporting.types.deviation')}</SelectItem>
              <SelectItem value="whistleblower">{t('reporting.types.whistleblower')}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <Button variant="outline" size="sm" onClick={() => fetchReports()} disabled={loading} className="gap-2">
          <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
          {t('common.refresh')}
        </Button>
      </div>

      {loading ? (
        <div className="rounded-md border border-border/50 bg-card/30 p-4 space-y-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="flex items-center justify-between gap-4 py-2 border-b border-border/20 last:border-0">
              <Skeleton className="h-4 w-28" />
              <Skeleton className="h-4 w-36" />
              <Skeleton className="h-4 w-48" />
              <Skeleton className="h-4 w-20" />
              <Skeleton className="h-6 w-16 rounded-md" />
            </div>
          ))}
        </div>
      ) : reports.length === 0 ? (
        <div className="flex h-40 flex-col items-center justify-center text-muted-foreground border border-dashed rounded-lg">
          <MessageSquare className="mb-2 h-10 w-10 opacity-20" />
          <p>{t('reporting.list.empty')}</p>
        </div>
      ) : (
        <div className="rounded-md border border-border/50 bg-card/30">
          <Table>
            <TableHeader>
              <TableRow className="border-border/50 hover:bg-transparent">
                <TableHead>{t('reporting.list.type')}</TableHead>
                <TableHead>{t('reporting.list.reporter')}</TableHead>
                <TableHead>{t('reporting.form.subject')}</TableHead>
                <TableHead>{t('reporting.list.date')}</TableHead>
                <TableHead>{t('reporting.list.status')}</TableHead>
                <TableHead className="text-right">{t('common.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {reports.map((report) => (
                <TableRow key={report.id} className="border-border/50 group hover:bg-muted/30">
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-2">
                      {report.is_anonymous && (
                        <ShieldAlert className="h-3 w-3 text-primary" />
                      )}
                      {t(`reporting.types.${report.type}`)}
                    </div>
                  </TableCell>
                  <TableCell>
                    {report.is_anonymous ? (
                      <span className="text-muted-foreground italic">{t('messages.anonymous')}</span>
                    ) : (
                      report.user?.full_name || 'System'
                    )}
                  </TableCell>
                  <TableCell className="max-w-[200px] truncate">
                    {report.content.subject}
                  </TableCell>
                  <TableCell>
                    {format(new Date(report.created_at), 'PPP', { locale })}
                  </TableCell>
                  <TableCell>{getStatusBadge(report.status)}</TableCell>
                  <TableCell className="text-right">
                    <Dialog>
                      <DialogTrigger asChild>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => setSelectedReport(report)}
                          className="opacity-0 group-hover:opacity-100 transition-opacity"
                        >
                          <Eye className="h-4 w-4" />
                        </Button>
                      </DialogTrigger>
                      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-[540px]">
                        <DialogHeader className="flex h-12 flex-row items-center justify-between border-b border-border bg-secondary/40 px-5">
                          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
                            <ShieldAlert className="size-3.5 text-muted-foreground/70 shrink-0" />
                            <span>{t(`reporting.types.${report.type}`)}</span>
                            {report.is_anonymous && (
                              <span className="rounded bg-secondary px-1.5 py-0.5 text-[10px] text-muted-foreground">
                                {t('messages.anonymous')}
                              </span>
                            )}
                          </DialogTitle>
                          <span className="text-[11px] text-muted-foreground mr-6">
                            {format(new Date(report.created_at), 'PPP', { locale })}
                          </span>
                        </DialogHeader>
                        <div className="space-y-3.5 p-5">
                          <div className="grid grid-cols-2 gap-3">
                            <div>
                              <h4 className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                                {t('reporting.form.subject')}
                              </h4>
                              <p className="text-xs font-medium text-foreground">{report.content.subject}</p>
                            </div>
                            <div>
                              <h4 className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                                {t('reporting.list.status')}
                              </h4>
                              {getStatusBadge(report.status)}
                            </div>
                          </div>

                          <div>
                            <h4 className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                              {t('reporting.form.description')}
                            </h4>
                            <p className="whitespace-pre-wrap rounded-md border border-border bg-secondary/20 p-3 text-xs leading-relaxed text-foreground">
                              {report.content.description}
                            </p>
                          </div>

                          {report.content.date_of_incident && (
                            <div>
                              <h4 className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                                {t('reporting.form.date_of_incident')}
                              </h4>
                              <p className="text-xs text-foreground">
                                {report.content.date_of_incident}
                              </p>
                            </div>
                          )}

                          {isAdmin && (
                            <div className="space-y-2 border-t border-border pt-3">
                              <h4 className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                                {t('reporting.admin.manage_status')}
                              </h4>
                              <div className="flex items-center gap-2">
                                <Select 
                                  value={report.status} 
                                  onValueChange={(val) => handleStatusChange(report.id, val)}
                                  disabled={updateStatusMutation.isPending && updateStatusMutation.variables?.reportId === report.id}
                                >
                                  <SelectTrigger className="h-8 text-xs">
                                    <SelectValue placeholder={t('reporting.admin.select_status')} />
                                  </SelectTrigger>
                                  <SelectContent>
                                    <SelectItem value="pending">{t('reporting.status.pending')}</SelectItem>
                                    <SelectItem value="reviewed">{t('reporting.status.reviewed')}</SelectItem>
                                    <SelectItem value="resolved">{t('reporting.status.resolved')}</SelectItem>
                                  </SelectContent>
                                </Select>
                                {updateStatusMutation.isPending && updateStatusMutation.variables?.reportId === report.id && (
                                  <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                                )}
                              </div>
                            </div>
                          )}
                        </div>
                      </DialogContent>
                    </Dialog>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  )
}
