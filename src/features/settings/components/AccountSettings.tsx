import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '@/hooks/useAuth'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { Input } from '@/components/ui/input'
import { Slider } from '@/components/ui/slider'
import {
  User,
  Palette,
  Sun,
  Moon,
  Monitor,
} from 'lucide-react'
import { useTheme, type Theme } from '@/components/theme-provider'
import { cn } from '@/lib/utils'
import { supabase } from '@/lib/supabase'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { UserPreferences, UserPrivacySettings } from '@/types'
import { FloatingSaveBar } from './FloatingSaveBar'
import { toast } from 'sonner'

interface AccountSettingsProps {
  setIsSaving?: (val: boolean) => void
}

export const AccountSettings: React.FC<AccountSettingsProps> = ({ setIsSaving }) => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const { preferences, updatePreferences } = useWorkspace()
  const { theme, setTheme } = useTheme()

  const getSafeScale = (val: number | undefined) => {
    if (!val) return 1
    return val > 2 ? val / 100 : val
  }

  const [localFontScale, setLocalFontScale] = useState(getSafeScale(preferences.font_scale))
  const [name, setName] = useState(user?.name || '')
  const [phone, setPhone] = useState(user?.phone || '')
  const [location, setLocation] = useState(user?.location || '')
  const [privacy, setPrivacy] = useState<UserPrivacySettings>(
    user?.privacy_settings || {
      phone: 'organization',
      location: 'organization',
    },
  )

  const [baseline, setBaseline] = useState({
    name: user?.name || '',
    phone: user?.phone || '',
    location: user?.location || '',
    privacyPhone: user?.privacy_settings?.phone || 'organization',
    privacyLocation: user?.privacy_settings?.location || 'organization',
  })
  const [isInternalSaving, setIsInternalSaving] = useState(false)

  useEffect(() => {
    if (user) {
      const initialName = user.name || ''
      const initialPhone = user.phone || ''
      const initialLocation = user.location || ''
      const initialPrivacyPhone = user.privacy_settings?.phone || 'organization'
      const initialPrivacyLocation = user.privacy_settings?.location || 'organization'

      setName(initialName)
      setPhone(initialPhone)
      setLocation(initialLocation)
      setPrivacy({
        phone: initialPrivacyPhone,
        location: initialPrivacyLocation,
      })
      setBaseline({
        name: initialName,
        phone: initialPhone,
        location: initialLocation,
        privacyPhone: initialPrivacyPhone,
        privacyLocation: initialPrivacyLocation,
      })
    }
  }, [user])

  useEffect(() => {
    setLocalFontScale(getSafeScale(preferences.font_scale))
  }, [preferences.font_scale])

  const isDirty =
    name !== baseline.name ||
    phone !== baseline.phone ||
    location !== baseline.location ||
    privacy.phone !== baseline.privacyPhone ||
    privacy.location !== baseline.privacyLocation

  const handleReset = () => {
    setName(baseline.name)
    setPhone(baseline.phone)
    setLocation(baseline.location)
    setPrivacy({
      phone: baseline.privacyPhone as 'everyone' | 'organization' | 'none',
      location: baseline.privacyLocation as 'everyone' | 'organization' | 'none',
    })
  }

  const handleUpdatePreference = async (newPrefs: Partial<UserPreferences>) => {
    setIsSaving?.(true)
    await updatePreferences(newPrefs)
    setTimeout(() => setIsSaving?.(false), 500)
  }

  const handleSaveProfile = async () => {
    if (!user) return
    setIsInternalSaving(true)
    setIsSaving?.(true)

    try {
      await supabase.auth.updateUser({
        data: { full_name: name },
      })

      await supabase
        .from('users')
        .update({
          full_name: name,
          phone,
          location,
          privacy_settings: privacy,
        })
        .eq('id', user.id)

      setBaseline({
        name,
        phone,
        location,
        privacyPhone: privacy.phone,
        privacyLocation: privacy.location,
      })
      toast.success(t('common.saved', 'Ändringarna sparades'))
    } catch (error) {
      console.error('Failed to update profile:', error)
      toast.error('Kunde inte spara ändringarna')
    } finally {
      setIsInternalSaving(false)
      setTimeout(() => setIsSaving?.(false), 400)
    }
  }

  return (
    <div className="space-y-8 animate-in fade-in duration-300">
      {/* 1. Profile Section */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <User className="h-4 w-4 text-muted-foreground/70" />
          <h2 className="text-sm font-medium tracking-tight text-foreground">
            {t('settings.account.profile')}
          </h2>
        </div>

        <div className="overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs divide-y divide-border/40">
          {/* Email (Read-only) */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('common.email')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                {t('settings.account.email_change_info')}
              </p>
            </div>
            <div className="w-full sm:w-64">
              <Input
                value={user?.email || ''}
                disabled
                className="h-[30px] rounded-md border-border/50 bg-muted/40 text-xs text-muted-foreground/80 cursor-not-allowed"
              />
            </div>
          </div>

          {/* Full Name */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('common.name')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Ditt visningsnamn i schemat och meddelanden
              </p>
            </div>
            <div className="w-full sm:w-64">
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="h-[30px] rounded-md border-border/60 bg-background text-xs px-2.5 transition-colors focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>
          </div>

          {/* Phone + Privacy */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.account.phone')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Kontaktnummer för akuta pass och aviseringar
              </p>
            </div>
            <div className="flex items-center gap-2 w-full sm:w-72">
              <Input
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                placeholder="+46..."
                className="h-[30px] flex-1 rounded-md border-border/60 bg-background text-xs px-2.5"
              />
              <Select
                value={privacy.phone}
                onValueChange={(val: string) =>
                  setPrivacy((prev) => ({
                    ...prev,
                    phone: val as 'everyone' | 'organization' | 'none',
                  }))
                }
              >
                <SelectTrigger className="h-[30px] w-28 rounded-md border-border/60 bg-background text-[11px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="rounded-lg text-xs">
                  <SelectItem value="everyone" className="text-xs">
                    {t('settings.account.visibility_everyone')}
                  </SelectItem>
                  <SelectItem value="organization" className="text-xs">
                    {t('settings.account.visibility_organization')}
                  </SelectItem>
                  <SelectItem value="none" className="text-xs">
                    {t('settings.account.visibility_none')}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Location + Privacy */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.account.location')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Plats eller basstation
              </p>
            </div>
            <div className="flex items-center gap-2 w-full sm:w-72">
              <Input
                value={location}
                onChange={(e) => setLocation(e.target.value)}
                placeholder="Stockholm, SE"
                className="h-[30px] flex-1 rounded-md border-border/60 bg-background text-xs px-2.5"
              />
              <Select
                value={privacy.location}
                onValueChange={(val: string) =>
                  setPrivacy((prev) => ({
                    ...prev,
                    location: val as 'everyone' | 'organization' | 'none',
                  }))
                }
              >
                <SelectTrigger className="h-[30px] w-28 rounded-md border-border/60 bg-background text-[11px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="rounded-lg text-xs">
                  <SelectItem value="everyone" className="text-xs">
                    {t('settings.account.visibility_everyone')}
                  </SelectItem>
                  <SelectItem value="organization" className="text-xs">
                    {t('settings.account.visibility_organization')}
                  </SelectItem>
                  <SelectItem value="none" className="text-xs">
                    {t('settings.account.visibility_none')}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>
      </div>

      {/* 2. Appearance Section */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Palette className="h-4 w-4 text-muted-foreground/70" />
          <h2 className="text-sm font-medium tracking-tight text-foreground">
            {t('settings.account.appearance')}
          </h2>
        </div>

        <div className="overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs divide-y divide-border/40">
          {/* Theme Row - Balanced, neat theme buttons */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3.5">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.choose_theme')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Välj mellan mörkt, ljust eller systemsynk
              </p>
            </div>
            <div className="flex items-center gap-2">
              {[
                { value: 'dark', icon: Moon, label: t('settings.theme_dark') },
                { value: 'light', icon: Sun, label: t('settings.theme_light') },
                { value: 'system', icon: Monitor, label: t('settings.theme_system') },
              ].map((item) => {
                const isActive = theme === item.value
                return (
                  <button
                    key={item.value}
                    type="button"
                    onClick={() => {
                      setTheme(item.value as Theme)
                      handleUpdatePreference({ theme: item.value as Theme })
                    }}
                    className={cn(
                      'flex h-[34px] items-center gap-2 rounded-md px-3 text-xs font-medium transition-colors',
                      isActive
                        ? 'bg-secondary text-foreground shadow-xs border border-border/70'
                        : 'text-muted-foreground hover:bg-secondary/40 hover:text-foreground border border-transparent',
                    )}
                  >
                    <item.icon className="h-3.5 w-3.5" />
                    <span>{item.label}</span>
                  </button>
                )
              })}
            </div>
          </div>

          {/* Font Scale Row */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3.5">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.font_scale')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Justera gränssnittets textstorlek
              </p>
            </div>
            <div className="flex items-center gap-3 w-full sm:w-56">
              <Slider
                value={[localFontScale]}
                min={0.8}
                max={1.2}
                step={0.05}
                onValueChange={([val]) => setLocalFontScale(val)}
                onValueCommit={([val]) => handleUpdatePreference({ font_scale: val })}
                className="flex-1"
              />
              <span className="w-12 text-right font-mono text-xs text-muted-foreground tabular-nums">
                {Math.round(localFontScale * 100)}%
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Floating Save Dock */}
      <FloatingSaveBar
        show={isDirty}
        isSaving={isInternalSaving}
        onSave={handleSaveProfile}
        onReset={handleReset}
      />
    </div>
  )
}

export default AccountSettings
