import React from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { assistanceService } from '@/services/assistanceService'
import { Plus, Pill, Clock, Activity } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'

export const MedicationTab: React.FC<{ clientId: string }> = ({ clientId }) => {
  const { t } = useTranslation()

  const { data: meds = [], isLoading } = useQuery({
    queryKey: ['medications', clientId],
    queryFn: () => assistanceService.getMedications(clientId),
  })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium tracking-tight text-foreground">
            {t('assistance.medication.title')}
          </h3>
          <p className="text-xs text-muted-foreground">{t('assistance.medication.subtitle')}</p>
        </div>
        <Button
          size="sm"
          className="h-8 rounded-md bg-primary/10 text-xs font-medium text-primary hover:bg-primary/20"
        >
          <Plus className="mr-1.5 h-3.5 w-3.5" /> {t('assistance.medication.add_button')}
        </Button>
      </div>

      {isLoading && meds.length === 0 ? (
        <div className="grid gap-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="flex items-center justify-between rounded-lg border border-border/50 bg-card p-4">
              <div className="flex items-center gap-3">
                <Skeleton className="size-8 rounded-md" />
                <div className="space-y-1.5">
                  <Skeleton className="h-4 w-36" />
                  <Skeleton className="h-3 w-24" />
                </div>
              </div>
              <Skeleton className="h-7 w-20 rounded-md" />
            </div>
          ))}
        </div>
      ) : meds.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-border bg-card/30 py-20 text-center shadow-inner">
          <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full bg-primary/5 text-primary/30">
            <Pill className="h-10 w-10" />
          </div>
          <h4 className="text-lg font-semibold text-foreground">
            {t('assistance.medication.empty_state_title')}
          </h4>
          <p className="mt-1 max-w-[240px] text-sm text-muted-foreground">
            {t('assistance.medication.empty_state')}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
          {meds.map((med) => (
            <div
              key={med.id}
              className="group relative overflow-hidden rounded-xl border border-border bg-card p-6 transition-all hover:border-primary/30 hover:shadow-sm"
            >
              <div className="absolute -right-4 -top-4 h-24 w-24 rounded-full bg-primary/5 transition-all group-hover:scale-150" />
              
              <div className="flex items-start gap-4">
                <div className="flex size-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
                  <Pill className="h-4 w-4" />
                </div>
                <div className="flex-1">
                  <h4 className="text-sm font-medium text-foreground">{med.name}</h4>
                  
                  <div className="mt-4 space-y-2">
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <Activity className="h-4 w-4 text-primary/60" />
                      <span className="font-medium text-foreground/80">{med.dosage}</span>
                    </div>
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <Clock className="h-4 w-4 text-primary/60" />
                      <span className="font-medium text-foreground/80">{med.frequency}</span>
                    </div>
                  </div>
                </div>
              </div>
              
              {med.instructions && (
                <div className="mt-5 rounded-lg bg-muted/50 p-4 text-xs font-medium text-muted-foreground ring-1 ring-border/50">
                  <span className="mb-1 block uppercase tracking-widest text-primary/70">
                    {t('assistance.medication.instructions_label')}
                  </span>
                  {med.instructions}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
