import React, { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useSetDynamicBreadcrumbs } from '@/contexts/BreadcrumbContext'
import { useTimeManager } from '../hooks/useTimeManager'
import { PlatformOverview } from './PlatformOverview'
import { TeamOverview } from './TeamOverview'
import { AssistantTeams } from './AssistantTeams'
import { ShiftSystem } from './ShiftSystem'

export const TimeManagerPage: React.FC = () => {
  const { t } = useTranslation()
  const setDynamicBreadcrumbs = useSetDynamicBreadcrumbs()
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
    handleReport,
    handleDelete,
  } = useTimeManager()

  useEffect(() => {
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
      if (customEvent.detail?.path === '/time') {
        navigateBack()
      }
    }
    window.addEventListener('yntra:breadcrumb-navigate', handleReset)
    return () => {
      window.removeEventListener('yntra:breadcrumb-navigate', handleReset)
    }
  }, [navigateBack])

  return (
    <div className="relative flex h-full flex-col bg-background">
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
          />
        )}
      </div>
    </div>
  )
}

export default TimeManagerPage
