import React, { useState, useMemo, useCallback } from 'react'
import {
  User,
  Users,
  HeartPulse,
  Trash2,
  Plus,
  UserPlus,
  Search,
  Inbox,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useNavigate } from 'react-router-dom'
import { supabase } from '@/lib/supabase'
import type { MemberItem, WorkspaceRole, ClientItem } from '../hooks/useDirectoryData'

interface MembersViewProps {
  members: MemberItem[]
  userRole: string
  selectedTeam: string | null
  dbWorkspaceRoles: WorkspaceRole[]
  onSelectMember: (member: MemberItem) => void
  onOpenInviteManager: () => void
  onOpenClientManager: () => void
  onEditClient: (client: ClientItem) => void
  client?: ClientItem | null
}

export const MembersView: React.FC<MembersViewProps> = ({
  members,
  userRole,
  selectedTeam,
  dbWorkspaceRoles,
  onSelectMember,
  onOpenInviteManager,
  onOpenClientManager,
  onEditClient,
  client,
}) => {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [searchQuery, setSearchQuery] = useState('')

  const getRoleName = useCallback((role: string) => {
    if (role === 'platform_admin') return t('directory.roles.platform_admin')
    if (role === 'admin') return t('directory.roles.admin')
    if (role === 'assistant') return t('directory.roles.assistant')
    if (role === 'user') return t('directory.roles.user')
    return role ? role.charAt(0).toUpperCase() + role.slice(1) : t('directory.roles.unknown')
  }, [t])

  const filteredMembers = useMemo(() => {
    if (!searchQuery.trim()) return members
    const q = searchQuery.toLowerCase()
    return members.filter(
      (m) =>
        (m.name && m.name.toLowerCase().includes(q)) ||
        (m.email && m.email.toLowerCase().includes(q)),
    )
  }, [members, searchQuery])

  const groupedMembers = useMemo(() => {
    return filteredMembers.reduce(
      (acc, member) => {
        const roleGroup = getRoleName(member.role)
        if (!acc[roleGroup]) acc[roleGroup] = []
        acc[roleGroup].push(member)
        return acc
      },
      {} as Record<string, MemberItem[]>,
    )
  }, [filteredMembers, getRoleName])

  const sortedRoles = useMemo(() => {
    return Object.keys(groupedMembers).sort((a, b) => {
      if (a === t('directory.roles.platform_admin')) return -1
      if (a === t('directory.roles.admin') && b !== t('directory.roles.platform_admin')) return -1
      return a.localeCompare(b)
    })
  }, [groupedMembers, t])

  const calculateAge = useCallback((personalNumber: string) => {
    if (!personalNumber) return null
    const cleanPn = personalNumber.replace(/\D/g, '')
    if (cleanPn.length < 4) return null
    if (cleanPn.length > 4 && cleanPn.length < 10) return null

    const yearStr = cleanPn.length === 12 ? cleanPn.substring(0, 4) : cleanPn.substring(0, 2)
    const monthStr = cleanPn.length === 12 ? cleanPn.substring(4, 6) : cleanPn.substring(2, 4)
    const dayStr = cleanPn.length === 12 ? cleanPn.substring(6, 8) : cleanPn.substring(4, 6)

    let year = parseInt(yearStr)
    if (cleanPn.length === 10) {
      const now = new Date().getFullYear() % 100
      year += year <= now ? 2000 : 1900
    } else if (cleanPn.length === 4) {
      year = parseInt(cleanPn)
    }

    const birthDate =
      cleanPn.length === 4
        ? new Date(year, 0, 1)
        : new Date(year, parseInt(monthStr) - 1, parseInt(dayStr))
    const today = new Date()
    let age = today.getFullYear() - birthDate.getFullYear()
    if (cleanPn.length !== 4) {
      const m = today.getMonth() - birthDate.getMonth()
      if (m < 0 || (m === 0 && today.getDate() < birthDate.getDate())) {
        age--
      }
    }
    return age
  }, [])

  const isAdmin = userRole === 'platform_admin' || userRole === 'admin'

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header Controls - exactly matching Inbox */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <h2 className="flex items-center gap-1.5 text-xs font-medium text-foreground">
            {selectedTeam === 'all_members' ? (
              <Users className="h-3.5 w-3.5 text-muted-foreground" />
            ) : (
              <User className="h-3.5 w-3.5 text-muted-foreground" />
            )}
            {selectedTeam === 'all_members'
              ? t('directory.members.org_title')
              : t('directory.members.team_title')}
          </h2>
          <div className="rounded border border-border/50 bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
            {filteredMembers.length} {t('directory.workspaces.users_count').toLowerCase()}
          </div>
        </div>

        <div className="flex items-center gap-4">
          <div className="group relative w-64">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
            <Input
              placeholder={t('common.search')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 rounded-md border border-border/50 bg-muted/50 pl-8 text-xs text-foreground transition-all focus-visible:bg-muted focus-visible:ring-1 focus-visible:ring-primary"
            />
          </div>

          {selectedTeam !== 'all_members' && isAdmin && (
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                className="h-8 border-border/60 bg-transparent text-xs text-foreground hover:bg-secondary"
                onClick={onOpenClientManager}
              >
                <UserPlus className="mr-1.5 h-3.5 w-3.5 text-primary" />
                {t('directory.teams.create_client_button')}
              </Button>
              <Button
                size="sm"
                className="h-8 text-xs shadow-sm"
                onClick={onOpenInviteManager}
              >
                <Plus className="mr-1.5 h-3.5 w-3.5" />
                {t('directory.members.invite_button')}
              </Button>
            </div>
          )}
        </div>
      </div>

      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {/* Sleek Minimalist Client Subheader Bar */}
        {client && selectedTeam !== 'all_members' && userRole !== 'client' && !searchQuery.trim() && (
          <div className="flex h-11 shrink-0 items-center justify-between border-b border-border/40 bg-muted/20 px-6 text-xs">
            <div className="flex items-center gap-2 truncate pr-4">
              <HeartPulse className="h-3.5 w-3.5 shrink-0 text-primary" />
              <span className="font-medium text-foreground">
                {client.firstName} {client.lastName}
              </span>
              <span className="text-muted-foreground/40">·</span>
              <span className="text-muted-foreground">{client.careLevel}</span>
              {(client.location || client.address) && (
                <>
                  <span className="text-muted-foreground/40">·</span>
                  <span className="truncate text-muted-foreground/70">
                    {client.location || client.address}
                  </span>
                </>
              )}
              {calculateAge(client.personalNumber) !== null && (
                <>
                  <span className="text-muted-foreground/40">·</span>
                  <span className="text-muted-foreground/70">
                    {calculateAge(client.personalNumber)} {t('common.years_old')}
                  </span>
                </>
              )}
            </div>

            <div className="flex shrink-0 items-center gap-3">
              <button
                type="button"
                onClick={() => navigate(`/notes?team=${client.teamId}`)}
                className="text-xs font-medium text-muted-foreground transition-colors hover:text-foreground"
              >
                {t('sidebar.notes')}
              </button>
              <span className="text-border/60">|</span>
              <button
                type="button"
                onClick={() => navigate(`/medication?team=${client.teamId}`)}
                className="text-xs font-medium text-muted-foreground transition-colors hover:text-foreground"
              >
                {t('sidebar.medication')}
              </button>
              {isAdmin && (
                <>
                  <span className="text-border/60">|</span>
                  <button
                    type="button"
                    onClick={() => onEditClient(client)}
                    className="text-xs font-medium text-primary transition-colors hover:underline"
                  >
                    {t('common.edit')}
                  </button>
                </>
              )}
            </div>
          </div>
        )}

        {filteredMembers.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
              <Inbox className="h-10 w-10 opacity-20" />
            </div>
            <p className="text-sm font-medium">{t('notes.list.empty_state')}</p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {sortedRoles.map((roleGroup) => (
              <div key={roleGroup} className="flex flex-col">
                <div className="sticky top-16 z-10 flex h-7 items-center gap-1.5 border-b border-border/40 bg-background/95 px-6 text-[10px] font-medium uppercase tracking-wider text-muted-foreground/60 backdrop-blur-md">
                  <Users className="h-3 w-3 opacity-70" />
                  <span>{roleGroup} ({groupedMembers[roleGroup].length})</span>
                </div>
                {groupedMembers[roleGroup].map((member, idx) => {
                  const displayName =
                    member.name === member.email
                      ? t('directory.members.name_unspecified')
                      : member.name
                  const displayInitial = (
                    displayName !== t('directory.members.name_unspecified')
                      ? displayName.charAt(0)
                      : member.email.charAt(0)
                  ).toUpperCase()

                  return (
                    <div
                      key={member.id}
                      onClick={() => onSelectMember(member)}
                      style={{ animationDelay: `${idx * 30}ms` }}
                      className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                      tabIndex={0}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') onSelectMember(member)
                      }}
                    >
                      {/* Compact Size-6 User Avatar - logically placed, preserving exact 48px height */}
                      <div className="mr-3 flex size-6 shrink-0 items-center justify-center">
                        <Avatar className="size-6 border border-border/60">
                          <AvatarImage src={member.avatar} />
                          <AvatarFallback className="bg-secondary/80 text-[10px] font-medium text-foreground/80">
                            {displayInitial}
                          </AvatarFallback>
                        </Avatar>
                      </div>

                      {/* Name Column - identical to Inbox */}
                      <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors">
                        {displayName}
                      </div>

                      {/* Email / Details snippet - identical to Inbox */}
                      <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
                        <span className="truncate text-sm font-light italic text-muted-foreground/60">
                          {member.email}
                        </span>
                      </div>

                      {/* Hover Actions & Role Selector/Badge - identical to Inbox */}
                      <div className="flex w-48 shrink-0 items-center justify-end">
                        <div className="mr-6 flex translate-x-2 items-center gap-3.5 text-muted-foreground/60 opacity-0 transition-all duration-300 group-hover:translate-x-0 group-hover:opacity-100">
                          {isAdmin && (
                            <button
                              type="button"
                              onClick={async (e) => {
                                e.stopPropagation()
                                if (selectedTeam !== 'all_members') {
                                  if (
                                    confirm(
                                      t('directory.members.delete_team_confirm', {
                                        name: member.name,
                                      }),
                                    )
                                  ) {
                                    const { error } = await supabase
                                      .from('team_members')
                                      .delete()
                                      .eq('user_id', member.id)
                                      .eq('team_id', selectedTeam)
                                    if (error)
                                      alert(t('directory.members.delete_error') + ' ' + error.message)
                                  }
                                } else {
                                  if (
                                    confirm(
                                      t('directory.members.delete_org_confirm', {
                                        name: member.name,
                                      }),
                                    )
                                  ) {
                                    const { error } = await supabase
                                      .from('users')
                                      .update({ workspace_id: null })
                                      .eq('id', member.id)
                                    if (error)
                                      alert(t('directory.members.delete_error') + ' ' + error.message)
                                    else {
                                      await supabase
                                        .from('team_members')
                                        .delete()
                                        .eq('user_id', member.id)
                                    }
                                  }
                                }
                              }}
                              className="p-0.5 text-muted-foreground/60 transition-colors hover:text-rose-500"
                              title={
                                selectedTeam !== 'all_members'
                                  ? t('directory.members.delete_team_tooltip')
                                  : t('directory.members.delete_org_tooltip')
                              }
                            >
                              <Trash2 className="h-[18px] w-[18px]" />
                            </button>
                          )}
                        </div>

                        {selectedTeam !== 'all_members' && isAdmin ? (
                          <div className="w-28" onClick={(e) => e.stopPropagation()}>
                            <Select
                              value={member.roleId || 'none'}
                              onValueChange={async (value) => {
                                const newRoleId = value === 'none' ? null : value
                                const { error } = await supabase
                                  .from('team_members')
                                  .update({ role_id: newRoleId })
                                  .eq('user_id', member.id)
                                  .eq('team_id', selectedTeam)
                                if (error)
                                  alert(
                                    t('directory.members.update_role_error') + ' ' + error.message,
                                  )
                              }}
                            >
                              <SelectTrigger className="h-7 border-border/40 bg-muted/40 text-[11px] font-medium text-foreground">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="none">
                                  {t('directory.members.default_assistant_role')}
                                </SelectItem>
                                {dbWorkspaceRoles.map((r) => (
                                  <SelectItem key={r.id} value={r.id}>
                                    {r.name}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          </div>
                        ) : (
                          <span className="flex items-center gap-1 text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60 transition-colors">
                            <User className="h-3 w-3 opacity-70" />
                            <span>{roleGroup}</span>
                          </span>
                        )}
                      </div>
                    </div>
                  )
                })}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}



