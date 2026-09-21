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
          evQuery = evQuery.eq('assignee_id', user.id)
        }

        const { data: eventsData, error: evError } = await evQuery
          
        if (!evError && eventsData) {
          const reportedSet = new Set(
            (reportsData as any[]).map(r => `${r.date}_${r.start_time}`)
          )
          // Filter out reported events
          unreportedEvents = eventsData.filter(e => {
            const d = e.start_time.split('T')[0]
            const t = new Date(e.start_time).toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit' })
            return !reportedSet.has(`${d}_${t}`)
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
        const employeeName = uData ? uData.full_name || uData.email || t('common.unknown_agent') : t('common.unknown_agent')

        return {
          id: dbShift.id,
          employeeId: dbShift.user_id,
          employee: employeeName,
          role: uData?.role === 'admin' ? t('directory.roles.admin') : t('directory.roles.assistant'),
          teamId: dbShift.team_id,
          team: teamName,
          workspaceId: dbShift.workspace_id,
          date: new Date(dbShift.date).toLocaleDateString('sv-SE'),
          start: dbShift.start_time || '08:00',
          end: dbShift.end_time || '17:00',
          duration: dbShift.hours || 0,
          break: 0,
          status: dbShift.status,
          location: '',
          note: dbShift.note || '',
        }
      })

      // Map unreported events to TimeReportUI
      const mappedEvents: TimeReportUI[] = unreportedEvents.map(e => {
        const team = teamsData.find((t) => t.id === e.team_id)
        const teamName = team?.name || t('timereports.unassigned_team')
        
        // Let's find the user name from dbUsers based on assignee_id
        const assignedUser = dbUsers.find(u => u.id === e.assignee_id)
        const employeeName = assignedUser ? (assignedUser.full_name || assignedUser.email || t('common.unknown_agent')) : t('common.unknown_agent')
        
        const startObj = new Date(e.start_time)
        const endObj = new Date(e.end_time)
        
        let diffMs = endObj.getTime() - startObj.getTime()
        if (diffMs < 0) diffMs += 24 * 60 * 60 * 1000
        const hours = Math.round((diffMs / (1000 * 60 * 60)) * 100) / 100

        return {
          id: e.id, // Using event ID here to identify it later when submitting
          employeeId: e.assignee_id,
          employee: employeeName,
          role: t('directory.roles.assistant'),
          teamId: e.team_id,
          team: teamName,
          workspaceId: e.workspace_id,
          date: startObj.toLocaleDateString('sv-SE'),
          start: startObj.toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit' }),
          end: endObj.toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit' }),
          duration: hours,
          break: 0,
          status: 'not_submitted', // Special status
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
    } else {
      setCurrentLevel('team_overview')
    }
    setSelectedContext({ type: null, id: null })
  }, [activeRole])

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

  const handleApprove = async () => {
    if (selectedShifts.length === 0) return

    const { error } = await supabase
      .from('time_reports')
      .update({ status: 'approved' })
      .in('id', selectedShifts)
    if (error) {
      toast.error(t('timereports.error_approving') + ': ' + error.message)
      return
    }

    toast.success(t('timereports.success_approved', { count: selectedShifts.length }))
    setSelectedShifts([])
    fetchData()
  }

  const handleReport = async () => {
    if (selectedShifts.length === 0 || !user?.id || !workspaceId) return

    // Find the shifts in our list that match the selected IDs
    const shiftsToReport = shifts.filter(s => selectedShifts.includes(s.id) && s.status === 'not_submitted')
    if (shiftsToReport.length === 0) return

    const inserts = shiftsToReport.map(s => ({
      workspace_id: workspaceId,
      user_id: s.employeeId, // <--- Correctly attribute to the assigned employee
      team_id: s.teamId || null,
      date: new Date(s.date).toISOString().split('T')[0],
      start_time: s.start,
      end_time: s.end,
      hours: s.duration,
      status: 'pending_attest',
      note: `Schemalagt pass: ${s.note}`
    }))

    const { error } = await supabase.from('time_reports').insert(inserts)
    if (error) {
      toast.error(t('timereports.error_reporting') + ': ' + error.message)
      return
    }

    toast.success(t('timereports.success_reported'))
    setSelectedShifts([])
    fetchData()
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
  }
}
