import { useState, useEffect } from 'react'
import { supabase } from '@/lib/supabase'
import { useAuth } from './useAuth'

export interface TeamPermissions {
  can_manage_schedule: boolean
  can_manage_notes: boolean
  can_approve_time_reports: boolean
  is_admin: boolean
}

const DEFAULT_PERMISSIONS: TeamPermissions = {
  can_manage_schedule: false,
  can_manage_notes: false,
  can_approve_time_reports: false,
  is_admin: false,
}

const FULL_PERMISSIONS: TeamPermissions = {
  can_manage_schedule: true,
  can_manage_notes: true,
  can_approve_time_reports: true,
  is_admin: true,
}

export const useTeamPermissions = (teamId: string | null) => {
  const { user } = useAuth()
  const [permissions, setPermissions] = useState<TeamPermissions>(DEFAULT_PERMISSIONS)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!user) {
      setPermissions(DEFAULT_PERMISSIONS)
      setLoading(false)
      return
    }

    const uRole = user.role
    if (uRole === 'platform_admin' || uRole === 'admin') {
      setPermissions(FULL_PERMISSIONS)
      setLoading(false)
      return
    }

    if (!teamId) {
      setPermissions(DEFAULT_PERMISSIONS)
      setLoading(false)
      return
    }

    const fetchTeamRole = async () => {
      try {
        setLoading(true)
        const { data: tmData } = await supabase
          .from('team_members')
          .select('role_id')
          .eq('team_id', teamId)
          .eq('user_id', user.id)
          .single()

        if (tmData && tmData.role_id) {
          const { data: roleData } = await supabase
            .from('workspace_roles')
            .select('permissions')
            .eq('id', tmData.role_id)
            .single()

          if (roleData && roleData.permissions) {
            setPermissions({
              can_manage_schedule: !!roleData.permissions.can_manage_schedule,
              can_manage_notes: !!roleData.permissions.can_manage_notes,
              can_approve_time_reports: !!roleData.permissions.can_approve_time_reports,
              is_admin: false,
            })
            setLoading(false)
            return
          }
        }

        setPermissions(DEFAULT_PERMISSIONS)
      } catch {
        setPermissions(DEFAULT_PERMISSIONS)
      } finally {
        setLoading(false)
      }
    }

    fetchTeamRole()
  }, [teamId, user])

  return { permissions, loading }
}
