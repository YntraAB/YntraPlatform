export interface EditHistoryEntry {
  editedBy: string
  editedAt: string
  oldSubject?: string
  newSubject?: string
  oldContent?: string
  newContent?: string
  [key: string]: unknown
}

export interface Note {
  id: string
  teamId: string
  date: string
  timestamp: string
  author: string
  authorId: string
  subject: string
  content: string
  editHistory: EditHistoryEntry[]
}

export interface NoteTeam {
  id: string
  name: string
  workspace_id: string
  notesCount: number
  displayName: string
  recentNote: string | null
}
