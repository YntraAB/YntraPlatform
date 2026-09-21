import React, { useState, useEffect } from 'react'
import { User } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { supabase } from '@/lib/supabase'

interface InviteManagerModalProps {
  isOpen: boolean
  onClose: () => void
  selectedTeam: string | null
  selectedWorkspace: string | null
  workspaceId: string | null
}

interface WorkspaceUser {
  id: string
  full_name: string | null
  email: string
}

export const InviteManagerModal: React.FC<InviteManagerModalProps> = ({
  isOpen,
  onClose,
  selectedTeam,
  selectedWorkspace,
  workspaceId,
}) => {
  const { t } = useTranslation()
  const [inviteTab, setInviteTab] = useState<'existing' | 'new'>('existing')
  const [inviteEmail, setInviteEmail] = useState('')
  const [selectedExistingUserId, setSelectedExistingUserId] = useState('')
  const [workspaceUsers, setWorkspaceUsers] = useState<WorkspaceUser[]>([])
  const [isLoading, setIsLoading] = useState(false)

  useEffect(() => {
    if (isOpen) {
      const fetchWorkspaceUsers = async () => {
        const targetWS = selectedWorkspace || workspaceId
        const { data: usersData } = await supabase
          .from('users')
          .select('*')
          .eq('workspace_id', targetWS)
        const { data: teamMembersData } = await supabase
          .from('team_members')
          .select('user_id')
          .eq('team_id', selectedTeam)

        const existingMemberIds = (teamMembersData || []).map((tm) => tm.user_id)
        const availableUsers = (usersData || []).filter((u) => !existingMemberIds.includes(u.id))
        setWorkspaceUsers(availableUsers)
      }
      fetchWorkspaceUsers()
    }
  }, [isOpen, selectedWorkspace, workspaceId, selectedTeam])

  if (!isOpen) return null

  const handleInviteExisting = async () => {
    const { error } = await supabase
      .from('team_members')
      .insert([{ team_id: selectedTeam, user_id: selectedExistingUserId }])
    if (error) alert(t('directory.members.delete_error') + ' ' + error.message)
    else {
      alert(t('directory.invite.existing_success'))
      onClose()
      setSelectedExistingUserId('')
    }
  }

  const handleInviteNew = async () => {
    setIsLoading(true)
    const { data, error } = await supabase.functions.invoke('invite_user', {
      body: {
        email: inviteEmail,
        role: 'assistant',
        workspaceId: selectedWorkspace || workspaceId,
        teamId: selectedTeam,
      },
    })
    setIsLoading(false)
    if (error) alert(t('directory.members.delete_error') + ' ' + error.message)
    else if (data && data.success === false)
      alert(t('directory.members.delete_error') + ' ' + data.error)
    else {
      alert(t('directory.invite.new_success'))
      onClose()
      setInviteEmail('')
    }
  }

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="flex w-[450px] flex-col overflow-hidden rounded-xl border border-border bg-sidebar p-0 shadow-sm">
        <div className="border-b border-border p-6 pb-2">
          <h3 className="mb-4 flex items-center gap-2 text-lg font-medium text-foreground">
            <User className="h-5 w-5 text-primary" /> {t('directory.invite.title')}
          </h3>

          <div className="mb-4 flex rounded-lg bg-muted p-1">
            <button
              className={`flex-1 rounded-md py-1.5 text-xs font-medium transition-colors ${inviteTab === 'existing' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}
              onClick={() => setInviteTab('existing')}
            >
              {t('directory.invite.tab_existing')}
            </button>
            <button
              className={`flex-1 rounded-md py-1.5 text-xs font-medium transition-colors ${inviteTab === 'new' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}
              onClick={() => setInviteTab('new')}
            >
              {t('directory.invite.tab_new')}
            </button>
          </div>
        </div>

        <div className="scrollbar-dark max-h-[350px] flex-1 overflow-y-auto p-6">
          {inviteTab === 'existing' ? (
            <div className="space-y-4">
              <p className="text-xs text-muted-foreground">{t('directory.invite.existing_desc')}</p>
              <div className="space-y-2">
                {workspaceUsers.length === 0 ? (
                  <div className="py-4 text-center text-sm text-muted-foreground">
                    {t('directory.invite.no_users')}
                  </div>
                ) : (
                  workspaceUsers.map((u) => (
                    <div
                      key={u.id}
                      onClick={() => setSelectedExistingUserId(u.id)}
                      className={`flex cursor-pointer items-center justify-between rounded-lg border p-3 transition-colors ${selectedExistingUserId === u.id ? 'border-primary/50 bg-primary/10' : 'border-border bg-muted hover:border-border'}`}
                    >
                      <div>
                        <div className="text-sm font-medium text-foreground">
                          {u.full_name || t('directory.invite.anonymous')}
                        </div>
                        <div className="text-xs text-muted-foreground">{u.email}</div>
                      </div>
                      <div
                        className={`flex h-4 w-4 items-center justify-center rounded-full border ${selectedExistingUserId === u.id ? 'border-primary bg-primary' : 'border-muted-foreground/30'}`}
                      >
                        {selectedExistingUserId === u.id && (
                          <div className="h-1.5 w-1.5 rounded-full bg-white" />
                        )}
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              <p className="mb-1 text-xs text-muted-foreground">{t('directory.invite.new_desc')}</p>
              <div>
                <label className="mb-1.5 block text-xs text-muted-foreground">
                  {t('directory.invite.email_label')}
                </label>
                <input
                  type="email"
                  value={inviteEmail}
                  onChange={(e) => setInviteEmail(e.target.value)}
                  className="w-full rounded-md border border-border bg-muted px-3 py-2 text-sm text-foreground focus:border-primary focus:outline-none"
                  placeholder={t('directory.invite.email_placeholder')}
                />
              </div>
            </div>
          )}
        </div>

        <div className="flex justify-end gap-3 border-t border-border bg-background p-5">
          <Button
            variant="ghost"
            onClick={() => {
              onClose()
              setSelectedExistingUserId('')
            }}
            className="h-9 text-xs text-muted-foreground hover:text-foreground"
          >
            {t('directory.hub.cancel')}
          </Button>
          {inviteTab === 'existing' ? (
            <Button
              size="sm"
              onClick={handleInviteExisting}
              disabled={!selectedExistingUserId}
            >
              {t('directory.invite.title')}
            </Button>
          ) : (
            <Button
              size="sm"
              onClick={handleInviteNew}
              disabled={isLoading || !inviteEmail}
            >
              {isLoading ? t('directory.invite.sending') : t('directory.invite.send_button')}
            </Button>
          )}
        </div>
      </div>
    </div>
  )
}
