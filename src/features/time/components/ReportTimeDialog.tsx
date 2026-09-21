import React from 'react'
import { useTranslation } from 'react-i18next'
import { Clock, Check } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

interface ReportTimeDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export const ReportTimeDialog: React.FC<ReportTimeDialogProps> = ({ open, onOpenChange }) => {
  const { t } = useTranslation()

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[360px] overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl gap-0">
        <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <Clock className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>{t('timereports.report_time', 'Rapportera tid')}</span>
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3.5 p-5">
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-muted-foreground">
              {t('timereports.client_or_team', 'Brukare / Team')}
            </label>
            <select className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none transition-colors focus:border-border focus:ring-1 focus:ring-ring">
              <option value="börje">Börje Olofsson</option>
              <option value="eva">Eva Larsson</option>
            </select>
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-muted-foreground">
              {t('common.date', 'Datum')}
            </label>
            <Input
              type="date"
              className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
            />
          </div>

          <div className="flex gap-3">
            <div className="flex flex-1 flex-col gap-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                {t('timereports.start', 'Start')}
              </label>
              <Input
                type="time"
                defaultValue="08:00"
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
              />
            </div>
            <div className="flex flex-1 flex-col gap-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                {t('timereports.end', 'Slut')}
              </label>
              <Input
                type="time"
                defaultValue="16:00"
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
              />
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-muted-foreground">
              {t('timereports.break_minutes', 'Rast (minuter)')}
            </label>
            <Input
              type="number"
              placeholder="0"
              className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-muted-foreground">
              {t('timereports.note', 'Anteckning')}
            </label>
            <Input
              type="text"
              placeholder={t('timereports.note_placeholder', 'Valfri anteckning...')}
              className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
            />
          </div>
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onOpenChange(false)}
            className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
          >
            {t('common.cancel', 'Avbryt')}
          </Button>
          <Button
            size="sm"
            onClick={() => onOpenChange(false)}
            className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
          >
            <Check className="mr-1.5 size-3" />
            <span>{t('timereports.save_shift', 'Spara pass')}</span>
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
