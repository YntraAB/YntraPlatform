import React from 'react'
import { ChevronLeft, Check } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useTranslation } from 'react-i18next'
import type { NoteTeam } from '../types'

interface NoteComposePaneProps {
  selectedTeam: NoteTeam | null
  setIsComposing: (isComposing: boolean) => void
  composeSubject: string
  setComposeSubject: (subject: string) => void
  composeText: string
  setComposeText: (text: string) => void
  handleSaveNote: () => void
}

export const NoteComposePane: React.FC<NoteComposePaneProps> = ({
  selectedTeam,
  setIsComposing,
  composeSubject,
  setComposeSubject,
  composeText,
  setComposeText,
  handleSaveNote,
}) => {
  const { t } = useTranslation()

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background">
      <div className="flex h-12 items-center justify-between border-b border-border px-6">
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsComposing(false)}
            className="h-8 gap-1.5 px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-secondary/60 hover:text-foreground"
          >
            <ChevronLeft className="h-3.5 w-3.5" />
            <span>{t('common.back', 'Tillbaka')}</span>
          </Button>
          <div className="h-4 w-px bg-border/40" />
          <h2 className="text-xs font-medium text-foreground">
            {t('notes.compose.title')} {selectedTeam?.displayName || selectedTeam?.name}
          </h2>
        </div>

        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            className="text-xs font-medium text-muted-foreground hover:text-foreground"
            onClick={() => setIsComposing(false)}
          >
            {t('notes.compose.cancel')}
          </Button>
          <Button
            className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs transition-all hover:bg-primary/90"
            disabled={composeText.trim().length === 0}
            onClick={handleSaveNote}
          >
            <Check className="mr-1.5 h-3.5 w-3.5" />
            {t('notes.compose.save_button')}
          </Button>
        </div>
      </div>

      <div className="scrollbar-dark flex flex-1 flex-col overflow-y-auto px-6 py-6 md:px-12 md:py-8">
        <div className="mx-auto flex w-full max-w-3xl flex-col gap-5">
          <div className="flex items-center gap-3 border-b border-border/40 pb-3 transition-colors">
            <label className="w-14 shrink-0 text-xs font-normal text-muted-foreground">
              {t('notes.compose.subject_label')}
            </label>
            <input
              placeholder={t('notes.compose.subject_placeholder')}
              value={composeSubject}
              onChange={(e) => setComposeSubject(e.target.value)}
              className="h-8 flex-1 border-none bg-transparent px-0 text-xs font-normal text-foreground placeholder:text-xs placeholder:text-muted-foreground/40 focus:outline-none focus:ring-0"
              autoFocus
            />
          </div>

          <div className="flex flex-1 flex-col pb-8">
            <textarea
              value={composeText}
              onChange={(e) => setComposeText(e.target.value)}
              className="min-h-[200px] w-full flex-1 resize-none border-none bg-transparent text-xs font-normal leading-relaxed text-foreground/90 placeholder:text-xs placeholder:text-muted-foreground/40 focus:outline-none"
              placeholder={t('notes.compose.content_placeholder')}
            />
          </div>
        </div>
      </div>
    </div>
  )
}
