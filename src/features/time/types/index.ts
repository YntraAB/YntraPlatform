export type DevRole = 'platform_admin' | 'admin' | 'assistant'
export type NavLevel = 'platform_overview' | 'team_overview' | 'assistant_teams' | 'shift_list'

export interface TimeReportUI {
  id: string
  employeeId: string
  employee: string
  role: string
  teamId: string | null
  team: string
  workspaceId: string
  date: string
  start: string
  end: string
  duration: number
  break: number
  status: string
  location: string
  note: string
}
