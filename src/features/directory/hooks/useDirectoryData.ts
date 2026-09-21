import { useState, useEffect, useCallback, useRef } from 'react'
import { supabase } from '@/lib/supabase'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'

export type DirectoryLevel = 'workspaces' | 'teams' | 'members'

export interface WorkspaceItem {
  id: string
  name: string
  type: string
  teamsCount: number
  membersCount: number
}

export interface TeamItem {
  id: string
  name: string
  workspaceId: string
  membersCount: number
  patientsCount: number
  leader: string
}

export interface ClientItem {
  id: string
  firstName: string
  lastName: string
  personalNumber: string
  careLevel: string
  teamId: string | null
  workspaceId: string
  location?: string
  address?: string
  avatar?: string
}

export interface MemberItem {
  id: string
  name: string
  email: string
  ssn: string
  teamId: string | null
  phone: string
  location?: string
  address: string
  careLevel: string
  avatar: string
  alerts: string[]
  notes: string
  upcomingVisits: { id: string; date: string; title: string }[]
  roleId: string | null
  role: string
  privacy_settings?: import('@/types').UserPrivacySettings
}

interface TeamMemberLink {
  team_id: string
  user_id: string
  role_id: string | null
}

interface UserDbRow {
  id: string
  full_name: string | null
  email: string
  phone: string | null
  location: string | null
  privacy_settings: import('@/types').UserPrivacySettings | null
  role: string | null
  workspace_id: string
}

export interface WorkspaceRole {
  id: string
  name: string
  workspace_id: string
  permissions: {
    can_manage_schedule: boolean
    can_manage_notes: boolean
    can_approve_time_reports: boolean
  }
}

import { useSearchParams } from 'react-router-dom'

