import type React from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { supabase } from '@/lib/supabase'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { School, HeartPulse, Building2, Plus } from 'lucide-react'
import { cn } from '@/lib/utils'

interface DevHubModalProps {
  isOpen: boolean
  onClose: () => void
}

export const DevHubModal: React.FC<DevHubModalProps> = ({ isOpen, onClose }) => {
  const { t } = useTranslation()
  const [hubWsName, setHubWsName] = useState('')
  const [hubAdminEmail, setHubAdminEmail] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [selectedModules, setSelectedModules] = useState({
    school: false,
    assistance: false,
  })

  const handleModuleToggle = (moduleKey: 'school' | 'assistance', checked: boolean) => {
    if (checked) {
      const otherKey = moduleKey === 'school' ? 'assistance' : 'school'
      setSelectedModules({ [moduleKey]: true, [otherKey]: false } as typeof selectedModules)
    } else {
      setSelectedModules((prev) => ({ ...prev, [moduleKey]: false }))
    }
  }

  const handleCreate = async () => {
    setIsLoading(true)
    const { data, error } = await supabase.functions.invoke('invite_user', {
      body: {
        newWorkspaceName: hubWsName,
        email: hubAdminEmail,
        role: 'admin',
        modules: selectedModules,
      },
    })
    setIsLoading(false)
    if (error) alert(t('directory.members.delete_error') + ' ' + error.message)
    else if (data && data.success === false)
      alert(t('directory.members.delete_error') + ' ' + data.error)
    else {
      alert(t('directory.hub.success'))
      onClose()
      setHubWsName('')
      setHubAdminEmail('')
      setSelectedModules({ school: false, assistance: false })
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-[450px]">
        <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <Building2 className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>{t('directory.hub.title')}</span>
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4 p-5">
          <div className="space-y-3">
            <div className="space-y-1">
              <Label htmlFor="wsName" className="text-[11px] font-medium text-muted-foreground">
                {t('directory.hub.ws_name_label')}
              </Label>
              <Input
                id="wsName"
                value={hubWsName}
                onChange={(e) => setHubWsName(e.target.value)}
                placeholder={t('directory.hub.ws_name_placeholder')}
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="adminEmail" className="text-[11px] font-medium text-muted-foreground">
                {t('directory.hub.admin_email_label')}
              </Label>
              <Input
                id="adminEmail"
                type="email"
                value={hubAdminEmail}
                onChange={(e) => setHubAdminEmail(e.target.value)}
                placeholder={t('directory.hub.admin_email_placeholder')}
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label className="block text-[11px] font-medium text-muted-foreground">{t('settings.tabs.modules')}</Label>
            <div className="grid grid-cols-2 gap-2.5">
              {/* School Module */}
              <div
                onClick={() => handleModuleToggle('school', !selectedModules.school)}
                className={cn(
                  'flex cursor-pointer flex-col gap-2 rounded-md border p-2.5 transition-all',
                  selectedModules.school
                    ? 'border-primary bg-secondary/60'
                    : 'border-border bg-secondary/20 hover:bg-secondary/30',
                )}
              >
                <div className="flex items-center justify-between">
                  <div
                    className={cn(
                      'flex size-6 items-center justify-center rounded-md transition-colors',
                      selectedModules.school
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-secondary text-muted-foreground',
                    )}
                  >
                    <School className="size-3.5" />
                  </div>
                  <Switch
                    checked={selectedModules.school}
                    onCheckedChange={(c) => handleModuleToggle('school', c)}
                  />
                </div>
                <div>
                  <span className="block text-xs font-medium text-foreground">
                    {t('settings.modules.school_title')}
                  </span>
                  <span className="line-clamp-1 text-[10px] text-muted-foreground">
                    {t('settings.modules.school_desc')}
                  </span>
                </div>
              </div>

              {/* Assistance Module */}
              <div
                onClick={() => handleModuleToggle('assistance', !selectedModules.assistance)}
                className={cn(
                  'flex cursor-pointer flex-col gap-2 rounded-md border p-2.5 transition-all',
                  selectedModules.assistance
                    ? 'border-primary bg-secondary/60'
                    : 'border-border bg-secondary/20 hover:bg-secondary/30',
                )}
              >
                <div className="flex items-center justify-between">
                  <div
                    className={cn(
                      'flex size-6 items-center justify-center rounded-md transition-colors',
                      selectedModules.assistance
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-secondary text-muted-foreground',
                    )}
                  >
                    <HeartPulse className="size-3.5" />
                  </div>
                  <Switch
                    checked={selectedModules.assistance}
                    onCheckedChange={(c) => handleModuleToggle('assistance', c)}
                  />
                </div>
                <div>
                  <span className="block text-xs font-medium text-foreground">
                    {t('settings.modules.assistance_title')}
                  </span>
                  <span className="line-clamp-1 text-[10px] text-muted-foreground">
                    {t('settings.modules.assistance_desc')}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
        <DialogFooter className="flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
          <Button
            variant="outline"
            size="sm"
            onClick={onClose}
            className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
          >
            {t('directory.hub.cancel')}
          </Button>
          <Button
            size="sm"
            onClick={handleCreate}
            disabled={isLoading || !hubWsName || !hubAdminEmail}
            className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
          >
            {isLoading ? (
              t('directory.hub.creating')
            ) : (
              <>
                <Plus className="mr-1.5 size-3" />
                <span>{t('directory.hub.create_invite')}</span>
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
