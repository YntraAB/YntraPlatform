import { supabase } from '@/lib/supabase'
import type { UserPreferences } from '@/types'

export const userService = {
  /**
   * Fetch all users for a specific workspace
   */
  async getWorkspaceUsers(workspaceId: string) {
    const { data, error } = await supabase.from('users').select('*').eq('workspace_id', workspaceId)

    if (error) throw new Error(error.message)
    return data || []
  },

  /**
   * Fetch all admins for a specific workspace
   */
  async getWorkspaceAdmins(workspaceId: string) {
    const { data, error } = await supabase
      .from('users')
      .select('id')
      .eq('workspace_id', workspaceId)
      .in('role', ['admin', 'platform_admin'])

    if (error) throw new Error(error.message)
    return data || []
  },

  /**
   * Fetch details for a specific user
   */
  async getUserDetails(userId: string) {
    const { data, error } = await supabase.from('users').select('*').eq('id', userId).single()

    if (error) throw new Error(error.message)
    return data
  },

  /**
   * Get workspace ID for a specific user
   */
  async getUserWorkspaceId(userId: string) {
    const { data, error } = await supabase
      .from('users')
      .select('workspace_id')
      .eq('id', userId)
      .single()

    if (error) return null
    return data?.workspace_id || null
  },

  /**
   * Get user preferences
   */
  async getUserPreferences(userId: string): Promise<UserPreferences | null> {
    const { data, error } = await supabase
      .from('users')
      .select('preferences')
      .eq('id', userId)
      .single()

    if (error) throw new Error(error.message)
    return (data?.preferences as unknown as UserPreferences) || null
  },

  /**
   * Update user preferences
   */
  async updateUserPreferences(userId: string, preferences: Partial<UserPreferences>) {
    const { error } = await supabase.from('users').update({ preferences }).eq('id', userId)

    if (error) throw new Error(error.message)
  },
}
