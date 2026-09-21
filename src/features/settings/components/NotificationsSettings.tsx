import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '@/hooks/useAuth'
import { supabase } from '@/lib/supabase'
import { Switch } from '@/components/ui/switch'
import { Mail, BellRing, Check, Bell } from 'lucide-react'
import { cn } from '@/lib/utils'

interface NotificationsSettingsProps {
  setIsSaving: (val: boolean) => void
}

export const NotificationsSettings: React.FC<NotificationsSettingsProps> = ({ setIsSaving }) => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const [notifSettings, setNotifSettings] = useState({
    on: true,
    type: 'full_content',
  })

  useEffect(() => {
    async function loadUserSettings() {
      if (!user) return
      const { data } = await supabase
        .from('users')
        .select('notifications_on, notification_type')
        .eq('id', user.id)
        .single()
      if (data) {
        setNotifSettings({
          on: !!data.notifications_on,
          type: data.notification_type || 'full_content',
        })
      }
    }
    loadUserSettings()
  }, [user])

  const updateNotifSetting = async (key: 'on' | 'type', value: string | boolean) => {
    if (!user) return
    setIsSaving(true)
    const newSettings = { ...notifSettings, [key]: value }
    setNotifSettings(newSettings)

    await supabase
      .from('users')
      .update({
        notifications_on: newSettings.on,
        notification_type: newSettings.type,
      })
      .eq('id', user.id)

    setTimeout(() => setIsSaving(false), 500)
  }

  return (
    <div className="space-y-8 animate-in fade-in duration-300">
      {/* 1. Master Notification Toggle */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Bell className="h-4 w-4 text-muted-foreground/70" />
          <h2 className="text-sm font-medium tracking-tight text-foreground">
            {t('settings.notifications.title')}
          </h2>
        </div>

        <div className="overflow-hidden rounded-lg border border-border/40 bg-card/40 divide-y divide-border/30">
          <div className="flex items-center justify-between gap-4 px-4 py-3.5">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                Aktivera aviseringar
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                {t('settings.notifications.desc')}
              </p>
            </div>
            <Switch
              checked={notifSettings.on}
              onCheckedChange={(c) => updateNotifSetting('on', c)}
              className="scale-85"
            />
          </div>
        </div>
      </div>

      {/* 2. Notification Format / Type */}
      <div className="space-y-3">
        <h3 className="text-xs font-medium text-muted-foreground/70">
          Aviseringsstil och integritet
        </h3>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {/* Full Content Option */}
          <button
            type="button"
            disabled={!notifSettings.on}
            onClick={() => updateNotifSetting('type', 'full_content')}
            className={cn(
              'group relative flex flex-col justify-between rounded-lg border p-4 text-left transition-colors',
              notifSettings.type === 'full_content'
                ? 'border-foreground/30 bg-secondary/40'
                : 'border-border/40 bg-card/30 hover:bg-secondary/20',
              !notifSettings.on && 'cursor-not-allowed opacity-40',
            )}
          >
            <div className="flex items-start justify-between gap-2">
              <div className="flex size-7 items-center justify-center rounded-md border border-border/40 bg-secondary/40 text-foreground">
                <BellRing className="h-3.5 w-3.5" />
              </div>
              {notifSettings.type === 'full_content' && (
                <div className="flex size-4 items-center justify-center rounded-full bg-primary text-primary-foreground">
                  <Check className="h-2.5 w-2.5" />
                </div>
              )}
            </div>
            <div className="mt-3 space-y-1">
              <h4 className="text-xs font-medium text-foreground">
                {t('settings.notifications.full_content')}
              </h4>
              <p className="text-[11px] font-light text-muted-foreground/70 leading-relaxed">
                {t('settings.notifications.full_content_desc')}
              </p>
            </div>
          </button>

          {/* Alert Only Option */}
          <button
            type="button"
            disabled={!notifSettings.on}
            onClick={() => updateNotifSetting('type', 'alert_only')}
            className={cn(
              'group relative flex flex-col justify-between rounded-lg border p-4 text-left transition-colors',
              notifSettings.type === 'alert_only'
                ? 'border-foreground/30 bg-secondary/40'
                : 'border-border/40 bg-card/30 hover:bg-secondary/20',
              !notifSettings.on && 'cursor-not-allowed opacity-40',
            )}
          >
            <div className="flex items-start justify-between gap-2">
              <div className="flex size-7 items-center justify-center rounded-md border border-border/40 bg-secondary/40 text-foreground">
                <Mail className="h-3.5 w-3.5" />
              </div>
              {notifSettings.type === 'alert_only' && (
                <div className="flex size-4 items-center justify-center rounded-full bg-primary text-primary-foreground">
                  <Check className="h-2.5 w-2.5" />
                </div>
              )}
            </div>
            <div className="mt-3 space-y-1">
              <h4 className="text-xs font-medium text-foreground">
                {t('settings.notifications.alert_only')}
              </h4>
              <p className="text-[11px] font-light text-muted-foreground/70 leading-relaxed">
                {t('settings.notifications.alert_only_desc')}
              </p>
            </div>
          </button>
        </div>

        {/* Quiet footer notice */}
        <p className="text-[11px] font-light text-muted-foreground/50 pt-1">
          {t('settings.notifications.no_spam')}
        </p>
      </div>
    </div>
  )
}

export default NotificationsSettings
