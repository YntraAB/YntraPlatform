import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Slider } from '@/components/ui/slider'
import { Calendar, Clock, Maximize2 } from 'lucide-react'
import { cn } from '@/lib/utils'
import { FloatingSaveBar } from './FloatingSaveBar'
import { toast } from 'sonner'

interface SchedulerSettingsProps {
  setIsSaving?: (val: boolean) => void
}

export const SchedulerSettings: React.FC<SchedulerSettingsProps> = ({ setIsSaving }) => {
  const { t } = useTranslation()
  const { settings, updateSettings, preferences, updatePreferences } = useWorkspace()

  const [localView, setLocalView] = useState(settings.default_calendar_view || 'week')
  const [localDensity, setLocalDensity] = useState(preferences.calendar_density || 'compact')
  const [localHours, setLocalHours] = useState({
    start: settings.business_hours?.start || 7,
    end: settings.business_hours?.end || 17,
  })

  const [baseline, setBaseline] = useState({
    view: settings.default_calendar_view || 'week',
    density: preferences.calendar_density || 'compact',
    start: settings.business_hours?.start || 7,
    end: settings.business_hours?.end || 17,
  })
  const [isInternalSaving, setIsInternalSaving] = useState(false)

  useEffect(() => {
    const initView = settings.default_calendar_view || 'week'
    const initDensity = preferences.calendar_density || 'compact'
    const initStart = settings.business_hours?.start || 7
    const initEnd = settings.business_hours?.end || 17

    setLocalView(initView)
    setLocalDensity(initDensity)
    setLocalHours({ start: initStart, end: initEnd })
    setBaseline({
      view: initView,
      density: initDensity,
      start: initStart,
      end: initEnd,
    })
  }, [settings.default_calendar_view, preferences.calendar_density, settings.business_hours])

  const isDirty =
    localView !== baseline.view ||
    localDensity !== baseline.density ||
    localHours.start !== baseline.start ||
    localHours.end !== baseline.end

  const handleReset = () => {
    setLocalView(baseline.view)
    setLocalDensity(baseline.density)
    setLocalHours({ start: baseline.start, end: baseline.end })
  }

  const handleSaveAll = async () => {
    setIsInternalSaving(true)
    setIsSaving?.(true)

    try {
      await updateSettings({
        default_calendar_view: localView as 'month' | 'week' | 'day' | undefined,
        business_hours: { ...settings.business_hours, start: localHours.start, end: localHours.end },
      })
      await updatePreferences({
        calendar_density: localDensity as 'compact' | 'relaxed',
      })

      setBaseline({
        view: localView,
        density: localDensity,
        start: localHours.start,
        end: localHours.end,
      })
      toast.success(t('common.saved', 'Ändringarna sparades'))
    } catch (err) {
      console.error('Failed to save scheduler settings:', err)
      toast.error('Kunde inte spara schemainställningarna')
    } finally {
      setIsInternalSaving(false)
      setTimeout(() => setIsSaving?.(false), 400)
    }
  }

  return (
    <div className="space-y-8 animate-in fade-in duration-300">
      {/* 1. Calendar View & Display */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Calendar className="h-4 w-4 text-muted-foreground/70" />
          <h2 className="text-sm font-medium tracking-tight text-foreground">
            {t('settings.scheduler.display')}
          </h2>
        </div>

        <div className="overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs divide-y divide-border/40">
          {/* Default Calendar View */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.scheduler.default_view')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Förvald visning när schemat öppnas
              </p>
            </div>
            <div className="w-full sm:w-48">
              <Select
                value={localView}
                onValueChange={(val) => setLocalView(val as 'month' | 'week' | 'day')}
              >
                <SelectTrigger className="h-[30px] w-full rounded-md border-border/60 bg-background text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="rounded-lg text-xs">
                  <SelectItem value="day" className="text-xs">{t('scheduler.views.day')}</SelectItem>
                  <SelectItem value="week" className="text-xs">{t('scheduler.views.week')}</SelectItem>
                  <SelectItem value="month" className="text-xs">{t('scheduler.views.month')}</SelectItem>
                  <SelectItem value="agenda" className="text-xs">{t('scheduler.views.agenda')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Density Toggle */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.scheduler.density')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Kompakt för hög informationsmängd, luftig för pekskärmar
              </p>
            </div>
            <div className="flex items-center gap-1.5">
              <button
                type="button"
                onClick={() => setLocalDensity('compact')}
                className={cn(
                  'flex h-[30px] items-center gap-1.5 rounded-md px-2.5 text-xs transition-colors',
                  localDensity === 'compact'
                    ? 'bg-secondary font-medium text-foreground shadow-xs border border-border/60'
                    : 'text-muted-foreground hover:bg-secondary/40 hover:text-foreground',
                )}
              >
                <Maximize2 className="h-3.5 w-3.5 rotate-45 scale-75" />
                <span>{t('settings.scheduler.density_compact')}</span>
              </button>
              <button
                type="button"
                onClick={() => setLocalDensity('relaxed')}
                className={cn(
                  'flex h-[30px] items-center gap-1.5 rounded-md px-2.5 text-xs transition-colors',
                  localDensity === 'relaxed'
                    ? 'bg-secondary font-medium text-foreground shadow-xs border border-border/60'
                    : 'text-muted-foreground hover:bg-secondary/40 hover:text-foreground',
                )}
              >
                <Maximize2 className="h-3.5 w-3.5" />
                <span>{t('settings.scheduler.density_relaxed')}</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* 2. Business Working Hours */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Clock className="h-4 w-4 text-muted-foreground/70" />
          <h2 className="text-sm font-medium tracking-tight text-foreground">
            {t('settings.scheduler.hours')}
          </h2>
        </div>

        <div className="overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs divide-y divide-border/40">
          {/* Start Hour */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3.5">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.scheduler.start_hour')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Tidigaste schemalagda tidpunkten på dygnet
              </p>
            </div>
            <div className="flex items-center gap-3 w-full sm:w-56">
              <Slider
                value={[localHours.start]}
                min={0}
                max={12}
                step={1}
                onValueChange={([val]) => setLocalHours((prev) => ({ ...prev, start: val }))}
                className="flex-1"
              />
              <span className="w-12 text-right font-mono text-xs text-foreground tabular-nums">
                {String(localHours.start).padStart(2, '0')}:00
              </span>
            </div>
          </div>

          {/* End Hour */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3.5">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.scheduler.end_hour')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Senaste schemalagda tidpunkten på dygnet
              </p>
            </div>
            <div className="flex items-center gap-3 w-full sm:w-56">
              <Slider
                value={[localHours.end]}
                min={13}
                max={23}
                step={1}
                onValueChange={([val]) => setLocalHours((prev) => ({ ...prev, end: val }))}
                className="flex-1"
              />
              <span className="w-12 text-right font-mono text-xs text-foreground tabular-nums">
                {String(localHours.end).padStart(2, '0')}:00
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Floating Save Dock */}
      <FloatingSaveBar
        show={isDirty}
        isSaving={isInternalSaving}
        onSave={handleSaveAll}
        onReset={handleReset}
      />
    </div>
  )
}

export default SchedulerSettings
