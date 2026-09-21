import React, { useState } from 'react'
import { Settings, Shield, Plus, Trash2, Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { supabase } from '@/lib/supabase'
import type { WorkspaceRole } from '../hooks/useDirectoryData'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'

interface RoleManagerModalProps {
  isOpen: boolean
  onClose: () => void
  workspaceRoles: WorkspaceRole[]
  workspaceId: string | null
  selectedWorkspace: string | null
}

export const RoleManagerModal: React.FC<RoleManagerModalProps> = ({
  isOpen,
  onClose,
  workspaceRoles,
  workspaceId,
  selectedWorkspace,
}) => {
  const { t } = useTranslation()
  const [editingRole, setEditingRole] = useState<WorkspaceRole | 'new' | null>(null) // null = list view, 'new' = creating, or role_object = editing
  const [roleForm, setRoleForm] = useState({
    name: '',
    can_manage_schedule: false,
    can_manage_notes: false,
    can_approve_time_reports: false,
  })

  if (!isOpen) return null

  const handleSaveRole = async () => {
    const targetWS = selectedWorkspace || workspaceId
    const payload = {
      workspace_id: targetWS,
      name: roleForm.name,
      permissions: {
        can_manage_schedule: roleForm.can_manage_schedule,
        can_manage_notes: roleForm.can_manage_notes,
        can_approve_time_reports: roleForm.can_approve_time_reports,
      },
    }

    let error
    if (editingRole === 'new') {
      const res = await supabase.from('workspace_roles').insert([payload])
      error = res.error
    } else {
      if (!editingRole) return
      const res = await supabase.from('workspace_roles').update(payload).eq('id', editingRole.id)
      error = res.error
    }

    if (error) alert(t('directory.members.delete_error') + ' ' + error.message)
    else {
      alert(
        editingRole === 'new'
          ? t('directory.role_manager.create_success')
          : t('directory.role_manager.update_success'),
      )
      setEditingRole(null)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="gap-0 overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-[800px]">
        <div className="flex h-[560px]">
          {/* Left side: List of Roles */}
          <div className="flex w-[240px] flex-col border-r border-border bg-secondary/30">
            <div className="flex h-12 items-center gap-2 border-b border-border bg-secondary/40 px-4">
              <Shield className="size-3.5 text-muted-foreground/70 shrink-0" />
              <h3 className="text-xs font-medium text-foreground">
                {t('directory.role_manager.title')}
              </h3>
            </div>
            <div className="scrollbar-dark flex-1 overflow-y-auto p-2.5">
              {/* Default Assistant Role (Read Only) */}
              <div
                className={`mb-1.5 cursor-pointer rounded-md p-2.5 transition-colors ${editingRole === null ? 'border border-primary bg-secondary/60 font-medium' : 'border border-transparent hover:bg-secondary/40'}`}
                onClick={() => {
                  setEditingRole(null)
                  setRoleForm({
                    name: '',
                    can_manage_schedule: false,
                    can_manage_notes: false,
                    can_approve_time_reports: false,
                  })
                }}
              >
                <div className="text-xs font-medium text-foreground">
                  {t('directory.members.default_assistant_role')}
                </div>
                <div className="mt-0.5 text-[10px] leading-relaxed text-muted-foreground">
                  {t('directory.role_manager.locked_role_desc')}
                </div>
              </div>

              {workspaceRoles.map((role) => (
                <div
                  key={role.id}
                  className={`mb-1.5 cursor-pointer rounded-md p-2.5 transition-colors ${editingRole !== 'new' && editingRole?.id === role.id ? 'border border-primary bg-secondary/60 font-medium' : 'border border-transparent hover:bg-secondary/40'}`}
                  onClick={() => {
                    setEditingRole(role)
                    setRoleForm({
                      name: role.name,
                      can_manage_schedule: role.permissions?.can_manage_schedule || false,
                      can_manage_notes: role.permissions?.can_manage_notes || false,
                      can_approve_time_reports:
                        role.permissions?.can_approve_time_reports || false,
                    })
                  }}
                >
                  <div className="text-xs font-medium text-foreground">{role.name}</div>
                  <div className="mt-0.5 text-[10px] leading-relaxed text-muted-foreground">
                    {t('directory.role_manager.custom_role_desc')}
                  </div>
                </div>
              ))}
            </div>
            <div className="border-t border-border p-3 bg-secondary/40">
              <Button
                variant="outline"
                size="sm"
                className="h-8 w-full rounded-md border border-border bg-background text-xs font-medium text-foreground hover:bg-secondary"
                onClick={() => {
                  setEditingRole('new')
                  setRoleForm({
                    name: '',
                    can_manage_schedule: false,
                    can_manage_notes: false,
                    can_approve_time_reports: false,
                  })
                }}
              >
                <Plus className="mr-1.5 size-3" />
                <span>Skapa ny roll</span>
              </Button>
            </div>
          </div>

          {/* Right side: Editor */}
          <div className="flex flex-1 flex-col bg-card">
            <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
              <DialogTitle className="text-xs font-medium text-foreground">
                {editingRole === 'new'
                  ? t('directory.role_manager.create_title')
                  : editingRole
                    ? t('directory.role_manager.edit_title')
                    : 'Information'}
              </DialogTitle>
            </DialogHeader>

            {editingRole === 'new' || (editingRole && editingRole.id) ? (
              <>
                <div className="scrollbar-dark flex-1 space-y-4 overflow-y-auto p-5">
                  <div className="space-y-1">
                    <label className="block text-[11px] font-medium text-muted-foreground">
                      {t('directory.role_manager.role_name_label')} *
                    </label>
                    <input
                      value={roleForm.name}
                      onChange={(e) => setRoleForm({ ...roleForm, name: e.target.value })}
                      className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:border-border focus:ring-1 focus:ring-ring"
                      placeholder={t('directory.role_manager.role_name_placeholder')}
                    />
                  </div>

                  <div className="space-y-2.5">
                    <label className="block text-[11px] font-medium text-muted-foreground">
                      {t('directory.role_manager.permissions_label')}
                    </label>

                    <div className="flex items-start gap-2.5 rounded-md border border-border bg-secondary/20 p-3">
                      <input
                        type="checkbox"
                        id="p_sched"
                        checked={roleForm.can_manage_schedule}
                        onChange={(e) =>
                          setRoleForm({ ...roleForm, can_manage_schedule: e.target.checked })
                        }
                        className="mt-0.5 size-3.5 cursor-pointer rounded border-border"
                      />
                      <label htmlFor="p_sched" className="flex-1 cursor-pointer">
                        <div className="text-xs font-medium text-foreground">
                          {t('directory.role_manager.perm_schedule_title')}
                        </div>
                        <div className="mt-0.5 text-[10px] leading-relaxed text-muted-foreground">
                          {t('directory.role_manager.perm_schedule_desc')}
                        </div>
                      </label>
                    </div>

                    <div className="flex items-start gap-2.5 rounded-md border border-border bg-secondary/20 p-3">
                      <input
                        type="checkbox"
                        id="p_notes"
                        checked={roleForm.can_manage_notes}
                        onChange={(e) =>
                          setRoleForm({ ...roleForm, can_manage_notes: e.target.checked })
                        }
                        className="mt-0.5 size-3.5 cursor-pointer rounded border-border"
                      />
                      <label htmlFor="p_notes" className="flex-1 cursor-pointer">
                        <div className="text-xs font-medium text-foreground">
                          {t('directory.role_manager.perm_notes_title')}
                        </div>
                        <div className="mt-0.5 text-[10px] leading-relaxed text-muted-foreground">
                          {t('directory.role_manager.perm_notes_desc')}
                        </div>
                      </label>
                    </div>

                    <div className="flex items-start gap-2.5 rounded-md border border-border bg-secondary/20 p-3">
                      <input
                        type="checkbox"
                        id="p_time"
                        checked={roleForm.can_approve_time_reports}
                        onChange={(e) =>
                          setRoleForm({ ...roleForm, can_approve_time_reports: e.target.checked })
                        }
                        className="mt-0.5 size-3.5 cursor-pointer rounded border-border"
                      />
                      <label htmlFor="p_time" className="flex-1 cursor-pointer">
                        <div className="text-xs font-medium text-foreground">
                          {t('directory.role_manager.perm_time_title')}
                        </div>
                        <div className="mt-0.5 text-[10px] leading-relaxed text-muted-foreground">
                          {t('directory.role_manager.perm_time_desc')}
                        </div>
                      </label>
                    </div>
                  </div>
                </div>

                <div className="flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={onClose}
                    className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
                  >
                    {t('directory.role_manager.close')}
                  </Button>
                  {editingRole !== 'new' && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={async () => {
                        if (confirm(t('directory.role_manager.delete_confirm'))) {
                          await supabase.from('workspace_roles').delete().eq('id', editingRole.id)
                          setEditingRole(null)
                        }
                      }}
                      className="h-8 rounded-md px-3 text-xs font-medium text-destructive hover:bg-destructive/10"
                    >
                      <Trash2 className="mr-1.5 size-3" />
                      <span>Ta bort</span>
                    </Button>
                  )}
                  <Button
                    size="sm"
                    onClick={handleSaveRole}
                    disabled={!roleForm.name}
                    className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
                  >
                    <Check className="mr-1.5 size-3" />
                    <span>
                      {editingRole === 'new'
                        ? t('directory.role_manager.save_new')
                        : t('directory.role_manager.save_changes')}
                    </span>
                  </Button>
                </div>
              </>
            ) : (
              <div className="flex flex-1 flex-col items-center justify-center p-6 text-center text-muted-foreground">
                <Settings className="mb-3 size-8 text-muted-foreground/30" />
                <p className="text-xs text-muted-foreground">{t('directory.role_manager.empty_state')}</p>
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-4 h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
                  onClick={onClose}
                >
                  {t('directory.role_manager.close_manager')}
                </Button>
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
