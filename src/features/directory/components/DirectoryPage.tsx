import React, { useState } from 'react'

import { useDirectoryData, type MemberItem } from '../hooks/useDirectoryData'
import { WorkspacesView } from './WorkspacesView'
import { TeamsView } from './TeamsView'
import { MembersView } from './MembersView'
import { MemberDetailSheet } from './MemberDetailSheet'
import { DevHubModal } from './DevHubModal'
import { RoleManagerModal } from './RoleManagerModal'
import { TeamManagerModal } from './TeamManagerModal'
import { InviteManagerModal } from './InviteManagerModal'
import { AdminInviteModal } from './AdminInviteModal'
import { ClientManagerModal } from './ClientManagerModal'
import { EditMemberModal } from './EditMemberModal'
import { useSetDynamicBreadcrumbs } from '@/contexts/BreadcrumbContext'
import { useEffect } from 'react'
import { useAuth } from '@/hooks/useAuth'
import { useTranslation } from 'react-i18next'

export const DirectoryPage: React.FC = () => {
  const { t } = useTranslation()
  const {
    userRole,
    currentLevel,
    selectedWorkspace,
    selectedTeam,
    selectedEntity,
    setSelectedEntity,
    dbWorkspaces,
    setDbWorkspaces,
    dbTeams,
    dbMembers,
    dbWorkspaceRoles,
    dbClients,
    handleSelectWorkspace,
    handleSelectTeam,
    handleBreadcrumbClick,
    prefetchTeamMembers,
    loadDirectory,
    workspaceId,
  } = useDirectoryData()

  const { user } = useAuth()

  const [isHubOpen, setIsHubOpen] = useState(false)
  const [isRoleManagerOpen, setIsRoleManagerOpen] = useState(false)
  const [isTeamManagerOpen, setIsTeamManagerOpen] = useState(false)
  const [isInviteManagerOpen, setIsInviteManagerOpen] = useState(false)
  const [isAdminInviteOpen, setIsAdminInviteOpen] = useState(false)
  const [isClientManagerOpen, setIsClientManagerOpen] = useState(false)
  const [isEditMemberOpen, setIsEditMemberOpen] = useState(false)
  const [memberToEdit, setMemberToEdit] = useState<MemberItem | null>(null)
  const [clientToEdit, setClientToEdit] = useState<
    import('../hooks/useDirectoryData').ClientItem | null
  >(null)

  const setDynamicBreadcrumbs = useSetDynamicBreadcrumbs()

  useEffect(() => {
    const breadcrumbs = []
    if (userRole === 'platform_admin' && (currentLevel === 'teams' || currentLevel === 'members')) {
      const workspace = dbWorkspaces.find((w) => w.id === selectedWorkspace)
      if (workspace) {
        breadcrumbs.push({
          label: workspace.name,
          onClick: () => {
            setSelectedEntity(null)
            handleSelectWorkspace(workspace.id)
          },
        })
      }
    }
    if (currentLevel === 'members') {
      if (selectedTeam === 'all_members') {
        breadcrumbs.push({
          label: t('directory.levels.all_members', 'Alla konton i organisationen'),
          onClick: () => {
            setSelectedEntity(null)
            handleSelectTeam('all_members')
          },
        })
      } else {
        const team = dbTeams.find((t) => t.id === selectedTeam)
        if (team) {
          breadcrumbs.push({
            label: team.name,
            onClick: () => {
              setSelectedEntity(null)
              handleSelectTeam(team.id)
            },
          })
        }
      }
    }
    if (selectedEntity) {
      breadcrumbs.push({
        label: (selectedEntity as MemberItem).name || (selectedEntity as MemberItem).email,
      })
    }
    setDynamicBreadcrumbs(breadcrumbs)
  }, [
    currentLevel,
    selectedWorkspace,
    selectedTeam,
    selectedEntity,
    dbWorkspaces,
    dbTeams,
    userRole,
    setDynamicBreadcrumbs,
    handleSelectWorkspace,
    handleSelectTeam,
    setSelectedEntity,
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
      if (customEvent.detail?.path === '/directory') {
        handleBreadcrumbClick(userRole === 'platform_admin' ? 'workspaces' : 'teams')
      }
    }
    window.addEventListener('yntra:breadcrumb-navigate', handleReset)
    return () => {
      window.removeEventListener('yntra:breadcrumb-navigate', handleReset)
    }
  }, [userRole, handleBreadcrumbClick])

  // Auto-navigate clients to their team
  useEffect(() => {
    if (
      user?.role === 'client' &&
      dbClients.length > 0 &&
      (currentLevel === 'workspaces' || currentLevel === 'teams')
    ) {
      // If we have a client_id, use it.
      // Otherwise (for simulation/testing), use the first available client in the workspace
      const myClient = user.client_id
        ? dbClients.find((c) => c.id === user.client_id)
        : dbClients[0]

      if (myClient && myClient.teamId) {
        handleSelectTeam(myClient.teamId)
      }
    }
  }, [user, dbClients, currentLevel, handleSelectTeam])

  return (
    <div className="relative flex h-full flex-col bg-background">
      {/* Views */}
      <div className="flex h-full w-full flex-1 flex-col">
        {currentLevel === 'workspaces' && (
          <WorkspacesView
            workspaces={dbWorkspaces}
            userRole={userRole}
            onSelectWorkspace={handleSelectWorkspace}
            onOpenHub={() => setIsHubOpen(true)}
            setDbWorkspaces={setDbWorkspaces}
          />
        )}
        {currentLevel === 'teams' && (
          <TeamsView
            teams={
              userRole === 'platform_admin'
                ? dbTeams.filter((t) => t.workspaceId === selectedWorkspace)
                : dbTeams
            }
            userRole={userRole}
            onSelectTeam={handleSelectTeam}
            onOpenRoleManager={() => setIsRoleManagerOpen(true)}
            onOpenTeamManager={() => setIsTeamManagerOpen(true)}
            onOpenClientManager={() => setIsClientManagerOpen(true)}
            onOpenAdminInvite={() => setIsAdminInviteOpen(true)}
            onPrefetchTeam={prefetchTeamMembers}
          />
        )}
        {currentLevel === 'members' && (
          <MembersView
            members={dbMembers.filter((p) => p.teamId === selectedTeam)}
            userRole={userRole}
            selectedTeam={selectedTeam}
            dbWorkspaceRoles={dbWorkspaceRoles}
            onSelectMember={setSelectedEntity}
            onOpenInviteManager={() => setIsInviteManagerOpen(true)}
            onOpenClientManager={() => {
              setClientToEdit(null)
              setIsClientManagerOpen(true)
            }}
            onEditClient={(c) => {
              setClientToEdit(c)
              setIsClientManagerOpen(true)
            }}
            client={dbClients.find((c) => c.teamId === selectedTeam)}
          />
        )}
      </div>

      {/* Sheets & Modals */}
      <MemberDetailSheet
        member={currentLevel === 'members' ? (selectedEntity as MemberItem | null) : null}
        onClose={() => setSelectedEntity(null)}
        userRole={userRole}
        onEdit={(m) => {
          setMemberToEdit(m)
          setIsEditMemberOpen(true)
        }}
      />

      <DevHubModal isOpen={isHubOpen} onClose={() => setIsHubOpen(false)} />

      <RoleManagerModal
        isOpen={isRoleManagerOpen}
        onClose={() => setIsRoleManagerOpen(false)}
        workspaceRoles={dbWorkspaceRoles}
        workspaceId={workspaceId}
        selectedWorkspace={selectedWorkspace}
      />

      <TeamManagerModal
        isOpen={isTeamManagerOpen}
        onClose={() => setIsTeamManagerOpen(false)}
        workspaceId={workspaceId}
        selectedWorkspace={selectedWorkspace}
      />

      <InviteManagerModal
        isOpen={isInviteManagerOpen}
        onClose={() => setIsInviteManagerOpen(false)}
        selectedTeam={selectedTeam}
        selectedWorkspace={selectedWorkspace}
        workspaceId={workspaceId}
      />

      <ClientManagerModal
        isOpen={isClientManagerOpen}
        onClose={() => {
          setIsClientManagerOpen(false)
          setClientToEdit(null)
        }}
        workspaceId={workspaceId}
        teams={dbTeams}
        initialData={clientToEdit}
      />

      <AdminInviteModal
        isOpen={isAdminInviteOpen}
        onClose={() => setIsAdminInviteOpen(false)}
        workspaceId={workspaceId}
        selectedWorkspace={selectedWorkspace}
      />

      <EditMemberModal
        isOpen={isEditMemberOpen}
        onClose={() => {
          setIsEditMemberOpen(false)
          setMemberToEdit(null)
        }}
        member={memberToEdit}
        onSave={() => {
          loadDirectory()
        }}
      />
    </div>
  )
}
