import React, { useState, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { supabase } from '@/lib/supabase'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ImageCropperDialog } from './ImageCropperDialog'
import {
  Building2,
  Globe,
  Upload,
  X,
  Loader2,
} from 'lucide-react'
import type { WorkspaceSettings } from '@/types'
import { FloatingSaveBar } from './FloatingSaveBar'
import { toast } from 'sonner'

interface GeneralSettingsProps {
  setIsSaving?: (val: boolean) => void
}

export const GeneralSettings: React.FC<GeneralSettingsProps> = ({ setIsSaving }) => {
  const { t } = useTranslation()
  const {
    settings,
    updateSettings,
    workspaceName,
    workspaceLogo,
    brandColor,
    updateWorkspace,
    workspaceId,
  } = useWorkspace()

  const [localName, setLocalName] = useState(workspaceName || '')
  const [localLogo, setLocalLogo] = useState(workspaceLogo || '')
  const [localBrandColor, setLocalBrandColor] = useState(brandColor || '#3b82f6')
  const [localLanguage, setLocalLanguage] = useState(settings.language || 'sv')
  const [localTimezone, setLocalTimezone] = useState(settings.timezone || 'Europe/Stockholm')
  const [localWeekStart, setLocalWeekStart] = useState(settings.week_start ?? 1)

  const [baseline, setBaseline] = useState({
    name: workspaceName || '',
    brandColor: brandColor || '#3b82f6',
    language: settings.language || 'sv',
    timezone: settings.timezone || 'Europe/Stockholm',
    weekStart: settings.week_start ?? 1,
  })

  const [isUploading, setIsUploading] = useState(false)
  const [isInternalSaving, setIsInternalSaving] = useState(false)
  const [cropperOpen, setCropperOpen] = useState(false)
  const [selectedImage, setSelectedImage] = useState<string | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    const initName = workspaceName || ''
    const initBrand = brandColor || '#3b82f6'
    const initLang = settings.language || 'sv'
    const initTz = settings.timezone || 'Europe/Stockholm'
    const initWs = settings.week_start ?? 1

    setLocalName(initName)
    setLocalBrandColor(initBrand)
    setLocalLanguage(initLang)
    setLocalTimezone(initTz)
    setLocalWeekStart(initWs)
    setBaseline({
      name: initName,
      brandColor: initBrand,
      language: initLang,
      timezone: initTz,
      weekStart: initWs,
    })
  }, [workspaceName, brandColor, settings.language, settings.timezone, settings.week_start])

  useEffect(() => {
    setLocalLogo(workspaceLogo || '')
  }, [workspaceLogo])

  const isDirty =
    localName !== baseline.name ||
    localBrandColor !== baseline.brandColor ||
    localLanguage !== baseline.language ||
    localTimezone !== baseline.timezone ||
    localWeekStart !== baseline.weekStart

  const handleReset = () => {
    setLocalName(baseline.name)
    setLocalBrandColor(baseline.brandColor)
    setLocalLanguage(baseline.language)
    setLocalTimezone(baseline.timezone)
    setLocalWeekStart(baseline.weekStart)
  }

  const handleSaveAll = async () => {
    setIsInternalSaving(true)
    setIsSaving?.(true)

    try {
      const workspaceUpdates: { name?: string; brand_color?: string } = {}
      if (localName !== baseline.name) workspaceUpdates.name = localName
      if (localBrandColor !== baseline.brandColor) workspaceUpdates.brand_color = localBrandColor
      if (Object.keys(workspaceUpdates).length > 0) {
        await updateWorkspace(workspaceUpdates)
      }

      const settingsUpdates: Partial<WorkspaceSettings> = {}
      if (localLanguage !== baseline.language) settingsUpdates.language = localLanguage
      if (localTimezone !== baseline.timezone) settingsUpdates.timezone = localTimezone
      if (localWeekStart !== baseline.weekStart) settingsUpdates.week_start = localWeekStart
      if (Object.keys(settingsUpdates).length > 0) {
        await updateSettings(settingsUpdates)
      }

      setBaseline({
        name: localName,
        brandColor: localBrandColor,
        language: localLanguage,
        timezone: localTimezone,
        weekStart: localWeekStart,
      })
      toast.success(t('common.saved', 'Ändringarna sparades'))
    } catch (err) {
      console.error('Failed to save settings:', err)
      toast.error('Kunde inte spara inställningarna')
    } finally {
      setIsInternalSaving(false)
      setTimeout(() => setIsSaving?.(false), 400)
    }
  }

  const handleUpdateLogo = async (logo_url: string | null) => {
    setIsSaving?.(true)
    await updateWorkspace({ logo_url })
    setLocalLogo(logo_url || '')
    setTimeout(() => setIsSaving?.(false), 500)
  }

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    const reader = new FileReader()
    reader.onload = () => {
      setSelectedImage(reader.result as string)
      setCropperOpen(true)
    }
    reader.readAsDataURL(file)
    e.target.value = ''
  }

  const handleCropComplete = async (croppedBlob: Blob) => {
    if (!workspaceId) return

    setIsUploading(true)
    setIsSaving?.(true)
    try {
      const fileName = `${workspaceId}-${Date.now()}.png`
      const { error: uploadError } = await supabase.storage
        .from('logos')
        .upload(fileName, croppedBlob)

      if (uploadError) throw uploadError

      const {
        data: { publicUrl },
      } = supabase.storage.from('logos').getPublicUrl(fileName)

      await updateWorkspace({ logo_url: publicUrl })
      setLocalLogo(publicUrl)
    } catch (error) {
      console.error('Error uploading logo:', error)
    } finally {
      setIsUploading(false)
      setTimeout(() => setIsSaving?.(false), 500)
    }
  }

  return (
    <div className="space-y-8 animate-in fade-in duration-300">
      {/* Organization Identity Section */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Building2 className="h-4 w-4 text-muted-foreground/70" />
          <h2 className="text-sm font-medium tracking-tight text-foreground">
            {t('settings.organization_identity')}
          </h2>
        </div>

        <div className="overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs divide-y divide-border/40">
          {/* Org Name Row */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <label htmlFor="org-name" className="text-xs font-medium text-foreground cursor-pointer">
                {t('settings.organization_name')}
              </label>
              <p className="text-[11px] font-light text-muted-foreground/70">
                {t('settings.identity_desc')}
              </p>
            </div>
            <div className="w-full sm:w-64">
              <Input
                id="org-name"
                value={localName}
                onChange={(e) => setLocalName(e.target.value)}
                className="h-[30px] rounded-md border-border/60 bg-background text-xs px-2.5 transition-colors focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>
          </div>

          {/* Org Logo Row */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.workspace_logo')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                PNG, SVG eller JPG som visas i sidomenyn och rapporter
              </p>
            </div>
            <div className="flex items-center gap-2.5">
              <input
                type="file"
                ref={fileInputRef}
                onChange={handleFileSelect}
                accept="image/*"
                className="hidden"
              />
              {localLogo ? (
                <div className="flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-md border border-border/50 bg-background/50 p-0.5">
                  <img src={localLogo} alt="Logo" className="h-full w-full object-contain" />
                </div>
              ) : (
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border/40 bg-secondary/30 text-muted-foreground/50">
                  <Building2 className="h-4 w-4" />
                </div>
              )}
              <Button
                type="button"
                variant="outline"
                size="default"
                onClick={() => fileInputRef.current?.click()}
                disabled={isUploading}
                className="h-[30px] gap-1.5 rounded-md px-2.5 text-xs font-normal"
              >
                {isUploading ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Upload className="h-3.5 w-3.5" />
                )}
                <span>{t('settings.upload_new_logo')}</span>
              </Button>
              {localLogo && (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleUpdateLogo(null)}
                  className="h-[30px] w-[30px] rounded-md text-muted-foreground/60 hover:text-destructive hover:bg-destructive/10"
                >
                  <X className="h-3.5 w-3.5" />
                </Button>
              )}
            </div>
          </div>

          {/* Brand Color Row */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.workspace.brand_color')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                {t('settings.workspace.brand_color_desc')}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={localBrandColor}
                onChange={(e) => setLocalBrandColor(e.target.value)}
                className="h-7 w-7 cursor-pointer rounded border border-border/50 bg-transparent p-0"
              />
              <span className="font-mono text-xs text-muted-foreground uppercase">
                {localBrandColor}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Localization & Region Section */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Globe className="h-4 w-4 text-muted-foreground/70" />
          <h2 className="text-sm font-medium tracking-tight text-foreground">
            {t('settings.localization_time')}
          </h2>
        </div>

        <div className="overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs divide-y divide-border/40">
          {/* Language Row */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.language')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Plattformens primära gränssnittsspråk
              </p>
            </div>
            <div className="w-full sm:w-52">
              <Select
                value={localLanguage}
                onValueChange={(val) => setLocalLanguage(val)}
              >
                <SelectTrigger className="h-[30px] w-full rounded-md border-border/60 bg-background text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="rounded-lg border-border/50 text-xs">
                  <SelectItem value="sv" className="text-xs">Svenska</SelectItem>
                  <SelectItem value="en" className="text-xs">English</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Timezone Row */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.timezone')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Används för tidsrapporter, pass och schemaläggning
              </p>
            </div>
            <div className="w-full sm:w-52">
              <Select
                value={localTimezone}
                onValueChange={(val) => setLocalTimezone(val)}
              >
                <SelectTrigger className="h-[30px] w-full rounded-md border-border/60 bg-background text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="rounded-lg border-border/50 text-xs">
                  <SelectItem value="Europe/Stockholm" className="text-xs">
                    Europe/Stockholm (GMT+1)
                  </SelectItem>
                  <SelectItem value="UTC" className="text-xs">
                    UTC / GMT
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Week Start Row */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 px-4 py-3">
            <div className="space-y-0.5">
              <span className="text-xs font-medium text-foreground">
                {t('settings.week_start')}
              </span>
              <p className="text-[11px] font-light text-muted-foreground/70">
                Första dagen i veckovyn och schemakalendern
              </p>
            </div>
            <div className="w-full sm:w-52">
              <Select
                value={localWeekStart.toString()}
                onValueChange={(val) => setLocalWeekStart(parseInt(val))}
              >
                <SelectTrigger className="h-[30px] w-full rounded-md border-border/60 bg-background text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="rounded-lg border-border/50 text-xs">
                  <SelectItem value="1" className="text-xs">{t('settings.monday')}</SelectItem>
                  <SelectItem value="0" className="text-xs">{t('settings.sunday')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>
      </div>

      {selectedImage && (
        <ImageCropperDialog
          image={selectedImage}
          open={cropperOpen}
          onClose={() => {
            setCropperOpen(false)
            setSelectedImage(null)
          }}
          onCropComplete={handleCropComplete}
        />
      )}

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

export default GeneralSettings
