import { useMemo, useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router-dom'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'
import { useUnreadNotes } from '@/hooks/useUnreadNotes'
import { useSetDynamicBreadcrumbs } from '@/contexts/BreadcrumbContext'
import {
  useCreateNote,
  useDeleteNote,
  useMarkTeamNotesRead,
  useNoteTeams,
  useTeamNotes,
  useUpdateNote,
} from '@/hooks/queries/useNotes'
import type { Json } from '@/types/database'
import type { EditHistoryEntry, Note } from '../types'

export const useNotesPage = () => {
  const { t } = useTranslation()
  const { workspaceId } = useWorkspace()
  const { user } = useAuth()
  const unreadNotes = useUnreadNotes()
  const [searchParams] = useSearchParams()
  const setDynamicBreadcrumbs = useSetDynamicBreadcrumbs()

  const userRole = (user?.role as 'platform_admin' | 'admin' | 'assistant') || 'admin'
  const notesScope = userRole === 'platform_admin' ? 'all' : workspaceId || null

  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(searchParams.get('team'))
  const [activeNoteId, setActiveNoteId] = useState<string | null>(searchParams.get('note'))
  const [isComposing, setIsComposing] = useState(false)
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const [noteSearchQuery, setNoteSearchQuery] = useState('')
  const [composeText, setComposeText] = useState('')
  const [composeSubject, setComposeSubject] = useState('')
  const [expandedAudit, setExpandedAudit] = useState(false)

  const { data: teams = [] } = useNoteTeams(workspaceId || undefined, userRole === 'platform_admin')
  const selectedTeam = useMemo(
    () => teams.find((team) => team.id === selectedTeamId) || null,
    [teams, selectedTeamId],
  )
  const { data: teamNotes = [] } = useTeamNotes(selectedTeam?.id || null)

  const createNoteMutation = useCreateNote(notesScope, selectedTeam?.id || null)
  const updateNoteMutation = useUpdateNote(notesScope, selectedTeam?.id || null)
  const deleteNoteMutation = useDeleteNote(notesScope, selectedTeam?.id || null)
  const markTeamNotesReadMutation = useMarkTeamNotesRead()

  useEffect(() => {
    const teamId = searchParams.get('team')
    if (teamId) {
      setSelectedTeamId(teamId)
    }
  }, [searchParams])

  const notes = useMemo((): Note[] => {
    return teamNotes.map((dbNote) => {
      const createdAt = dbNote.created_at ? new Date(dbNote.created_at) : new Date()
      const authorData = Array.isArray(dbNote.author) ? dbNote.author[0] : dbNote.author
      const authorName =
        authorData?.full_name || authorData?.email || t('notes.general.unknown_agent')

      return {
        id: dbNote.id,
        teamId: dbNote.team_id,
        date: createdAt.toLocaleDateString(),
        timestamp: createdAt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
        author: authorName,
        authorId: dbNote.author_id || '',
        subject: dbNote.subject,
        content: dbNote.content,
        editHistory: Array.isArray(dbNote.edit_history)
          ? (dbNote.edit_history as unknown as EditHistoryEntry[])
          : [],
      }
    })
  }, [teamNotes, t])

  useEffect(() => {
    if (selectedTeamId && !selectedTeam && teams.length > 0) {
      setSelectedTeamId(null)
    }
  }, [selectedTeamId, selectedTeam, teams.length])

  useEffect(() => {
    if (activeNoteId && !notes.some((note) => note.id === activeNoteId)) {
      setActiveNoteId(null)
    }
  }, [activeNoteId, notes])

  const activeNoteSubject = useMemo(() => {
    if (!activeNoteId) return null
    return notes.find((n) => n.id === activeNoteId)?.subject || null
  }, [notes, activeNoteId])

  useEffect(() => {
    const breadcrumbs = []
    if (selectedTeam) {
      breadcrumbs.push({
        label: selectedTeam.displayName || selectedTeam.name,
        onClick: () => {
          setActiveNoteId(null)
          setIsComposing(false)
        },
      })
    }
    if (activeNoteSubject) {
      breadcrumbs.push({ label: activeNoteSubject })
    } else if (isComposing) {
      breadcrumbs.push({ label: t('notes.compose.title') })
    }
    setDynamicBreadcrumbs(breadcrumbs)
  }, [
    selectedTeam?.id,
    selectedTeam?.displayName,
    selectedTeam?.name,
    activeNoteSubject,
    isComposing,
    setDynamicBreadcrumbs,
    t,
  ])

  useEffect(() => {
    return () => {
      setDynamicBreadcrumbs([])
    }
  }, [setDynamicBreadcrumbs])

  useEffect(() => {
    const handleReset = (e: Event) => {
      const customEvent = e as CustomEvent<{ path: string }>
      if (customEvent.detail?.path === '/notes') {
        setSelectedTeamId(null)
        setActiveNoteId(null)
        setIsComposing(false)
      }
    }
    window.addEventListener('yntra:breadcrumb-navigate', handleReset)
    return () => {
      window.removeEventListener('yntra:breadcrumb-navigate', handleReset)
    }
  }, [])

  const activeNote = notes.find((n) => n.id === activeNoteId) || null

  const handleSaveNote = async () => {
    if (!selectedTeam || !composeSubject || !composeText || !user) return
    const targetWorkspaceId =
      userRole === 'platform_admin' ? selectedTeam.workspace_id : workspaceId
    if (!targetWorkspaceId) return

    if (editingNoteId) {
      const oldNote = notes.find((n) => n.id === editingNoteId)
      if (!oldNote) return

      const now = new Date()
      const timeStr =
        now.getHours().toString().padStart(2, '0') +
        ':' +
        now.getMinutes().toString().padStart(2, '0')

      const changeEntry: EditHistoryEntry = {
        editedBy: user?.name || user?.email || 'Unknown',
        editedAt: timeStr,
      }

      if (oldNote.subject !== composeSubject) {
        changeEntry.oldSubject = oldNote.subject
        changeEntry.newSubject = composeSubject
      }
      if (oldNote.content !== composeText) {
        changeEntry.oldContent = oldNote.content
        changeEntry.newContent = composeText
      }

      const newHistory = [changeEntry, ...oldNote.editHistory]

      await updateNoteMutation.mutateAsync({
        noteId: editingNoteId,
        subject: composeSubject,
        content: composeText,
        edit_history: newHistory as unknown as Json,
      })
    } else {
      await createNoteMutation.mutateAsync({
        workspace_id: targetWorkspaceId,
        team_id: selectedTeam.id,
        author_id: user.id,
        subject: composeSubject,
        content: composeText,
        edit_history: [],
      })
    }

    setIsComposing(false)
    setEditingNoteId(null)
    setComposeSubject('')
    setComposeText('')
  }

  const handleDeleteNote = async (e: React.MouseEvent | React.TouchEvent, id: string) => {
    e.stopPropagation()
    if (confirm(t('notes.list.delete_confirm'))) {
      await deleteNoteMutation.mutateAsync(id)
      if (activeNoteId === id) setActiveNoteId(null)
    }
  }

  const handleEditNote = (e: React.MouseEvent | React.TouchEvent, id: string) => {
    e.stopPropagation()
    const note = notes.find((n) => n.id === id)
    if (!note) return

    setComposeSubject(note.subject)
    setComposeText(note.content)
    setEditingNoteId(note.id)
    setIsComposing(true)
  }

  const handleSelectTeam = async (teamId: string) => {
    const unreadInTeam = unreadNotes?.byTeam[teamId] || 0
    setSelectedTeamId(teamId)
    setSearchQuery('')
    if (unreadInTeam > 0 && user) {
      await markTeamNotesReadMutation.mutateAsync({
        teamId: teamId,
        userId: user.id,
      })
    }
  }

  const startComposing = () => {
    setEditingNoteId(null)
    setComposeSubject('')
    setComposeText('')
    setIsComposing(true)
  }

  return {
    t,
    user,
    userRole,
    teams,
    selectedTeam,
    notes,
    activeNote,
    isComposing,
    editingNoteId,
    searchQuery,
    setSearchQuery,
    noteSearchQuery,
    setNoteSearchQuery,
    composeText,
    setComposeText,
    composeSubject,
    setComposeSubject,
    expandedAudit,
    setExpandedAudit,
    activeNoteId,
    setActiveNoteId,
    setSelectedTeamId,
    setIsComposing,
    handleSaveNote,
    handleDeleteNote,
    handleEditNote,
    handleSelectTeam,
    startComposing,
    unreadNotes,
  }
}
