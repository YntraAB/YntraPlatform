import { supabase } from '@/lib/supabase'
import type { Json } from '@/types/database'

export interface NoteAuthorRecord {
  full_name: string | null
  email: string | null
}

export interface WorkNoteRecord {
  id: string
  team_id: string
  workspace_id: string
  author_id: string | null
  subject: string
  content: string
  created_at: string | null
  edit_history: Json | null
  author?: NoteAuthorRecord | NoteAuthorRecord[] | null
}

export interface NoteTeamRecord {
  id: string
  name: string
  workspace_id: string
  workspaces?: { name: string } | { name: string }[] | null
  notesCount: number
  displayName: string
  recentNote: string | null
}

export const noteService = {
  /**
   * Fetch all notes for a workspace
   */
  async getWorkspaceNotes(workspaceId: string) {
    const { data, error } = await supabase
      .from('notes')
      .select(
        `
        *,
        author:users(full_name, email),
        team:teams(name)
      `,
      )
      .eq('workspace_id', workspaceId)
      .order('created_at', { ascending: false })

    if (error) throw error
    return data || []
  },

  async getNoteTeams(workspaceId?: string) {
    let query = supabase.from('teams').select('id, name, workspace_id, workspaces(name)')

    if (workspaceId) {
      query = query.eq('workspace_id', workspaceId)
    }

    const { data: teams, error: teamsError } = await query.order('name')

    if (teamsError) throw teamsError
    if (!teams || teams.length === 0) return [] as NoteTeamRecord[]

    const teamIds = teams.map((team: any) => team.id)
    const { data: notes, error: notesError } = await supabase
      .from('notes')
      .select('team_id, created_at')
      .in('team_id', teamIds)
      .order('created_at', { ascending: false })

    if (notesError) throw notesError

    return teams
      .map((team: any) => {
        const teamNotes = (notes || []).filter((note: any) => note.team_id === team.id)
        const workspaces = team.workspaces as
          | { name: string }
          | { name: string }[]
          | null
          | undefined
        const orgName = Array.isArray(workspaces) ? workspaces[0]?.name : workspaces?.name

        return {
          ...team,
          notesCount: teamNotes.length,
          displayName: orgName ? `${team.name} - ${orgName}` : team.name,
          recentNote: teamNotes[0]?.created_at || null,
        }
      })
      .sort((a: any, b: any) => a.displayName.localeCompare(b.displayName))
  },

  async getTeamNotes(teamId: string) {
    const { data, error } = await supabase
      .from('notes')
      .select(
        'id, team_id, workspace_id, author_id, subject, content, created_at, edit_history, author:users(full_name, email)',
      )
      .eq('team_id', teamId)
      .order('created_at', { ascending: false })

    if (error) throw error
    return (data || []) as WorkNoteRecord[]
  },

  async createNote(payload: {
    workspace_id: string
    team_id: string
    author_id: string
    subject: string
    content: string
    edit_history: Json
  }) {
    const { error } = await supabase.from('notes').insert(payload)

    if (error) throw error
  },

  async updateNote(
    noteId: string,
    payload: {
      subject: string
      content: string
      edit_history: Json
    },
  ) {
    const { error } = await supabase.from('notes').update(payload).eq('id', noteId)

    if (error) throw error
  },

  async deleteNote(noteId: string) {
    const { error } = await supabase.from('notes').delete().eq('id', noteId)

    if (error) throw error
  },

  async markTeamNotesRead(teamId: string, userId: string) {
    const { error } = await supabase
      .from('team_members')
      .update({ notes_last_read_at: new Date().toISOString() })
      .eq('team_id', teamId)
      .eq('user_id', userId)

    if (error) throw error
  },
}
