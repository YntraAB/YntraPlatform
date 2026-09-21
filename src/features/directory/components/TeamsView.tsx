import React, { useState } from 'react'
import { Users, User, HeartPulse, Trash2, Plus, ShieldCheck, Search, Inbox } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { supabase } from '@/lib/supabase'
import type { TeamItem } from '../hooks/useDirectoryData'

interface TeamsViewProps {
  teams: TeamItem[]
  userRole: string
  onSelectTeam: (id: string) => void
  onOpenRoleManager: () => void
  onOpenTeamManager: () => void
  onOpenClientManager: () => void
  onOpenAdminInvite: () => void
  onPrefetchTeam: (id: string) => void
}

export const TeamsView: React.FC<TeamsViewProps> = ({
  teams,
  userRole,
  onSelectTeam,
  onOpenRoleManager,
  onOpenTeamManager,
  onOpenAdminInvite,
  onPrefetchTeam,
}) => {
  const { t } = useTranslation()
  const [searchQuery, setSearchQuery] = useState('')

  let filteredTeams = teams
  if (searchQuery.trim()) {
    const q = searchQuery.toLowerCase()
    filteredTeams = filteredTeams.filter(
      (team) =>
        team.name.toLowerCase().includes(q) ||
        (team.leader && team.leader.toLowerCase().includes(q)),
    )
  }

  const isAdmin = userRole === 'admin' || userRole === 'platform_admin'

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header Controls - exactly matching Inbox */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <h2 className="flex items-center gap-1.5 text-xs font-medium text-foreground">
            <Users className="h-3.5 w-3.5 text-muted-foreground" />
            {t('directory.teams.title')}
          </h2>
          <div className="rounded border border-border/50 bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
            {filteredTeams.length} {t('directory.teams.title').toLowerCase()}
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

          {isAdmin && (
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                className="h-8 border-border/60 bg-transparent text-xs text-foreground hover:bg-secondary"
                onClick={onOpenRoleManager}
              >
                {t('directory.teams.manage_roles')}
              </Button>
              <Button
                size="sm"
                variant="outline"
                className="h-8 border-border/60 bg-transparent text-xs text-foreground hover:bg-secondary"
                onClick={onOpenAdminInvite}
              >
                <ShieldCheck className="mr-1.5 h-3.5 w-3.5 text-primary" />
                {t('directory.teams.add_admin_button')}
              </Button>
              <Button
                size="sm"
                className="h-8 text-xs shadow-sm"
                onClick={onOpenTeamManager}
              >
                <Plus className="mr-1.5 h-3.5 w-3.5" />
                {t('directory.teams.create_button')}
              </Button>
            </div>
          )}
        </div>
      </div>

      {/* Teams List - exactly matching Inbox row height & layout */}
      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {/* All Members Row (Admin only) */}
        {isAdmin && !searchQuery.trim() && (
          <div
            onClick={() => onSelectTeam('all_members')}
            onMouseEnter={() => onPrefetchTeam('all_members')}
            className="group flex h-12 cursor-pointer items-center border-b border-border/50 bg-secondary/30 dark:bg-muted/30 px-6 transition-all duration-200 hover:bg-secondary/50 dark:hover:bg-muted/45 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === 'Enter') onSelectTeam('all_members')
            }}
          >
            {/* Leading Users Icon */}
            <div className="mr-3 flex size-6 shrink-0 items-center justify-center rounded bg-secondary/70 dark:bg-secondary/50 text-foreground/80 transition-colors group-hover:text-primary">
              <Users className="h-4 w-4" />
            </div>

            {/* Title Column - matches team rows below */}
            <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors">
              {t('directory.levels.all_members')}
            </div>

            {/* Snippet Column - matches team rows below */}
            <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
              <span className="truncate text-sm font-normal text-muted-foreground/75">
                {t('directory.teams.all_members_subtitle')}
              </span>
            </div>

            {/* Right Tabular Text */}
            <div className="flex w-48 shrink-0 items-center justify-end">
              <span className="flex items-center gap-1 rounded bg-secondary/60 dark:bg-secondary/50 px-1.5 py-0.5 text-[10px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/70 transition-colors">
                <ShieldCheck className="h-3 w-3 opacity-70" />
                <span>{t('directory.roles.admin')}</span>
              </span>
            </div>
          </div>
        )}

        {filteredTeams.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
              <Inbox className="h-10 w-10 opacity-20" />
            </div>
            <p className="text-sm font-medium">{t('notes.teams.empty_state')}</p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredTeams.map((team, idx) => (
              <div
                key={team.id}
                onClick={() => onSelectTeam(team.id)}
                onMouseEnter={() => onPrefetchTeam(team.id)}
                style={{ animationDelay: `${idx * 30}ms` }}
                className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') onSelectTeam(team.id)
                }}
              >
                {/* Leading Team Icon */}
                <div className="mr-3 flex size-6 shrink-0 items-center justify-center text-muted-foreground/70 transition-colors group-hover:text-primary">
                  <Users className="h-4 w-4" />
                </div>

                {/* Team Name Column - identical to Inbox */}
                <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors">
                  {team.name}
                </div>

                {/* Led by & Details Snippet - identical to Inbox */}
                <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
                  <span className="truncate text-sm font-normal text-foreground">
                    {team.leader ? `${t('directory.teams.led_by')} ${team.leader}` : team.name}
                  </span>
                  <span className="flex items-center gap-1 truncate text-sm font-light italic text-muted-foreground/60">
                    — <User className="inline h-3 w-3 text-muted-foreground/50" /> {team.membersCount} {t('directory.teams.assistants_count')}
                    <HeartPulse className="ml-1 inline h-3 w-3 text-muted-foreground/50" /> {team.patientsCount} {t('directory.teams.patients_count')}
                  </span>
                </div>

                {/* Hover Actions & Assistants Count - identical to Inbox */}
                <div className="flex w-48 shrink-0 items-center justify-end">
                  <div className="mr-6 flex translate-x-2 items-center gap-3.5 text-muted-foreground/60 opacity-0 transition-all duration-300 group-hover:translate-x-0 group-hover:opacity-100">
                    {isAdmin && (
                      <button
                        type="button"
                        onClick={async (e) => {
                          e.stopPropagation()
                          if (confirm(t('directory.teams.delete_confirm', { name: team.name }))) {
                            const { error } = await supabase
                              .from('teams')
                              .delete()
                              .eq('id', team.id)
                            if (error)
                              alert(t('directory.members.delete_error') + ' ' + error.message)
                          }
                        }}
                        className="p-0.5 text-muted-foreground/60 transition-colors hover:text-rose-500"
                        title={t('directory.teams.delete_tooltip')}
                      >
                        <Trash2 className="h-[18px] w-[18px]" />
                      </button>
                    )}
                  </div>
                  <span className="flex items-center gap-1 text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60 transition-colors">
                    <User className="h-3 w-3 opacity-70" />
                    <span>{team.membersCount} {t('directory.teams.assistants_count')}</span>
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}



