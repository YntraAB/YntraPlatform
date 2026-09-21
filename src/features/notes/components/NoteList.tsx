import React from 'react'
import { Search, ChevronLeft, PenSquare, Trash2, Plus, Inbox, FileText, User, Users } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { useTranslation } from 'react-i18next'
import type { Note, NoteTeam } from '../types'

interface NoteListProps {
  selectedTeam: NoteTeam | null
  notes: Note[]
  noteSearchQuery: string
  setNoteSearchQuery: (query: string) => void
  setSelectedTeamId: (id: string | null) => void
  setActiveNoteId: (id: string | null) => void
  startComposing: () => void
  handleEditNote: (e: React.MouseEvent, id: string) => void
  handleDeleteNote: (e: React.MouseEvent, id: string) => void
  currentUser: string | undefined
  userRole: string
}

export const NoteList: React.FC<NoteListProps> = ({
  selectedTeam,
  notes,
  noteSearchQuery,
  setNoteSearchQuery,
  setSelectedTeamId,
  setActiveNoteId,
  startComposing,
  handleEditNote,
  handleDeleteNote,
  currentUser,
  userRole,
}) => {
  const { t } = useTranslation()

  if (!selectedTeam) return null

  let filteredNotes = notes.filter((n) => n.teamId === selectedTeam.id)
  if (noteSearchQuery) {
    filteredNotes = filteredNotes.filter(
      (n) =>
        n.subject.toLowerCase().includes(noteSearchQuery.toLowerCase()) ||
        n.content.toLowerCase().includes(noteSearchQuery.toLowerCase()),
    )
  }

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header Controls - exactly matching Inbox */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSelectedTeamId(null)}
            className="h-8 w-8 rounded-md text-muted-foreground hover:text-foreground hover:bg-secondary"
            aria-label={t('common.back')}
          >
            <ChevronLeft className="h-3.5 w-3.5" />
          </Button>
          <div className="h-3.5 w-px bg-border/40" />
          <div className="flex items-center gap-2">
            <h2 className="flex items-center gap-1.5 text-xs font-medium text-foreground">
              <Users className="h-3.5 w-3.5 text-muted-foreground" />
              {selectedTeam.displayName || selectedTeam.name}
            </h2>
            <div className="rounded border border-border/50 bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
              {filteredNotes.length} {t('notes.teams.notes_count')}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <div className="group relative w-64">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
            <Input
              placeholder={t('notes.list.search_placeholder')}
              value={noteSearchQuery}
              onChange={(e) => setNoteSearchQuery(e.target.value)}
              className="h-8 rounded-md border border-border/50 bg-muted/50 pl-8 text-xs text-foreground transition-all focus-visible:bg-muted focus-visible:ring-1 focus-visible:ring-primary"
            />
          </div>
        </div>
      </div>

      {/* Note Rows List - exactly matching Inbox row height & layout */}
      <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
        {filteredNotes.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
            <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
              <Inbox className="h-10 w-10 opacity-20" />
            </div>
            <p className="text-sm font-medium">{t('notes.list.empty_state')}</p>
          </div>
        ) : (
          <div className="flex w-full flex-col text-sm">
            {filteredNotes.map((note, idx) => {
              const canEdit = note.authorId === currentUser
              const canDelete = canEdit || userRole === 'admin' || userRole === 'platform_admin'

              return (
                <div
                  key={note.id}
                  onClick={() => setActiveNoteId(note.id)}
                  style={{ animationDelay: `${idx * 30}ms` }}
                  className="group flex h-12 cursor-pointer items-center border-b border-border/40 px-6 transition-all duration-300 hover:bg-secondary/40 animate-in fade-in slide-in-from-bottom-2 fill-mode-both"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') setActiveNoteId(note.id)
                  }}
                >
                  {/* Leading Note Icon */}
                  <div className="mr-3 flex size-6 shrink-0 items-center justify-center text-muted-foreground/70 transition-colors group-hover:text-primary">
                    <FileText className="h-4 w-4" />
                  </div>

                  {/* Sender / Author Column - with inline User icon */}
                  <div className="flex w-36 sm:w-48 md:w-56 shrink-0 items-center gap-1.5 truncate pr-3 text-sm font-medium text-foreground transition-colors">
                    <User className="h-3.5 w-3.5 shrink-0 text-muted-foreground/60" />
                    <span className="truncate">{note.author}</span>
                  </div>

                  {/* Subject + Snippet Column - identical to Inbox */}
                  <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
                    <span className="truncate text-sm font-normal text-foreground">
                      {note.subject}
                    </span>
                    <span className="truncate text-sm font-light italic text-muted-foreground/60">
                      — {note.content}
                    </span>
                  </div>

                  {/* Hover Actions & Timestamp - identical to Inbox */}
                  <div className="flex w-48 shrink-0 items-center justify-end">
                    <div className="mr-6 flex translate-x-2 items-center gap-3.5 text-muted-foreground/60 opacity-0 transition-all duration-300 group-hover:translate-x-0 group-hover:opacity-100">
                      {canEdit && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation()
                            handleEditNote(e, note.id)
                          }}
                          className="p-0.5 text-muted-foreground/60 transition-colors hover:text-primary"
                          title={t('common.edit')}
                        >
                          <PenSquare className="h-[18px] w-[18px]" />
                        </button>
                      )}
                      {canDelete && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation()
                            handleDeleteNote(e, note.id)
                          }}
                          className="p-0.5 text-muted-foreground/60 transition-colors hover:text-rose-500"
                          title={t('common.delete')}
                        >
                          <Trash2 className="h-[18px] w-[18px]" />
                        </button>
                      )}
                    </div>
                    <span className="text-[11px] font-medium tracking-tight uppercase tabular-nums text-muted-foreground/60 transition-colors">
                      {note.date === t('notes.list.today') ? note.timestamp : note.date}
                    </span>
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>

      {/* Floating Action Button - identical to Inbox */}
      <div className="absolute bottom-6 right-6 z-20">
        <Button
          onClick={startComposing}
          size="default"
          className="h-10 gap-2 rounded-lg px-4 font-medium shadow-md"
        >
          <Plus className="h-4 w-4" />
          <span>{t('notes.list.new_note_button')}</span>
        </Button>
      </div>
    </div>
  )
}


