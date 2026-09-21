import { supabase } from '@/lib/supabase'
import type { WorkspaceSettings, WorkspaceModules } from '@/types'

export const workspaceService = {
  /**
   * Fetch workspace details
   */
  async getWorkspace(workspaceId: string) {
    const { data, error } = await supabase
      .from('workspaces')
      .select('*')
      .eq('id', workspaceId)
      .single()

    if (error) throw new Error(error.message)
    return data
  },

  /**
   * Update workspace settings
   */
  async updateSettings(workspaceId: string, settings: Partial<WorkspaceSettings>) {
    console.log('workspaceService.updateSettings called for WS:', workspaceId, 'with:', settings)
    const { data, error } = await supabase
      .from('workspaces')
      .update({ settings })
      .eq('id', workspaceId)
      .select()

    if (error) {
      console.error('Supabase error updating settings:', error)
      throw new Error(error.message)
    }
    console.log('Supabase update response:', data)
  },

  /**
   * Update active modules
   */
  async updateModules(workspaceId: string, modules_active: Partial<WorkspaceModules>) {
    const { error } = await supabase
      .from('workspaces')
      .update({ modules_active })
      .eq('id', workspaceId)

    if (error) throw new Error(error.message)
  },

  /**
   * Update block settings
   */
  async updateBlockSettings(workspaceId: string, block_settings: Record<string, any>) {
    const { error } = await supabase
      .from('workspaces')
      .update({ block_settings })
      .eq('id', workspaceId)

    if (error) throw new Error(error.message)
  },

  /**
   * Update workspace core info (name, logo_url)
   */
  async updateWorkspace(
    workspaceId: string,
    updates: { name?: string; logo_url?: string | null; brand_color?: string },
  ) {
    const { error } = await supabase.from('workspaces').update(updates).eq('id', workspaceId)

    if (error) throw new Error(error.message)
  },

  /**
   * Get the first available workspace (for platform admins)
   */
  async getFirstWorkspace() {
    const { data, error } = await supabase.from('workspaces').select('id').limit(1).single()

    if (error) return null
    return data?.id || null
  },
}
