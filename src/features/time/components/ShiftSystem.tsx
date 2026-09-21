import React, { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import {
  ChevronLeft,
  ChevronRight,
  Search,
  Trash2,
  FileCheck,
  Clock,
  Inbox,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { StatusBadge } from './StatusBadge'
import type { TimeReportUI } from '../types'

interface ShiftSystemProps {
  shifts: TimeReportUI[]
  selectedContext: { type: 'employee' | 'team' | null; id: string | null }
  searchQuery: string
  filterType: string
  currentPage: number
  selectedShifts: string[]
  listMode: 'current' | 'history'
  isDeleteAlertOpen: boolean
  hasApprovePermission: boolean
  setListMode: (mode: 'current' | 'history') => void
  setFilterType: (type: string) => void
  setSearchQuery: (q: string) => void
  setCurrentPage: (p: number | ((prev: number) => number)) => void
  setSelectedShifts: (s: string[] | ((prev: string[]) => string[])) => void
  setIsDeleteAlertOpen: (open: boolean) => void
  handleApprove: () => void
  handleReport: () => void
  handleDelete: () => void
  onBack?: () => void
}

export const ShiftSystem: React.FC<ShiftSystemProps> = ({
  shifts,
  selectedContext,
  searchQuery,
  filterType,
  currentPage,
  selectedShifts,
  listMode,
  isDeleteAlertOpen,
  hasApprovePermission,
  setListMode,
  setFilterType,
  setSearchQuery,
  setCurrentPage,
  setSelectedShifts,
  setIsDeleteAlertOpen,
  handleApprove,
  handleReport,
  handleDelete,
  onBack,
}) => {
  const { t } = useTranslation()

  let contextShifts = shifts
  if (selectedContext.type === 'employee') {
    contextShifts = shifts.filter((s) => s.employeeId === selectedContext.id)
  } else if (selectedContext.type === 'team') {
    contextShifts = shifts.filter((s) => s.team === selectedContext.id)
  }

  const filteredShifts = useMemo(() => {
    return contextShifts.filter((s) => {
      const searchMatch =
        s.employee.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.team.toLowerCase().includes(searchQuery.toLowerCase())

      let statusMatch = true
      if (filterType !== 'all') {
        statusMatch = s.status === filterType
      }

      return searchMatch && statusMatch
    })
  }, [contextShifts, searchQuery, filterType])

  const itemsPerPage = 12
  const totalPages = Math.max(1, Math.ceil(filteredShifts.length / itemsPerPage))
  const currentShifts = filteredShifts.slice(
    (currentPage - 1) * itemsPerPage,
    currentPage * itemsPerPage,
  )

  const isAllSelected = filteredShifts.length > 0 && selectedShifts.length === filteredShifts.length
  const toggleSelectAll = () => {
    if (isAllSelected) setSelectedShifts([])
    else setSelectedShifts(filteredShifts.map((s) => s.id))
  }

  const toggleSelect = (id: string, e: React.MouseEvent<HTMLDivElement>) => {
    e.stopPropagation()
    setSelectedShifts((prev) =>
      Array.isArray(prev)
        ? prev.includes(id)
          ? prev.filter((s) => s !== id)
          : [...prev, id]
        : [id],
    )
  }

  const renderHistory = () => {
    let history = shifts
      .filter((s) => s.status === 'approved')
      .map((s) => {
        const d = new Date(s.date)
        const monthStr = d.toLocaleDateString('sv-SE', { month: 'long', year: 'numeric' })
        return {
          id: 'h' + s.id,
          employeeId: s.employeeId,
          teamId: s.team,
          month: monthStr.charAt(0).toUpperCase() + monthStr.slice(1),
          hours: s.duration,
          ob: 0,
          absence: 0,
          salary: '-',
        }
      })
    if (selectedContext.type === 'employee') {
      history = history.filter((h) => h.employeeId === selectedContext.id)
    } else if (selectedContext.type === 'team') {
      history = history.filter((h) => h.teamId === selectedContext.id)
    }

    return (
      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {history.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
              <Inbox className="h-10 w-10 opacity-20" />
            </div>
            <p className="text-sm font-medium">{t('timereports.no_history_title')}</p>
            <p className="mt-1 max-w-[250px] text-center text-xs text-muted-foreground/70">
              {t('timereports.no_history_desc')}
            </p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {history.map((h, idx) => (
              <div
                key={h.id}
                style={{ animationDelay: `${idx * 25}ms` }}
                className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
              >
                {/* Leading Icon */}
                <div className="mr-3 flex size-6 shrink-0 items-center justify-center text-muted-foreground/70 transition-colors group-hover:text-primary">
                  <FileCheck className="h-4 w-4" />
                </div>

                {/* Month Name Column */}
                <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors group-hover:text-primary">
                  {h.month}
                </div>

                {/* Middle Column - Statistics */}
                <div className="flex min-w-0 flex-1 items-center gap-x-6 truncate pr-6 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1 font-mono">
                    <span className="font-sans text-muted-foreground/60">{t('timereports.work_time')}:</span>{' '}
                    <span className="text-foreground/80">{h.hours}h</span>
                  </span>
                  <span className="flex items-center gap-1 font-mono">
                    <span className="font-sans text-muted-foreground/60">{t('timereports.ob_bonus')}:</span>{' '}
                    <span className="text-foreground/80">{h.ob}h</span>
                  </span>
                  {h.absence > 0 && (
                    <span className="flex items-center gap-1 font-mono text-rose-400/80">
                      <span className="font-sans">{t('timereports.absence')}:</span> {h.absence}h
                    </span>
                  )}
                </div>

                {/* Status Micro-Dot & Salary Column */}
                <div className="flex w-52 shrink-0 items-center justify-end gap-3">
                  <span className="flex items-center gap-1.5 text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60">
                    <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shrink-0" />
                    <span>{t('timereports.attested')}</span>
                  </span>
                  <span className="opacity-40 text-xs">·</span>
                  <span className="font-mono text-xs font-medium tabular-nums text-foreground/80">
                    {h.salary}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    )
  }

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header - Tabs & Back Button */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex items-center gap-3">
          {onBack && (
            <>
              <Button
                variant="ghost"
                size="icon"
                onClick={onBack}
                className="h-8 w-8 rounded-md text-muted-foreground hover:text-foreground hover:bg-secondary"
                aria-label={t('common.back')}
              >
                <ChevronLeft className="h-3.5 w-3.5" />
              </Button>
              <div className="h-3.5 w-px bg-border/40" />
            </>
          )}
          <div className="flex h-full items-center gap-5">
            <button
              onClick={() => setListMode('current')}
              className={`flex h-12 items-center gap-1.5 border-b-2 text-xs font-medium transition-all duration-200 ${
                listMode === 'current'
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground/70'
              }`}
            >
              {t('timereports.current_reports')}
            </button>
            <button
              onClick={() => setListMode('history')}
              className={`flex h-12 items-center gap-1.5 border-b-2 text-xs font-medium transition-all duration-200 ${
                listMode === 'history'
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground/70'
              }`}
            >
              {t('timereports.previous_months')}
            </button>
          </div>
        </div>
      </div>

      {listMode === 'history' ? (
        renderHistory()
      ) : (
        <div className="flex min-h-0 flex-1 flex-col">
          {/* Subheader Controls - Filter, Actions, Search, Pagination */}
          <div className="flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/30 px-6 backdrop-blur-sm">
            <div className="flex items-center gap-4">
              <div className="flex items-center justify-center">
                <Checkbox
                  checked={isAllSelected}
                  onCheckedChange={toggleSelectAll}
                  aria-label={t('common.select_all')}
                  className="h-4 w-4"
                />
              </div>

              <div className="flex items-center gap-3">
                <Select value={filterType} onValueChange={setFilterType}>
                  <SelectTrigger className="h-8 w-[150px] border-border/50 bg-muted/50 text-xs font-medium text-foreground">
                    <SelectValue placeholder={t('timereports.filter_status')} />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t('timereports.all_reports')}</SelectItem>
                    <SelectItem value="not_submitted">
                      {t('timereports.status.not_submitted', 'Ej inlämnade')} (
                      {filteredShifts.filter((s) => s.status === 'not_submitted').length})
                    </SelectItem>
                    <SelectItem value="pending_attest">
                      {t('timereports.pending')} (
                      {filteredShifts.filter((s) => s.status === 'pending_attest').length})
                    </SelectItem>
                    <SelectItem value="approved">
                      {t('timereports.approved')} (
                      {filteredShifts.filter((s) => s.status === 'approved').length})
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {selectedShifts.length > 0 && (
                <div className="ml-2 flex items-center gap-2 border-l border-border/50 pl-4 duration-200 animate-in fade-in slide-in-from-left-2">
                  {currentShifts.filter((s) => selectedShifts.includes(s.id)).some((s) => s.status === 'not_submitted') && (
                    <Button
                      onClick={handleReport}
                      size="sm"
                      className="h-8 gap-1.5 px-3 text-xs font-medium"
                    >
                      <Clock className="h-3.5 w-3.5" />
                      Rapportera valda ({currentShifts.filter((s) => selectedShifts.includes(s.id) && s.status === 'not_submitted').length})
                    </Button>
                  )}
                  {hasApprovePermission && (
                    <Button
                      onClick={handleApprove}
                      variant="default"
                      size="sm"
                      className="h-8 gap-1.5 px-3 text-xs font-medium"
                    >
                      <FileCheck className="h-3.5 w-3.5" />
                      {t('timereports.approve_count', { count: selectedShifts.length })}
                    </Button>
                  )}
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                    onClick={() => setIsDeleteAlertOpen(true)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              )}
            </div>

            <div className="flex items-center gap-4">
              <div className="group relative hidden w-56 sm:block">
                <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
                <Input
                  placeholder={t('timereports.search_placeholder')}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="h-8 rounded-md border border-border/50 bg-muted/50 pl-8 text-xs font-medium text-foreground transition-all focus-visible:bg-muted focus-visible:ring-1 focus-visible:ring-primary"
                />
              </div>
              <div className="flex items-center gap-2 text-muted-foreground/60">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 rounded-md"
                  disabled={currentPage === 1}
                  onClick={() => setCurrentPage((prev) => prev - 1)}
                >
                  <ChevronLeft className="h-4 w-4" />
                </Button>
                <span className="text-xs font-medium tabular-nums text-muted-foreground">
                  {currentPage} / {totalPages}
                </span>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 rounded-md"
                  disabled={currentPage === totalPages}
                  onClick={() => setCurrentPage((prev) => prev + 1)}
                >
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </div>

          {/* Shift Rows - exactly matching Inbox h-12 layout */}
          <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
            {filteredShifts.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
                <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
                  <Inbox className="h-10 w-10 opacity-20" />
                </div>
                <p className="text-sm font-medium">{t('timereports.no_reports_title')}</p>
                <p className="mt-1 max-w-[250px] text-center text-xs text-muted-foreground/70">
                  {t('timereports.no_reports_desc')}
                </p>
              </div>
            ) : (
              <div className="flex w-full flex-col text-sm">
                {currentShifts.map((shift, idx) => {
                  const isSelected = selectedShifts.includes(shift.id)
                  return (
                    <div
                      key={shift.id}
                      style={{ animationDelay: `${idx * 20}ms` }}
                      className={`group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-200 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both ${
                        isSelected ? 'bg-primary/5' : ''
                      }`}
                    >
                      {/* Select Checkbox */}
                      <div
                        className="mr-3 flex size-6 shrink-0 cursor-pointer items-center justify-center"
                        onClick={(e) => toggleSelect(shift.id, e)}
                      >
                        <Checkbox
                          checked={isSelected}
                          onCheckedChange={() =>
                            setSelectedShifts((prev) =>
                              Array.isArray(prev)
                                ? prev.includes(shift.id)
                                  ? prev.filter((s) => s !== shift.id)
                                  : [...prev, shift.id]
                                : [shift.id],
                            )
                          }
                          className="h-4 w-4"
                        />
                      </div>

                      {/* Target Name (Employee or Team) - identical to Inbox */}
                      <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors group-hover:text-primary">
                        {selectedContext.type === 'team' ? shift.employee : shift.team}
                      </div>

                      {/* Middle Column - Details / Note / Hours */}
                      <div className="flex min-w-0 flex-1 items-center gap-x-4 truncate pr-6 text-sm font-light">
                        <span className="truncate text-xs italic text-muted-foreground/60">
                          {shift.note || (selectedContext.type === 'team' ? shift.role : t('timereports.client'))}
                        </span>
                        <div className="flex shrink-0 items-center gap-1.5 font-mono text-xs text-muted-foreground/80">
                          <Clock className="h-3 w-3 opacity-60" />
                          <span>
                            {shift.start} - {shift.end}
                          </span>
                          <span className="opacity-40">·</span>
                          <span className="font-medium text-foreground/80">{shift.duration}h</span>
                        </div>
                      </div>

                      {/* Status & Date - identical to Inbox */}
                      <div className="flex w-52 shrink-0 items-center justify-end gap-4">
                        <StatusBadge status={shift.status} />
                        <span className="font-mono text-xs tabular-nums text-muted-foreground/60">
                          {shift.date}
                        </span>
                      </div>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Delete Confirmation Alert */}
      <AlertDialog open={isDeleteAlertOpen} onOpenChange={setIsDeleteAlertOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('timereports.delete_alert.title')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('timereports.delete_alert.description', { count: selectedShifts.length })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90 font-medium"
            >
              {t('timereports.delete_alert.confirm')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

