import React from 'react'
import { Button } from '@/components/ui/button'
import { Loader2 } from 'lucide-react'

interface FloatingSaveBarProps {
  show: boolean
  isSaving: boolean
  onSave: () => void | Promise<void>
  onReset: () => void
  title?: string
  saveLabel?: string
  resetLabel?: string
}

export const FloatingSaveBar: React.FC<FloatingSaveBarProps> = ({
  show,
  isSaving,
  onSave,
  onReset,
  title = 'Osparade ändringar',
  saveLabel = 'Spara ändringar',
  resetLabel = 'Återställ',
}) => {
  if (!show) return null

  return (
    <div className="fixed bottom-6 inset-x-0 mx-auto w-[calc(100%-2rem)] max-w-md z-50 animate-in fade-in slide-in-from-bottom-5 duration-200">
      <div className="flex items-center justify-between gap-4 rounded-lg border border-border/80 bg-card/95 px-4 py-2.5 shadow-2xl backdrop-blur-md">
        <div className="flex items-center gap-2 min-w-0">
          <span className="truncate text-xs font-medium text-foreground">
            {title}
          </span>
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          <Button
            type="button"
            variant="ghost"
            onClick={onReset}
            disabled={isSaving}
            className="h-[30px] rounded-md px-2.5 text-xs text-muted-foreground hover:text-foreground"
          >
            {resetLabel}
          </Button>

          <Button
            type="button"
            onClick={onSave}
            disabled={isSaving}
            className="h-[30px] rounded-md px-3.5 text-xs font-medium bg-primary text-primary-foreground hover:bg-primary/90 shadow-xs flex items-center gap-1.5"
          >
            {isSaving ? (
              <>
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                <span>Sparar...</span>
              </>
            ) : (
              <span>{saveLabel}</span>
            )}
          </Button>
        </div>
      </div>
    </div>
  )
}

export default FloatingSaveBar
