import React from 'react'
import { ChevronLeft, PenSquare, Trash2, History, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useTranslation } from 'react-i18next'
import { getDiffSegments } from '../utils/diff'
import type { Note } from '../types'

interface NoteReadPaneProps {
  activeNote: Note | null
  setActiveNoteId: (id: string | null) => void
  handleEditNote: (e: React.MouseEvent, id: string) => void
  handleDeleteNote: (e: React.MouseEvent, id: string) => void
  currentUser: string | undefined
  userRole: string
  expandedAudit: boolean
  setExpandedAudit: (expanded: boolean) => void
}

export const NoteReadPane: React.FC<NoteReadPaneProps> = ({
  activeNote,
  setActiveNoteId,
  handleEditNote,
  handleDeleteNote,
  currentUser,
  userRole,
  expandedAudit,
  setExpandedAudit,
}) => {
  const { t } = useTranslation()

  if (!activeNote) return null

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background">
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-border bg-sidebar px-6">
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setActiveNoteId(null)}
            className="h-8 gap-1.5 px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-secondary/60 hover:text-foreground"
          >
            <ChevronLeft className="h-3.5 w-3.5" />
            <span>{t('common.back', 'Tillbaka')}</span>
          </Button>
          <div className="h-4 w-px bg-border/40" />
          <h2 className="text-xs font-medium text-foreground">{t('notes.read.title')}</h2>
        </div>

        <div className="flex items-center gap-2">
          {activeNote.authorId === currentUser && (
            <Button
              variant="ghost"
              size="icon"
              className="text-muted-foreground hover:text-foreground"
              onClick={(e) => handleEditNote(e, activeNote.id)}
            >
              <PenSquare className="h-4 w-4" />
            </Button>
          )}
          {(activeNote.authorId === currentUser ||
            userRole === 'admin' ||
            userRole === 'platform_admin') && (
            <Button
              variant="ghost"
              size="icon"
              className="text-muted-foreground hover:text-rose-400"
              onClick={(e) => handleDeleteNote(e, activeNote.id)}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      <div className="scrollbar-dark flex-1 overflow-y-auto px-8 py-10 md:px-24 lg:px-48">
        <div className="mb-8 flex items-center justify-between border-b border-border pb-8">
          <div className="flex items-center gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-full border border-border bg-secondary font-medium text-foreground">
              {activeNote.author.charAt(0)}
            </div>
            <div>
              <div className="mb-1 text-sm font-medium text-foreground">
                {t('notes.read.written_by')} {activeNote.author}
              </div>
              <div className="text-xs text-muted-foreground">
                {t('notes.read.published')} {activeNote.date} {t('notes.read.at_time')}{' '}
                {activeNote.timestamp}
              </div>
            </div>
          </div>

          <div className="flex items-center">
            {(userRole === 'admin' || userRole === 'platform_admin') && (
              <button
                onClick={() => setExpandedAudit(!expandedAudit)}
                className={`flex items-center gap-2 rounded-md px-3 py-1.5 text-xs font-medium uppercase tracking-wider transition-colors ${
                  activeNote.editHistory.length > 0
                    ? 'text-amber-500 hover:bg-amber-500/10'
                    : 'bg-muted text-muted-foreground hover:text-muted-foreground'
                }`}
              >
                <History className="h-3.5 w-3.5" />
                {t('notes.read.history_button')}
              </button>
            )}
          </div>
        </div>

        {expandedAudit &&
          activeNote.editHistory.length > 0 &&
          (userRole === 'admin' || userRole === 'platform_admin') && (
            <div className="mb-10 overflow-hidden rounded-md border border-border bg-background p-0 animate-in fade-in slide-in-from-top-2">
              <div className="flex items-center justify-between border-b border-border bg-muted/30 p-4 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                <div className="flex items-center gap-2">
                  <History className="h-4 w-4 text-amber-500" /> {t('notes.read.audit_log_title')}
                </div>
                <span className="text-[10px] font-normal opacity-70">
                  {activeNote.editHistory.length} {t('notes.read.changes_recorded')}
                </span>
              </div>
              <div className="divide-y divide-border">
                {activeNote.editHistory.map((h, i) => (
                  <div key={i} className="bg-background p-4">
                    <div className="mb-3 flex items-start justify-between">
                      <div className="flex items-center gap-2">
                        <div className="flex h-6 w-6 items-center justify-center rounded-full bg-primary/10 text-[10px] font-medium text-primary">
                          {h.editedBy.charAt(0).toUpperCase()}
                        </div>
                        <div className="text-xs">
                          <span className="font-medium text-foreground">{h.editedBy}</span>
                          <span className="ml-2 text-muted-foreground">
                            {activeNote.date} kl {h.editedAt}
                          </span>
                        </div>
                      </div>
                    </div>

                    <div className="ml-8 space-y-3">
                      {h.oldSubject && (
                        <div className="flex flex-col gap-1 text-[11px]">
                          <span className="font-medium uppercase tracking-tight text-muted-foreground opacity-70">
                            {t('notes.compose.subject_label')}
                          </span>
                          <div className="flex items-center gap-2">
                            <span className="max-w-[150px] truncate text-rose-400 line-through opacity-50">
                              {h.oldSubject}
                            </span>
                            <ChevronRight className="h-3 w-3 text-muted-foreground" />
                            <span className="max-w-[200px] truncate font-medium text-emerald-400">
                              {h.newSubject}
                            </span>
                          </div>
                        </div>
                      )}

                      {h.oldContent && h.newContent && (
                        <div className="flex flex-col gap-1 text-[11px]">
                          <span className="font-medium uppercase tracking-tight text-muted-foreground opacity-70">
                            {t('notes.compose.content_label')}
                          </span>
                          <div className="relative whitespace-pre-wrap rounded border border-border/50 bg-muted/50 p-3 font-mono text-[10px] leading-relaxed">
                            {getDiffSegments(h.oldContent, h.newContent).map((seg, idx) => (
                              <span
                                key={idx}
                                className={
                                  seg.type === 'added'
                                    ? 'rounded bg-emerald-500/20 px-0.5 text-emerald-400'
                                    : seg.type === 'removed'
                                      ? 'rounded bg-rose-500/20 px-0.5 text-rose-400 line-through opacity-70'
                                      : ''
                                }
                              >
                                {seg.text}
                              </span>
                            ))}
                            <div className="absolute right-2 top-1 text-[8px] font-bold uppercase opacity-20">
                              {t('notes.read.diff_label')}
                            </div>
                          </div>
                        </div>
                      )}

                      {!h.oldSubject && !h.oldContent && (
                        <div className="text-[10px] italic text-muted-foreground">
                          {t('notes.read.metadata_change')}
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

        <h3 className="mb-6 text-xl font-medium leading-tight text-foreground">
          {activeNote.subject}
        </h3>

        <div className="whitespace-pre-wrap text-[15px] leading-loose text-muted-foreground">
          {activeNote.content}
        </div>
      </div>
    </div>
  )
}
