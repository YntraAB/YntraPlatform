import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { supabase } from '@/lib/supabase'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'
import type { TimeReportUI, DevRole, NavLevel } from '../types'
import type { User } from '@/types'

interface WorkspaceRow {
  id: string
  name: string
}

interface TeamRow {
  id: string
  name: string
  workspace_id: string
}

export const useTimeManager = () => {
  const { t } = useTranslation()
  const { workspaceId } = useWorkspace()
  const { user } = useAuth()

  const activeRole: DevRole = (user?.role as DevRole) || 'admin'
  const [shifts, setShifts] = useState<TimeReportUI[]>([])
  const [dbTeams, setDbTeams] = useState<TeamRow[]>([])
  const [dbWorkspaces, setDbWorkspaces] = useState<WorkspaceRow[]>([])
  const [dbUsers, setDbUsers] = useState<User[]>([])
  const [loading, setLoading] = useState(true)

  const [currentLevel, setCurrentLevel] = useState<NavLevel>('team_overview')
  const [selectedContext, setSelectedContext] = useState<{
    type: 'employee' | 'team' | null
    id: string | null
  }>({ type: null, id: null })

  const [filterType, setFilterType] = useState('all')
  const [searchQuery, setSearchQuery] = useState('')
  const [currentPage, setCurrentPage] = useState(1)
  const [selectedShifts, setSelectedShifts] = useState<string[]>([])
  const [listMode, setListMode] = useState<'current' | 'history'>('current')
  const [isDeleteAlertOpen, setIsDeleteAlertOpen] = useState(false)
  const [hasApprovePermission, setHasApprovePermission] = useState(false)

  const [sortField, setSortField] = useState<'date' | 'hours' | 'employee' | 'team'>('date')
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('desc')

  const normalizeDate = (dStr: string) => {
    if (!dStr) return ''
    try {
      const d = new Date(dStr)
      if (isNaN(d.getTime())) return dStr.split('T')[0]
      return d.toLocaleDateString('sv-SE')
    } catch {
      return dStr.split('T')[0]
    }
  }

  const normalizeTime = (tStr: string) => {
    if (!tStr) return '00:00'
    if (tStr.includes('T')) {
      try {
        const d = new Date(tStr)
        if (!isNaN(d.getTime())) {
          return d.toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit' })
        }
      } catch {
        return tStr.substring(11, 16)
      }
    }
    return tStr.substring(0, 5)
  }

  const fetchData = useCallback(async () => {
    if (!workspaceId && activeRole !== 'platform_admin') return

    setLoading(true)
    try {
      let reportsData: unknown[] = []
      let teamsData: TeamRow[] = []
      let workspacesData: WorkspaceRow[] = []
      let usersData: User[] = []

      const promises = []

      if (activeRole === 'platform_admin') {
        promises.push(
          supabase
            .from('workspaces')
            .select('*')
            .then(({ data }) => (workspacesData = data || [])),
        )
        promises.push(
          supabase
            .from('teams')
            .select('*')
            .then(({ data }) => (teamsData = data || [])),
        )
        promises.push(
          supabase
            .from('users')
            .select('*')
            .then(({ data }) => (usersData = data || [])),
        )
      } else {
        promises.push(
          supabase
            .from('teams')
            .select('*')
            .eq('workspace_id', workspaceId)
            .then(({ data }) => (teamsData = data || [])),
        )
        promises.push(
          supabase
            .from('users')
            .select('*')
            .eq('workspace_id', workspaceId)
            .then(({ data }) => (usersData = data || [])),
        )
      }

      await Promise.all(promises)
      setDbWorkspaces(workspacesData)
      setDbTeams(teamsData)
      setDbUsers(usersData)

      let query = supabase.from('time_reports').select('*, user:users(*)')

      if (activeRole === 'admin') {
        query = query.eq('workspace_id', workspaceId)
      } else if (activeRole === 'assistant') {
        query = query.eq('user_id', user?.id)
      }

      const { data: r, error } = await query
      if (error) throw error
      reportsData = r || []

      let unreportedEvents: any[] = []

      // Fetch past scheduled events
      if (user?.id) {
        const now = new Date().toISOString()
        let evQuery = supabase
          .from('events')
          .select('*')
          .eq('workspace_id', workspaceId)
          .lt('end_time', now)
          .order('start_time', { ascending: false })
          .limit(100)

        if (activeRole === 'assistant') {
          // Check for events assigned to user or created by user
          evQuery = evQuery.eq('assignee_id', user.id)
        }

        const { data: eventsData, error: evError } = await evQuery

        if (!evError && eventsData) {
          const reportedSet = new Set(
            (reportsData as any[]).map(
              (rep) =>
                `${rep.user_id}_${normalizeDate(rep.date)}_${normalizeTime(rep.start_time)}`
            )
          )

          // Filter out reported events
          unreportedEvents = eventsData.filter((e) => {
            const targetUser = e.assignee_id || e.user_id
            const d = normalizeDate(e.start_time)
            const t = normalizeTime(e.start_time)
            return !reportedSet.has(`${targetUser}_${d}_${t}`)
          })
        }
      }

      const mapped: TimeReportUI[] = (reportsData as Array<{
        id: string
        user_id: string
        workspace_id: string
        team_id: string
        date: string
        start_time: string
        end_time: string
        hours: number
        status: TimeReportUI['status']
        note: string | null
        user: User | User[]
      }>).map((dbShift) => {
        const team = teamsData.find((t) => t.id === dbShift.team_id)
        const teamName = team?.name || t('timereports.unassigned_team')
        const uData = Array.isArray(dbShift.user) ? dbShift.user[0] : dbShift.user
        const employeeName = uData
          ? uData.full_name || uData.email || t('common.unknown_agent')
          : t('common.unknown_agent')

        return {
          id: dbShift.id,
          employeeId: dbShift.user_id,
          employee: employeeName,
          role:
            uData?.role === 'admin'
              ? t('directory.roles.admin')
              : t('directory.roles.assistant'),
          teamId: dbShift.team_id,
          team: teamName,
          workspaceId: dbShift.workspace_id,
          date: normalizeDate(dbShift.date),
          start: normalizeTime(dbShift.start_time || '08:00'),
          end: normalizeTime(dbShift.end_time || '17:00'),
          duration: dbShift.hours || 0,
          break: 0,
          status: dbShift.status || 'pending_attest',
          location: '',
          note: dbShift.note || '',
        }
      })

      // Map unreported events to TimeReportUI
      const mappedEvents: TimeReportUI[] = unreportedEvents.map((e) => {
        const team = teamsData.find((t) => t.id === e.team_id)
        const teamName = team?.name || t('timereports.unassigned_team')

        const assignedUserId = e.assignee_id || e.user_id || user?.id || ''
        const assignedUser = dbUsers.find((u) => u.id === assignedUserId)
        const employeeName = assignedUser
          ? assignedUser.full_name || assignedUser.email || t('common.unknown_agent')
          : t('common.unknown_agent')

        const startObj = new Date(e.start_time)
        const endObj = new Date(e.end_time)

        let diffMs = endObj.getTime() - startObj.getTime()
        if (diffMs < 0) diffMs += 24 * 60 * 60 * 1000
        const hours = Math.round((diffMs / (1000 * 60 * 60)) * 100) / 100

        return {
          id: e.id,
          employeeId: assignedUserId,
          employee: employeeName,
          role:
            assignedUser?.role === 'admin'
              ? t('directory.roles.admin')
              : t('directory.roles.assistant'),
          teamId: e.team_id,
          team: teamName,
          workspaceId: e.workspace_id,
          date: normalizeDate(e.start_time),
          start: normalizeTime(e.start_time),
          end: normalizeTime(e.end_time),
          duration: hours,
          break: 0,
          status: 'not_submitted',
          location: '',
          note: e.title || '',
        }
      })

      setShifts([...mappedEvents, ...mapped])
    } catch (error: unknown) {
      console.error('Error fetching data:', error)
      const message = error instanceof Error ? error.message : t('common.unknown_error')
      toast.error(t('timereports.error_fetching_data') + ': ' + message)
    } finally {
      setLoading(false)
    }
  }, [workspaceId, activeRole, user?.id, t])

  useEffect(() => {
    fetchData().catch(console.error)

    const channel = supabase
      .channel('timemanager-reports')
      .on('postgres_changes', { event: '*', schema: 'public', table: 'time_reports' }, () => {
        fetchData()
      })
      .on('postgres_changes', { event: '*', schema: 'public', table: 'teams' }, () => {
        fetchData()
      })
      .on('postgres_changes', { event: '*', schema: 'public', table: 'workspaces' }, () => {
        fetchData()
      })
      .subscribe()

    return () => {
      supabase.removeChannel(channel)
    }
  }, [fetchData])

  useEffect(() => {
    if (activeRole === 'platform_admin') {
      setCurrentLevel('platform_overview')
      setSelectedContext({ type: null, id: null })
    } else if (activeRole === 'assistant') {
      setCurrentLevel('shift_list')
      setSelectedContext({ type: 'employee', id: user?.id || null })
    } else {
      setCurrentLevel('team_overview')
      setSelectedContext({ type: null, id: null })
    }
  }, [activeRole, user?.id])

  useEffect(() => {
    async function checkPerms() {
      if (!user) return
      if (activeRole === 'admin' || activeRole === 'platform_admin') {
        setHasApprovePermission(true)
        return
      }

      const { data: memberships } = await supabase
        .from('team_members')
        .select('role_id')
        .eq('user_id', user.id)
      if (memberships) {
        const roleIds = memberships.map((m) => m.role_id).filter(Boolean)
        if (roleIds.length > 0) {
          const { data: roles } = await supabase
            .from('workspace_roles')
            .select('permissions')
            .in('id', roleIds)
          if (
            roles?.some((r) => (r.permissions as Record<string, boolean>)?.can_approve_time_reports)
          ) {
            setHasApprovePermission(true)
            return
          }
        }
      }
      setHasApprovePermission(false)
    }
    checkPerms()
  }, [activeRole, user])

  const openEmployeeShifts = useCallback((empId: string) => {
    setSelectedContext({ type: 'employee', id: empId })
    setCurrentLevel('shift_list')
    setFilterType('all')
    setSearchQuery('')
    setListMode('current')
  }, [])

  const openTeamShifts = useCallback((teamName: string) => {
    setSelectedContext({ type: 'team', id: teamName })
    setCurrentLevel('shift_list')
    setFilterType('all')
    setSearchQuery('')
    setListMode('current')
  }, [])

  const openWorkspaceOverview = useCallback((_wsId: string) => {
    void _wsId
    setCurrentLevel('team_overview')
  }, [])

  const navigateBack = useCallback(() => {
    if (currentLevel === 'shift_list') {
      setSelectedContext({ type: null, id: null })
      if (activeRole === 'platform_admin') {
        setCurrentLevel('platform_overview')
      } else if (activeRole === 'assistant') {
        setCurrentLevel('assistant_teams')
      } else {
        setCurrentLevel('team_overview')
      }
    } else if (currentLevel === 'team_overview' && activeRole === 'platform_admin') {
      setCurrentLevel('platform_overview')
    }
  }, [currentLevel, activeRole])

  const handleApprove = async (shiftIds?: string[]) => {
    const ids = shiftIds && shiftIds.length > 0 ? shiftIds : selectedShifts
    if (ids.length === 0) return

    const { error } = await supabase
      .from('time_reports')
      .update({ status: 'approved' })
      .in('id', ids)
    if (error) {
      toast.error(t('timereports.error_approving') + ': ' + error.message)
      return
    }

    toast.success(t('timereports.success_approved', { count: ids.length }))
    setSelectedShifts((prev) => prev.filter((id) => !ids.includes(id)))
    fetchData()
  }

  const handleApproveSingle = async (shiftId: string) => {
    return handleApprove([shiftId])
  }

  const handleReport = async (shiftIds?: string[]) => {
    const ids = shiftIds && shiftIds.length > 0 ? shiftIds : selectedShifts
    if (ids.length === 0 || !user?.id || !workspaceId) return

    // Find the shifts in our list that match the selected IDs
    const shiftsToReport = shifts.filter(
      (s) => ids.includes(s.id) && s.status === 'not_submitted'
    )
    if (shiftsToReport.length === 0) return

    const inserts = shiftsToReport.map((s) => ({
      workspace_id: workspaceId,
      user_id: s.employeeId,
      team_id: s.teamId || null,
      date: s.date,
      start_time: s.start,
      end_time: s.end,
      hours: s.duration,
      status: 'pending_attest',
      note: s.note ? `Schemalagt pass: ${s.note}` : 'Schemalagt pass',
    }))

    const { error } = await supabase.from('time_reports').insert(inserts)
    if (error) {
      toast.error(t('timereports.error_reporting') + ': ' + error.message)
      return
    }

    toast.success(t('timereports.success_reported'))
    setSelectedShifts((prev) => prev.filter((id) => !ids.includes(id)))
    fetchData()
  }

  const handleReportSingle = async (shiftId: string) => {
    return handleReport([shiftId])
  }

  const handleManualReport = async (data: {
    teamId?: string | null
    date: string
    start: string
    end: string
    hours: number
    note?: string
  }) => {
    if (!user?.id || !workspaceId) return false

    const insert = {
      workspace_id: workspaceId,
      user_id: user.id,
      team_id: data.teamId || null,
      date: data.date,
      start_time: data.start,
      end_time: data.end,
      hours: data.hours,
      status: 'pending_attest',
      note: data.note || null,
    }

    const { error } = await supabase.from('time_reports').insert([insert])
    if (error) {
      toast.error(t('timereports.error_reporting') + ': ' + error.message)
      return false
    }

    toast.success(t('timereports.success_reported'))
    fetchData()
    return true
  }

  const handleDelete = async () => {
    if (selectedShifts.length === 0) return

    const { error } = await supabase.from('time_reports').delete().in('id', selectedShifts)
    if (error) {
      toast.error(t('timereports.error_deleting') + ': ' + error.message)
      return
    }

    toast.success(t('timereports.success_deleted', { count: selectedShifts.length }))
    setSelectedShifts([])
    setIsDeleteAlertOpen(false)
    fetchData()
  }

  return {
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
    sortField,
    sortDirection,
    currentPage,
    selectedShifts,
    listMode,
    isDeleteAlertOpen,
    hasApprovePermission,
    setListMode,
    setFilterType,
    setSearchQuery,
    setSortField,
    setSortDirection,
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
  }
}
