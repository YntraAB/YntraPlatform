import React from 'react'
import { Users, FileText, Search, Inbox } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { useTranslation } from 'react-i18next'
import type { UnreadNotesMap } from '@/hooks/useUnreadNotes'
import type { NoteTeam } from '../types'

interface TeamOverviewProps {
  teams: NoteTeam[]
  searchQuery: string
  setSearchQuery: (query: string) => void
  unreadNotes: UnreadNotesMap
  handleSelectTeam: (teamId: string) => void
}

export const TeamOverview: React.FC<TeamOverviewProps> = ({
  teams,
  searchQuery,
  setSearchQuery,
  unreadNotes,
  handleSelectTeam,
}) => {
  const { t } = useTranslation()

  let filteredTeams = teams
  if (searchQuery) {
    filteredTeams = filteredTeams.filter((team) =>
      (team.displayName || team.name).toLowerCase().includes(searchQuery.toLowerCase()),
    )
  }

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header Controls - exactly matching Inbox */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <h2 className="flex items-center gap-1.5 text-xs font-medium text-foreground">
            <Users className="h-3.5 w-3.5 text-muted-foreground" />
            {t('notes.teams.title')}
          </h2>
          <div className="rounded border border-border/50 bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
            {filteredTeams.length} {t('directory.teams.title').toLowerCase()}
          </div>
        </div>

        <div className="flex items-center gap-4">
          <div className="group relative w-64">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
            <Input
              placeholder={t('notes.teams.search_placeholder')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="h-8 rounded-md border border-border/50 bg-muted/50 pl-8 text-xs text-foreground transition-all focus-visible:bg-muted focus-visible:ring-1 focus-visible:ring-primary"
            />
          </div>
        </div>
      </div>

      {/* Team Rows List - exactly matching Inbox row height & layout */}
      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {filteredTeams.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
              <Inbox className="h-10 w-10 opacity-20" />
            </div>
            <p className="text-sm font-medium">{t('notes.teams.empty_state')}</p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredTeams.map((team, idx) => {
              const unreadInTeam = unreadNotes?.byTeam[team.id] || 0

              return (
                <div
                  key={team.id}
                  onClick={() => handleSelectTeam(team.id)}
                  style={{ animationDelay: `${idx * 30}ms` }}
                  className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleSelectTeam(team.id)
                  }}
                >
                  {/* Leading Users Icon */}
                  <div className="mr-3 flex size-6 shrink-0 items-center justify-center text-muted-foreground/70 transition-colors group-hover:text-primary">
                    <Users className="h-4 w-4" />
                  </div>

                  {/* Team Name Column - identical to Inbox */}
                  <div className="w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 text-sm font-medium text-foreground transition-colors flex items-center gap-2">
                    <span className="truncate">{team.displayName || team.name}</span>
                    {unreadInTeam > 0 && (
                      <span className="h-1.5 w-1.5 rounded-full bg-primary shrink-0" />
                    )}
                  </div>

                  {/* Notes Count & Snippet - identical to Inbox */}
                  <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
                    <span className="flex items-center gap-1.5 truncate text-sm font-normal text-foreground">
                      <FileText className="inline h-3.5 w-3.5 text-muted-foreground/60" />
                      <span>{team.notesCount || 0} {t('notes.teams.notes_count')}</span>
                    </span>
                    {team.recentNote && (
                      <span className="truncate text-sm font-light italic text-muted-foreground/60">
                        — {t('notes.teams.last_updated')}: {new Date(team.recentNote).toLocaleDateString()}
                      </span>
                    )}
                  </div>

                  {/* Right Tabular Date - identical to Inbox */}
                  <div className="flex w-48 shrink-0 items-center justify-end">
                    <span className="text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60 transition-colors">
                      {team.recentNote ? new Date(team.recentNote).toLocaleDateString() : ''}
                    </span>
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}