export const useDirectoryData = () => {
  const { workspaceId, setAdminWorkspace } = useWorkspace()
  const { user } = useAuth()
  const [searchParams] = useSearchParams()
  const userRole = (user?.role as 'platform_admin' | 'admin' | 'assistant') || 'admin'

  const initialTeam = searchParams.get('team')
  const initialUser = searchParams.get('user')

  const [currentLevel, setCurrentLevel] = useState<DirectoryLevel>(
    initialUser || initialTeam ? 'members' : userRole === 'platform_admin' ? 'workspaces' : 'teams',
  )
  const [selectedWorkspace, setSelectedWorkspace] = useState<string | null>(null)
  const [selectedTeam, setSelectedTeam] = useState<string | null>(
    initialTeam || (initialUser ? 'all_members' : null),
  )
  const [selectedEntity, setSelectedEntity] = useState<
    WorkspaceItem | TeamItem | MemberItem | null
  >(null)

  const [dbWorkspaces, setDbWorkspaces] = useState<WorkspaceItem[]>([])
  const [dbTeams, setDbTeams] = useState<TeamItem[]>([])
  const [dbMembers, setDbMembers] = useState<MemberItem[]>([])
  const [dbWorkspaceRoles, setDbWorkspaceRoles] = useState<WorkspaceRole[]>([])
  const dbWorkspaceRolesRef = useRef<WorkspaceRole[]>([])
  const [dbClients, setDbClients] = useState<ClientItem[]>([])
  const [membersCache, setMembersCache] = useState<Record<string, MemberItem[]>>({})
  const membersCacheRef = useRef<Record<string, MemberItem[]>>({})

  useEffect(() => {
    if (initialUser && dbMembers.length > 0) {
      const target = dbMembers.find((m) => m.id === initialUser)
      if (target) {
        requestAnimationFrame(() => {
          setSelectedEntity(target)
        })
      }
    }
  }, [initialUser, dbMembers])

  useEffect(() => {
    if (userRole === 'platform_admin' && !selectedWorkspace && currentLevel === 'teams') {
      const timer = setTimeout(() => {
        setCurrentLevel('workspaces')
      }, 0)
      return () => clearTimeout(timer)
    }
  }, [userRole, selectedWorkspace, currentLevel])

  const loadDirectory = useCallback(async () => {
    const filterWs = selectedWorkspace || workspaceId
    let currentRoles: WorkspaceRole[] = []

    if (userRole === 'platform_admin') {
      const { data: wsData } = await supabase
        .from('workspaces')
        .select('*, teams(count), users(count)')
      if (wsData) {
        setDbWorkspaces(
          wsData.map((w) => ({
            id: w.id,
            name: w.name,
            type: 'Assistance',
            teamsCount: w.teams?.[0]?.count || 0,
            membersCount: w.users?.[0]?.count || 0,
          })),
        )
      }
    }

    if (filterWs) {
      const { data: rolesData } = await supabase
        .from('workspace_roles')
        .select('*')
        .eq('workspace_id', filterWs)
      currentRoles = rolesData || []
      if (rolesData) {
        setDbWorkspaceRoles(rolesData)
        dbWorkspaceRolesRef.current = rolesData
      }

      const { data: teamData } = await supabase
        .from('teams')
        .select('*')
        .eq('workspace_id', filterWs)
      const { data: clientData } = await supabase
        .from('clients')
        .select('*')
        .eq('workspace_id', filterWs)
      const { data: allUsers } = await supabase
        .from('users')
        .select('id, role')
        .eq('workspace_id', filterWs)
      const teamIds = teamData?.map((t) => t.id) || []
      let allMemberships: TeamMemberLink[] = []
      if (teamIds.length > 0) {
        const { data: memberships } = await supabase
          .from('team_members')
          .select('team_id, user_id, role_id')
          .in('team_id', teamIds)
        allMemberships = (memberships as TeamMemberLink[]) || []
      }

      if (teamData) {
        setDbTeams(
          teamData.map((t) => {
            const teamLinks = allMemberships.filter((tm) => tm.team_id === t.id)
            const assistantLinks = teamLinks.filter((tm) => {
              const u = allUsers?.find((u) => u.id === tm.user_id)
              return u && u.role === 'assistant' && !tm.role_id
            })

            const teamClients = (clientData || []).filter((c) => c.team_id === t.id)

            return {
              id: t.id,
              name: t.name,
              workspaceId: t.workspace_id,
              membersCount: assistantLinks.length,
              patientsCount: teamClients.length,
              leader: 'Unknown',
            }
          }),
        )
      }

      if (clientData) {
        setDbClients(
          clientData.map((c) => ({
            id: c.id,
            firstName: c.first_name,
            lastName: c.last_name,
            personalNumber: c.personal_number || '',
            careLevel: c.care_level || 'normal',
            teamId: c.team_id,
            workspaceId: c.workspace_id,
            location: c.location,
            address: c.address,
            avatar: c.avatar,
          })),
        )
      }
    }

    if (selectedTeam) {
      if (membersCacheRef.current[selectedTeam]) {
        setDbMembers(membersCacheRef.current[selectedTeam])
      }

      let memberIds: string[] = []
      let teamMembersLinkData: TeamMemberLink[] = []
      if (selectedTeam !== 'all_members') {
        const { data: tmData } = await supabase
          .from('team_members')
          .select('user_id, role_id')
          .eq('team_id', selectedTeam)
        teamMembersLinkData = (tmData as TeamMemberLink[]) || []
        memberIds = teamMembersLinkData.map((tm) => tm.user_id)
      }

      const { data: usersData } = await supabase
        .from('users')
        .select('*')
        .eq('workspace_id', filterWs)

      const actualTeamUsers =
        selectedTeam === 'all_members'
          ? usersData || []
          : (usersData || []).filter((u) => memberIds.includes(u.id))

      const mappedUsers = actualTeamUsers.map((u: UserDbRow) => {
        let customRoleName: string | null = null
        let customRoleId: string | null = null
        if (selectedTeam !== 'all_members') {
          const link = teamMembersLinkData.find((tm) => tm.user_id === u.id)
          if (link?.role_id) {
            customRoleId = link.role_id
            const roleDef = (currentRoles.length > 0 ? currentRoles : dbWorkspaceRolesRef.current).find(
              (r: WorkspaceRole) => r.id === customRoleId,
            )
            if (roleDef) customRoleName = roleDef.name
          }
        }

        return {
          id: u.id,
          name: u.full_name || u.email,
          email: u.email,
          ssn: '',
          teamId: selectedTeam,
          phone: u.phone || '',
          location: u.location || '',
          privacy_settings: u.privacy_settings || undefined,
          address: '',
          careLevel: '',
          avatar: '',
          alerts: [],
          notes: '',
          upcomingVisits: [],
          roleId: customRoleId,
          role: customRoleName || u.role || 'Assistant',
        }
      })

      setDbMembers(mappedUsers)
      setMembersCache((prev) => {
        const next = { ...prev, [selectedTeam]: mappedUsers }
        membersCacheRef.current = next
        return next
      })
    }
  }, [selectedWorkspace, workspaceId, selectedTeam, userRole])

  useEffect(() => {
    const initTimer = setTimeout(() => {
      loadDirectory().catch(console.error)
    }, 0)

    const channel = supabase
      .channel('directory-auto-update')
      .on('postgres_changes', { event: '*', schema: 'public', table: 'workspaces' }, () => {
        loadDirectory()
      })
      .on('postgres_changes', { event: '*', schema: 'public', table: 'teams' }, () => {
        loadDirectory()
      })
      .on('postgres_changes', { event: '*', schema: 'public', table: 'users' }, () => {
        loadDirectory()
      })
      .on('postgres_changes', { event: '*', schema: 'public', table: 'team_members' }, () => {
        loadDirectory()
      })
      .on('postgres_changes', { event: '*', schema: 'public', table: 'workspace_roles' }, () => {
        loadDirectory()
      })
      .on('postgres_changes', { event: '*', schema: 'public', table: 'clients' }, () => {
        loadDirectory()
      })
      .subscribe()

    return () => {
      clearTimeout(initTimer)
      supabase.removeChannel(channel)
    }
  }, [loadDirectory])

  const handleSelectWorkspace = useCallback((wId: string) => {
    setSelectedWorkspace(wId)
    if (userRole === 'platform_admin' && setAdminWorkspace) {
      setAdminWorkspace(wId)
    }
    setCurrentLevel('teams')
  }, [userRole, setAdminWorkspace])

  const handleSelectTeam = useCallback((tId: string) => {
    setSelectedTeam(tId)
    setCurrentLevel('members')
  }, [])

  const handleBreadcrumbClick = useCallback((level: DirectoryLevel) => {
    if (level === 'workspaces') {
      setSelectedWorkspace(null)
      setSelectedTeam(null)
      setCurrentLevel('workspaces')
    } else if (level === 'teams') {
      setSelectedTeam(null)
      setCurrentLevel('teams')
    }
  }, [])

  const prefetchTeamMembers = useCallback(
    async (tId: string) => {
      if (membersCache[tId]) return

      const filterWs = selectedWorkspace || workspaceId
      if (!filterWs) return

      let memberIds: string[] = []
      if (tId !== 'all_members') {
        const { data: tmData } = await supabase
          .from('team_members')
          .select('user_id')
          .eq('team_id', tId)
        memberIds = (tmData || []).map((tm) => tm.user_id)
      }

      const { data: usersData } = await supabase
        .from('users')
        .select('*')
        .eq('workspace_id', filterWs)
      const actualTeamUsers =
        tId === 'all_members'
          ? usersData || []
          : (usersData || []).filter((u) => memberIds.includes(u.id))

      const mappedUsers = actualTeamUsers.map((u: UserDbRow) => ({
        id: u.id,
        name: u.full_name || u.email,
        email: u.email,
        ssn: '',
        teamId: tId,
        phone: u.phone || '',
        location: u.location || '',
        privacy_settings: u.privacy_settings || undefined,
        address: '',
        careLevel: '',
        avatar: '',
        alerts: [],
        notes: '',
        upcomingVisits: [],
        roleId: null,
        role: u.role || 'Assistant',
      }))

      setMembersCache((prev) => ({ ...prev, [tId]: mappedUsers }))
    },
    [membersCache, selectedWorkspace, workspaceId],
  )

  return {
    userRole,
    currentLevel,
    setCurrentLevel,
    selectedWorkspace,
    setSelectedWorkspace,
    selectedTeam,
    setSelectedTeam,
    selectedEntity,
    setSelectedEntity,
    dbWorkspaces,
    setDbWorkspaces,
    dbTeams,
    setDbTeams,
    dbMembers,
    setDbMembers,
    dbWorkspaceRoles,
    setDbWorkspaceRoles,
    dbClients,
    setDbClients,
    handleSelectWorkspace,
    handleSelectTeam,
    handleBreadcrumbClick,
    prefetchTeamMembers,
    loadDirectory,
    workspaceId,
  }
}
