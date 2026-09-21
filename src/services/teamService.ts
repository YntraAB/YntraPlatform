import { supabase } from '@/lib/supabase'

export const teamService = {
  /**
   * Fetch all teams for a specific workspace
   */
  async getWorkspaceTeams(workspaceId: string) {
    const { data, error } = await supabase.from('teams').select('*').eq('workspace_id', workspaceId)

    if (error) throw new Error(error.message)
    return data || []
  },

  /**
   * Fetch details for a specific team
   */
  async getTeamDetails(teamId: string) {
    const { data, error } = await supabase.from('teams').select('*').eq('id', teamId).single()

    if (error) throw new Error(error.message)
    return data
  },
}
