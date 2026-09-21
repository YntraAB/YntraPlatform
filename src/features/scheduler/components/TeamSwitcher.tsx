import React from 'react'
import { useTranslation } from 'react-i18next'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useWorkspaceTeams } from '@/hooks/queries/useWorkspaceData'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Users, FilterX } from 'lucide-react'

export const TeamSwitcher: React.FC = () => {
  const { t } = useTranslation()
  const { workspaceId, selectedTeamId, setSelectedTeamId } = useWorkspace()
  const { data: teams = [], isLoading } = useWorkspaceTeams(workspaceId)

  if (isLoading) {
    return <div className="h-8 w-full animate-pulse rounded-md bg-secondary/20" />
  }

  return (
    <div className="space-y-1.5">
      <div className="flex items-center gap-1.5 px-0.5">
        <Users className="h-3 w-3 text-muted-foreground/60" />
        <span className="text-[11px] font-normal tracking-wide text-muted-foreground/70">
          {t('common.active_team') || 'Aktivt Team'}
        </span>
      </div>
      <Select
        value={selectedTeamId || 'all'}
        onValueChange={(val) => setSelectedTeamId(val === 'all' ? null : val)}
      >
        <SelectTrigger className="h-8 w-full border-border/40 bg-secondary/20 text-xs font-normal text-foreground transition-colors hover:bg-secondary/40 focus:ring-0">
          <SelectValue placeholder={t('common.all_teams') || 'Alla team'} />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">
            <div className="flex items-center gap-2 text-xs">
              <FilterX className="h-3.5 w-3.5 text-muted-foreground" />
              <span>{t('common.all_teams') || 'Alla team'}</span>
            </div>
          </SelectItem>
          {teams.map((team) => (
            <SelectItem key={team.id} value={team.id}>
              <div className="flex w-full items-center justify-between gap-2 text-xs">
                <span>{team.name}</span>
                {team.is_active === false && (
                  <span className="text-[9px] font-normal text-muted-foreground/60">
                    Inaktiv
                  </span>
                )}
              </div>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
