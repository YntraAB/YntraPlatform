import { useMemo } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { queryKeys } from '@/lib/query-keys'
import { noteService } from '@/services/noteService'
import type { Json } from '@/types/database'


export function useNoteTeams(workspaceId?: string, includeAllWorkspaces = false) {
  const scope = includeAllWorkspaces ? 'all' : workspaceId || null
  const effectiveWorkspaceId = includeAllWorkspaces ? undefined : workspaceId


  return useQuery({
    queryKey: queryKeys.noteTeams(scope),
    queryFn: () => noteService.getNoteTeams(effectiveWorkspaceId),
    enabled: includeAllWorkspaces || !!workspaceId,
  })
}

export function useTeamNotes(teamId: string | null) {
  const queryKey = useMemo(() => queryKeys.teamNotes(teamId), [teamId])


  return useQuery({
    queryKey,
    queryFn: () => noteService.getTeamNotes(teamId!),
    enabled: !!teamId,
  })
}

export function useCreateNote(noteScope: string | null, teamId: string | null) {
  const queryClient = useQueryClient()
  const teamsKey = queryKeys.noteTeams(noteScope)
  const notesKey = queryKeys.teamNotes(teamId)

  return useMutation({
    mutationFn: (payload: {
      workspace_id: string
      team_id: string
      author_id: string
      subject: string
      content: string
      edit_history: Json
    }) => noteService.createNote(payload),
    onMutate: async (newNote) => {
      await queryClient.cancelQueries({ queryKey: notesKey })
      const previousNotes = queryClient.getQueryData(notesKey)
      queryClient.setQueryData(notesKey, (old: any) => [
        { ...newNote, id: Math.random().toString(36).substring(2, 11), created_at: new Date().toISOString() },
        ...(old || []),
      ])
      return { previousNotes }
    },
    onError: (_err, _newNote, context) => {
      queryClient.setQueryData(notesKey, context?.previousNotes)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: teamsKey })
      queryClient.invalidateQueries({ queryKey: notesKey })
    },
  })
}

export function useUpdateNote(noteScope: string | null, teamId: string | null) {
  const queryClient = useQueryClient()
  const teamsKey = queryKeys.noteTeams(noteScope)
  const notesKey = queryKeys.teamNotes(teamId)

  return useMutation({
    mutationFn: ({
      noteId,
      subject,
      content,
      edit_history,
    }: {
      noteId: string
      subject: string
      content: string
      edit_history: Json
    }) => noteService.updateNote(noteId, { subject, content, edit_history }),
    onMutate: async ({ noteId, subject, content, edit_history }) => {
      await queryClient.cancelQueries({ queryKey: notesKey })
      const previousNotes = queryClient.getQueryData(notesKey)
      queryClient.setQueryData(notesKey, (old: any) =>
        old?.map((note: any) =>
          note.id === noteId ? { ...note, subject, content, edit_history } : note,
        ),
      )
      return { previousNotes }
    },
    onError: (_err, _variables, context) => {
      queryClient.setQueryData(notesKey, context?.previousNotes)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: teamsKey })
      queryClient.invalidateQueries({ queryKey: notesKey })
    },
  })
}

export function useDeleteNote(noteScope: string | null, teamId: string | null) {
  const queryClient = useQueryClient()
  const teamsKey = queryKeys.noteTeams(noteScope)
  const notesKey = queryKeys.teamNotes(teamId)

  return useMutation({
    mutationFn: (noteId: string) => noteService.deleteNote(noteId),
    onMutate: async (noteId) => {
      await queryClient.cancelQueries({ queryKey: notesKey })
      const previousNotes = queryClient.getQueryData(notesKey)
      queryClient.setQueryData(notesKey, (old: any) =>
        old?.filter((note: any) => note.id !== noteId),
      )
      return { previousNotes }
    },
    onError: (_err, _noteId, context) => {
      queryClient.setQueryData(notesKey, context?.previousNotes)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: teamsKey })
      queryClient.invalidateQueries({ queryKey: notesKey })
    },
  })
}

export function useMarkTeamNotesRead() {
  return useMutation({
    mutationFn: ({ teamId, userId }: { teamId: string; userId: string }) =>
      noteService.markTeamNotesRead(teamId, userId),
  })
}
