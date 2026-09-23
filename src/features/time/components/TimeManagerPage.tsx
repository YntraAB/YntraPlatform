import React, { useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router-dom'
import { useSetDynamicBreadcrumbs } from '@/contexts/BreadcrumbContext'
import { useAuth } from '@/hooks/useAuth'
import { useTimeManager } from '../hooks/useTimeManager'
import { PlatformOverview } from './PlatformOverview'
import { TeamOverview } from './TeamOverview'
import { AssistantTeams } from './AssistantTeams'
import { ShiftSystem } from './ShiftSystem'
import { ToReportView } from './ToReportView'
import { ToAttestView } from './ToAttestView'
import { TimeHistoryView } from './TimeHistoryView'
import {
  Clock,
  RefreshCw,
  ChevronLeft,
} from 'lucide-react'
import { Button } from '@/components/ui/button'

type TimeTab = 'to_report' | 'to_attest' | 'history' | 'teams'

export const TimeManagerPage: React.FC = () => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const setDynamicBreadcrumbs = useSetDynamicBreadcrumbs()
  const [searchParams, setSearchParams] = useSearchParams()

  const {
    workspaceId,
    activeRole,
    shifts,
    dbTeams,
    dbWorkspaces,
    dbUsers,
    loading,
    currentLevel,
    selectedContext,
    filterType,
    searchQuery,
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
    openEmployeeShifts,
    openTeamShifts,
    openWorkspaceOverview,
    navigateBack,
    handleApprove,
    handleApproveSingle,
    handleReport,
    handleReportSingle,
    handleManualReport,
    handleDelete,
    fetchData,
  } = useTimeManager()

  const isAdmin = activeRole === 'admin' || activeRole === 'platform_admin'
  const canAttest = isAdmin || hasApprovePermission

  // Counts for plain text labels
  const unreportedCount = useMemo(() => {
    return shifts.filter((s) => s.status === 'not_submitted').length
  }, [shifts])

  const pendingAttestCount = useMemo(() => {
    return shifts.filter((s) => s.status === 'pending_attest').length
  }, [shifts])

  // Determine active tab
  const rawTab = searchParams.get('tab') as TimeTab | null
  const defaultTab: TimeTab = canAttest && pendingAttestCount > 0
    ? 'to_attest'
    : 'to_report'

  const activeTab: TimeTab = rawTab || defaultTab

  const handleTabChange = (newTab: TimeTab) => {
    setSearchParams({ tab: newTab })
  }

  // Dynamic breadcrumbs handling
  useEffect(() => {
    if (activeTab !== 'teams') {
      setDynamicBreadcrumbs([])
      return
    }

    const breadcrumbs = []
    if (currentLevel === 'assistant_teams') {
      breadcrumbs.push({
        label: t('sidebar.sections.directory', 'Team'),
        onClick: () => navigateBack(),
      })
    } else if (currentLevel === 'shift_list') {
      if (selectedContext.type === 'team') {
        const team = dbTeams.find((item) => item.id === selectedContext.id)
        breadcrumbs.push({
          label: team?.name || 'Team',
          onClick: () => navigateBack(),
        })
      } else if (selectedContext.type === 'employee') {
        const emp = dbUsers.find((item) => item.id === selectedContext.id)
        breadcrumbs.push({
          label: emp?.name || emp?.email || 'Personal',
          onClick: () => navigateBack(),
        })
      }
    }
    setDynamicBreadcrumbs(breadcrumbs)
  }, [
    activeTab,
    currentLevel,
    selectedContext.type,
    selectedContext.id,
    dbTeams,
    dbUsers,
    navigateBack,
    setDynamicBreadcrumbs,
    t,
  ])

  useEffect(() => {
    return () => {
      setDynamicBreadcrumbs([])
    }
  }, [setDynamicBreadcrumbs])

  useEffect(() => {
    const handleReset = (e: Event) => {
      const customEvent = e as CustomEvent<{ path: string }>
      if (customEvent.detail?.path === '/time' || customEvent.detail?.path === '/timereports') {
        if (activeTab === 'teams') {
          navigateBack()
        }
      }
    }
    window.addEventListener('yntra:breadcrumb-navigate', handleReset)
    return () => {
      window.removeEventListener('yntra:breadcrumb-navigate', handleReset)
    }
  }, [navigateBack, activeTab])

  // Drilldown title in teams tab
  const isDrilledDownInTeams = activeTab === 'teams' && currentLevel === 'shift_list'

  return (
    <div className="relative flex h-full flex-col bg-background selection:bg-primary/20">
      {/* Top Header - Exact h-12 Standard */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        {/* Left: Module title or Back button */}
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            {isDrilledDownInTeams ? (
              <Button
                variant="ghost"
                size="icon"
                onClick={navigateBack}
                className="h-8 w-8 rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground"
                aria-label={t('common.back', 'Tillbaka')}
              >
                <ChevronLeft className="h-3.5 w-3.5" />
              </Button>
            ) : (
              <Clock className="size-4 text-primary" />
            )}

            <span className="text-xs font-medium text-foreground">
              {isDrilledDownInTeams
                ? selectedContext.type === 'team'
                  ? dbTeams.find((tItem) => tItem.id === selectedContext.id)?.name || 'Team'
                  : dbUsers.find((u) => u.id === selectedContext.id)?.name || 'Personal'
                : t('sidebar.sections.timereports', 'Tidsrapporter')}
            </span>
          </div>

          <div className="h-4 w-px bg-border/40" />

          {/* Clean Underline Tabs (Yntra Standard, zero circular badges) */}
          <div className="flex h-12 items-center gap-5">
            {/* To Attest (Admin or staff with approve permissions) */}
            {canAttest && (
              <button
                type="button"
                onClick={() => handleTabChange('to_attest')}
                className={`flex h-12 items-center gap-1.5 border-b-2 -mb-px text-xs font-medium transition-colors ${
                  activeTab === 'to_attest'
                    ? 'border-primary text-foreground'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                }`}
              >
                <span>{t('timereports.tabs.to_attest', 'Att attestera')}</span>
                {pendingAttestCount > 0 && (
                  <span className="font-mono text-[11px] text-muted-foreground">
                    ({pendingAttestCount})
                  </span>
                )}
              </button>
            )}

            {/* To Report */}
            <button
              type="button"
              onClick={() => handleTabChange('to_report')}
              className={`flex h-12 items-center gap-1.5 border-b-2 -mb-px text-xs font-medium transition-colors ${
                activeTab === 'to_report'
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              }`}
            >
              <span>{t('timereports.tabs.to_report', 'Att rapportera')}</span>
              {unreportedCount > 0 && (
                <span className="font-mono text-[11px] text-muted-foreground">
                  ({unreportedCount})
                </span>
              )}
            </button>

            {/* History */}
            <button
              type="button"
              onClick={() => handleTabChange('history')}
              className={`flex h-12 items-center gap-1.5 border-b-2 -mb-px text-xs font-medium transition-colors ${
                activeTab === 'history'
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              }`}
            >
              <span>{t('timereports.tabs.history', 'Historik')}</span>
            </button>

            {/* Teams & Medarbetare (Admin only) */}
            {isAdmin && (
              <button
                type="button"
                onClick={() => handleTabChange('teams')}
                className={`flex h-12 items-center gap-1.5 border-b-2 -mb-px text-xs font-medium transition-colors ${
                  activeTab === 'teams'
                    ? 'border-primary text-foreground'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                }`}
              >
                <span>{t('timereports.tabs.teams', 'Team & Medarbetare')}</span>
              </button>
            )}
          </div>
        </div>

        {/* Right: Quick Refresh */}
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => fetchData()}
            disabled={loading}
            className="h-8 w-8 text-muted-foreground hover:bg-secondary hover:text-foreground"
            title="Uppdatera tidsrapporter"
          >
            <RefreshCw className={`size-3.5 ${loading ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Tab 1: Att rapportera */}
        {activeTab === 'to_report' && (
          <ToReportView
            shifts={shifts}
            teams={dbTeams}
            userId={user?.id}
            onReportSingle={handleReportSingle}
            onReportBatch={handleReport}
            onManualReport={handleManualReport}
          />
        )}

        {/* Tab 2: Att attestera */}
        {activeTab === 'to_attest' && canAttest && (
          <ToAttestView
            shifts={shifts}
            teams={dbTeams}
            users={dbUsers}
            onApproveSingle={handleApproveSingle}
            onApproveBatch={handleApprove}
          />
        )}

        {/* Tab 3: Historik */}
        {activeTab === 'history' && (
          <TimeHistoryView
            shifts={shifts}
            teams={dbTeams}
            users={dbUsers}
            activeRole={activeRole}
            currentUserId={user?.id}
          />
        )}

        {/* Tab 4: Team & Medarbetare (Admin drilldown) */}
        {activeTab === 'teams' && isAdmin && (
          <div className="flex h-full w-full flex-1 flex-col">
            {currentLevel === 'platform_overview' && (
              <PlatformOverview
                dbWorkspaces={dbWorkspaces}
                shifts={shifts}
                onSelectWorkspace={openWorkspaceOverview}
              />
            )}
            {currentLevel === 'team_overview' && (
              <TeamOverview
                dbUsers={dbUsers}
                shifts={shifts}
                loading={loading}
                openEmployeeShifts={openEmployeeShifts}
                onBack={activeRole === 'platform_admin' ? navigateBack : undefined}
              />
            )}
            {currentLevel === 'assistant_teams' && (
              <AssistantTeams
                dbTeams={dbTeams}
                shifts={shifts}
                activeRole={activeRole}
                workspaceId={workspaceId}
                openTeamShifts={openTeamShifts}
                onBack={activeRole === 'platform_admin' ? navigateBack : undefined}
              />
            )}
            {currentLevel === 'shift_list' && (
              <ShiftSystem
                shifts={shifts}
                selectedContext={selectedContext}
                searchQuery={searchQuery}
                filterType={filterType}
                currentPage={currentPage}
                selectedShifts={selectedShifts}
                listMode={listMode}
                isDeleteAlertOpen={isDeleteAlertOpen}
                hasApprovePermission={hasApprovePermission}
                setListMode={setListMode}
                setFilterType={setFilterType}
                setSearchQuery={setSearchQuery}
                setCurrentPage={setCurrentPage}
                setSelectedShifts={setSelectedShifts}
                setIsDeleteAlertOpen={setIsDeleteAlertOpen}
                handleApprove={handleApprove}
                handleReport={handleReport}
                handleDelete={handleDelete}
                onBack={navigateBack}
                onOpenReportDialog={() => handleTabChange('to_report')}
              />
            )}
          </div>
        )}
      </div>
    </div>
  )
}

export default TimeManagerPage
