import React, { createContext, useContext, useState, useEffect, useMemo } from 'react'
import { useAuth } from '@/hooks/useAuth'
import i18n from 'i18next'
import { useWorkspaceInfo, useUserPreferences } from '@/hooks/queries/useWorkspaceData'
import { userService } from '@/services/userService'
import { workspaceService } from '@/services/workspaceService'
import type { WorkspaceModules, WorkspaceSettings, UserPreferences, Workspace } from '@/types'

interface WorkspaceState {
  workspaceId: string | null
  workspaceName: string
  workspaceLogo: string | null
  brandColor: string
  modules: WorkspaceModules
  blockSettings: Record<string, any>
  settings: WorkspaceSettings
  preferences: UserPreferences
  isLoading: boolean
  selectedTeamId: string | null
  updateModules: (newModules: Partial<WorkspaceModules>) => Promise<boolean>
  updateBlockSettings: (newSettings: Record<string, any>) => Promise<boolean>
  updateSettings: (newSettings: Partial<WorkspaceSettings>) => Promise<boolean>
  updatePreferences: (newPreferences: Partial<UserPreferences>) => Promise<boolean>
  updateWorkspace: (updates: {
    name?: string
    logo_url?: string | null
    brand_color?: string
  }) => Promise<boolean>
  setAdminWorkspace: (id: string) => void
  setSelectedTeamId: (id: string | null) => void
}

const defaultModules: WorkspaceModules = {
  school: false,
  assistance: true,
  messaging: true,
  scheduling: true,
  notes: true,
  time: true,
  directory: true,
  reporting: true,
}

const defaultSettings: WorkspaceSettings = {
  timezone: 'Europe/Stockholm',
  week_start: 1,
  language: 'sv',
  business_hours: { start: 0, end: 23 },
}

const defaultPreferences: UserPreferences = {
  theme: 'system',
  calendar_density: 'relaxed',
  font_scale: 1.0,
}

const WorkspaceContext = createContext<WorkspaceState>({
  workspaceId: null,
  workspaceName: '',
  workspaceLogo: null,
  brandColor: '#3b82f6',
  modules: defaultModules,
  blockSettings: {},
  settings: defaultSettings,
  preferences: defaultPreferences,
  isLoading: true,
  selectedTeamId: null,
  updateModules: async () => false,
  updateBlockSettings: async () => false,
  updateSettings: async () => false,
  updatePreferences: async () => false,
  updateWorkspace: async () => false,
  setAdminWorkspace: () => {},
  setSelectedTeamId: () => {},
})

export const WorkspaceProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { user } = useAuth()
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(() => {
    return localStorage.getItem('yntra_active_workspace_id')
  })
  const [selectedTeamId, setSelectedTeamIdState] = useState<string | null>(() => {
    return localStorage.getItem('yntra_selected_team_id')
  })
  const [internalLoading, setInternalLoading] = useState(true)

  useEffect(() => {
    if (selectedTeamId) {
      localStorage.setItem('yntra_selected_team_id', selectedTeamId)
    } else {
      localStorage.removeItem('yntra_selected_team_id')
    }
  }, [selectedTeamId])

  useEffect(() => {
    if (activeWorkspaceId) {
      localStorage.setItem('yntra_active_workspace_id', activeWorkspaceId)
    } else {
      localStorage.removeItem('yntra_active_workspace_id')
    }
  }, [activeWorkspaceId])

  useEffect(() => {
    async function resolveWorkspace() {
      if (!user) {
        setActiveWorkspaceId(null)
        setInternalLoading(false)
        return
      }

      try {
        const workspaceId = await userService.getUserWorkspaceId(user.id)

        if (workspaceId) {
          setActiveWorkspaceId(workspaceId)
        } else if (user.role === 'platform_admin') {
          const firstWs = await workspaceService.getFirstWorkspace()
          if (firstWs) setActiveWorkspaceId(firstWs)
        }
      } catch (error) {
        console.error('Failed to resolve workspace:', error)
      } finally {
        setInternalLoading(false)
      }
    }
    resolveWorkspace()
  }, [user])

  const {
    data: workspaceInfoRaw,
    isLoading: wsLoading,
    updateSettings: mutateSettings,
    updateModules: mutateModules,
    updateBlockSettings: mutateBlockSettings,
    updateWorkspace: mutateWorkspace,
  } = useWorkspaceInfo(activeWorkspaceId)

  const workspaceInfo = workspaceInfoRaw as unknown as Workspace | null

  const {
    data: userPrefs,
    isLoading: prefsLoading,
    updatePreferences: mutatePreferences,
  } = useUserPreferences(user?.id || null)

  const modules = useMemo(() => {
    if (!workspaceInfo?.modules_active) return defaultModules
    const active = workspaceInfo.modules_active as any
    return {
      school: !!active.school,
      assistance: active.assistance !== false, // Default to true if not specified
      messaging: active.messaging !== false,
      scheduling: active.scheduling !== false,
      notes: active.notes !== false,
      time: active.time !== false,
      directory: active.directory !== false,
      reporting: active.reporting !== false,
    }
  }, [workspaceInfo?.modules_active])
  
  const blockSettings = useMemo(() => {
    return workspaceInfo?.block_settings || {}
  }, [workspaceInfo?.block_settings])

  const settings = useMemo(() => {
    const fetched = (workspaceInfo?.settings as unknown as WorkspaceSettings) || {}
    return {
      ...defaultSettings,
      ...fetched,
      business_hours: {
        ...defaultSettings.business_hours,
        ...(fetched.business_hours || {}),
      },
    }
  }, [workspaceInfo?.settings])

  const preferences = useMemo(() => {
    const fetched = (userPrefs as unknown as UserPreferences) || {}
    return {
      ...defaultPreferences,
      ...fetched,
    }
  }, [userPrefs])

  useEffect(() => {
    if (settings.language && i18n.language !== settings.language) {
      i18n.changeLanguage(settings.language)
    }
  }, [settings.language])

  const isLoading = internalLoading || wsLoading || prefsLoading

  const updateModules = async (newModules: Partial<WorkspaceModules>) => {
    try {
      await mutateModules({ ...modules, ...newModules })
      return true
    } catch (e) {
      console.error(e)
      return false
    }
  }

  const updateBlockSettings = async (newSettings: Record<string, any>) => {
    try {
      await mutateBlockSettings({ ...blockSettings, ...newSettings })
      return true
    } catch (e) {
      console.error(e)
      return false
    }
  }

  const updateSettings = async (newSettings: Partial<WorkspaceSettings>) => {
    try {
      const merged = { ...settings, ...newSettings }
      await mutateSettings(merged)
      return true
    } catch (e) {
      console.error('Failed to update settings:', e)
      return false
    }
  }

  const updatePreferences = async (newPrefs: Partial<UserPreferences>) => {
    try {
      const merged = { ...preferences, ...newPrefs }
      await mutatePreferences(merged)
      return true
    } catch (e) {
      console.error('Failed to update preferences:', e)
      return false
    }
  }

  const updateWorkspace = async (updates: {
    name?: string
    logo_url?: string | null
    brand_color?: string
  }) => {
    try {
      await mutateWorkspace(updates)
      return true
    } catch (e) {
      console.error('Failed to update workspace:', e)
      return false
    }
  }

  const setAdminWorkspace = (id: string) => {
    if (user?.role === 'platform_admin') {
      setActiveWorkspaceId(id)
    }
  }

  return (
    <WorkspaceContext.Provider
      value={{
        workspaceId: activeWorkspaceId,
        workspaceName: workspaceInfo?.name || '',
        workspaceLogo: workspaceInfo?.logo_url || null,
        brandColor: workspaceInfo?.brand_color || '#3b82f6',
        modules,
        blockSettings,
        settings,
        preferences,
        isLoading,
        selectedTeamId,
        updateModules,
        updateBlockSettings,
        updateSettings,
        updatePreferences,
        updateWorkspace,
        setAdminWorkspace,
        setSelectedTeamId: setSelectedTeamIdState,
      }}
    >
      {children}
    </WorkspaceContext.Provider>
  )
}

export const useWorkspace = () => useContext(WorkspaceContext)
