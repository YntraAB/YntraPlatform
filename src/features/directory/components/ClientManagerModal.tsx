import React, { useState } from 'react'
import { supabase } from '@/lib/supabase'
import { Button } from '@/components/ui/button'
import { Loader2, User, Check } from 'lucide-react'
import { toast } from 'sonner'
import { useWorkspaceUsers } from '@/hooks/queries/useWorkspaceData'
import { useTranslation } from 'react-i18next'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'

interface ClientManagerModalProps {
  isOpen: boolean
  onClose: () => void
  workspaceId: string | null
  teams: { id: string; name: string }[]
  initialData?: import('../hooks/useDirectoryData').ClientItem | null
}

export const ClientManagerModal: React.FC<ClientManagerModalProps> = ({
  isOpen,
  onClose,
  workspaceId,
  teams,
  initialData,
}) => {
  const { t } = useTranslation()
  const [firstName, setFirstName] = useState('')
  const [lastName, setLastName] = useState('')
  const [personalNumber, setPersonalNumber] = useState('')
  const [teamId, setTeamId] = useState('')
  const [messageSetting, setMessageSetting] = useState('contact_person')
  const [contactPersonEmail, setContactPersonEmail] = useState('')
  const [location, setLocation] = useState('')
  const [address, setAddress] = useState('')
  const [avatar, setAvatar] = useState('')
  const [careLevel, setCareLevel] = useState('medium')
  const [showSuggestions, setShowSuggestions] = useState(false)
  const [hasContactPerson, setHasContactPerson] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)

  React.useEffect(() => {
    if (initialData) {
      setFirstName(initialData.firstName)
      setLastName(initialData.lastName)
      setPersonalNumber(initialData.personalNumber)
      setTeamId(initialData.teamId || '')
      setLocation(initialData.location || '')
      setAddress(initialData.address || '')
      setAvatar(initialData.avatar || '')
      setCareLevel(initialData.careLevel || 'medium')
    } else {
      setFirstName('')
      setLastName('')
      setPersonalNumber('')
      setTeamId('')
      setLocation('')
      setAddress('')
      setAvatar('')
    }
  }, [initialData, isOpen])

  const { data: users = [] } = useWorkspaceUsers(workspaceId || null)

  const filteredUsers = React.useMemo(() => {
    if (!contactPersonEmail) return []
    const lower = contactPersonEmail.toLowerCase()
    return users
      .filter((u) => u.email.toLowerCase().includes(lower) || u.name.toLowerCase().includes(lower))
      .slice(0, 5)
  }, [contactPersonEmail, users])

  const handleSelectUser = (email: string) => {
    setContactPersonEmail(email)
    setShowSuggestions(false)
  }

  if (!isOpen) return null

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!workspaceId) return

    setIsSubmitting(true)
    try {
      const finalEmail =
        messageSetting === 'contact_person' || (messageSetting === 'open' && hasContactPerson)
          ? contactPersonEmail
          : null

      const { error } = await supabase.from('clients').upsert({
        id: initialData?.id || undefined,
        workspace_id: workspaceId,
        first_name: firstName,
        last_name: lastName,
        personal_number: personalNumber,
        team_id: teamId || null,
        location: location || null,
        address: address || null,
        avatar: avatar || null,
        care_level: careLevel,
        message_settings: {
          allowed_contacts: messageSetting,
          contact_person_email: finalEmail,
        },
      })

      if (error) throw error

      toast.success(t('directory.client_manager.success'))
      onClose()
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : t('directory.client_manager.error')
      toast.error(message)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-[425px]">
        <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <User className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>
              {initialData
                ? t('directory.client_manager.edit_title', 'Redigera Brukare')
                : t('directory.client_manager.title')}
            </span>
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="p-5">
          <div className="max-h-[68vh] space-y-3 overflow-y-auto pr-1">
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1">
                <label className="block text-[11px] font-medium text-muted-foreground">
                  {t('common.first_name')} *
                </label>
                <input
                  required
                  value={firstName}
                  onChange={(e) => setFirstName(e.target.value)}
                  className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:border-ring"
                />
              </div>
              <div className="space-y-1">
                <label className="block text-[11px] font-medium text-muted-foreground">
                  {t('common.last_name')} *
                </label>
                <input
                  required
                  value={lastName}
                  onChange={(e) => setLastName(e.target.value)}
                  className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:border-ring"
                />
              </div>
            </div>
            <div className="space-y-1">
              <label className="block text-[11px] font-medium text-muted-foreground">
                {t('common.ssn')}
              </label>
              <input
                value={personalNumber}
                onChange={(e) => setPersonalNumber(e.target.value)}
                placeholder={t('directory.client_manager.ssn_placeholder')}
                className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:border-ring"
              />
            </div>
            <div className="space-y-1">
              <label className="block text-[11px] font-medium text-muted-foreground">
                {t('directory.client_manager.care_level_label')}
              </label>
              <Select value={careLevel} onValueChange={setCareLevel}>
                <SelectTrigger className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                  <SelectItem value="low" className="text-xs">{t('directory.client_manager.care_level_low')}</SelectItem>
                  <SelectItem value="medium" className="text-xs">
                    {t('directory.client_manager.care_level_medium')}
                  </SelectItem>
                  <SelectItem value="high" className="text-xs">
                    {t('directory.client_manager.care_level_high')}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1">
                <label className="block text-[11px] font-medium text-muted-foreground">
                  {t('directory.members.location_label')}
                </label>
                <input
                  value={location}
                  onChange={(e) => setLocation(e.target.value)}
                  placeholder={t('directory.members.location_placeholder')}
                  className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:border-ring"
                />
              </div>
              <div className="space-y-1">
                <label className="block text-[11px] font-medium text-muted-foreground">
                  {t('directory.members.address_label')}
                </label>
                <input
                  value={address}
                  onChange={(e) => setAddress(e.target.value)}
                  placeholder={t('directory.members.address_placeholder')}
                  className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:border-ring"
                />
              </div>
            </div>
            <div className="space-y-1">
              <label className="block text-[11px] font-medium text-muted-foreground">
                {t('directory.members.avatar_label')}
              </label>
              <input
                value={avatar}
                onChange={(e) => setAvatar(e.target.value)}
                placeholder="https://..."
                className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:border-ring"
              />
            </div>
            <div className="space-y-1">
              <label className="block text-[11px] font-medium text-muted-foreground">
                {t('directory.client_manager.team_label')}
              </label>
              <Select required value={teamId} onValueChange={setTeamId}>
                <SelectTrigger className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground">
                  <SelectValue placeholder={t('directory.client_manager.team_placeholder')} />
                </SelectTrigger>
                <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                  {teams.map((t) => (
                    <SelectItem key={t.id} value={t.id} className="text-xs">
                      {t.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-[10px] text-muted-foreground">
                {t('directory.client_manager.team_info')}
              </p>
            </div>
            <div className="space-y-1">
              <label className="block text-[11px] font-medium text-muted-foreground">
                {t('directory.client_manager.comm_level_label')}
              </label>
              <Select
                value={messageSetting}
                onValueChange={(val) => {
                  setMessageSetting(val)
                  if (val === 'admin_only') {
                    setHasContactPerson(false)
                    setContactPersonEmail('')
                  }
                }}
              >
                <SelectTrigger className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                  <SelectItem value="admin_only" className="text-xs">
                    {t('directory.client_manager.comm_admin_only')}
                  </SelectItem>
                  <SelectItem value="contact_person" className="text-xs">
                    {t('directory.client_manager.comm_contact')}
                  </SelectItem>
                  <SelectItem value="open" className="text-xs">{t('directory.client_manager.comm_open')}</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-[10px] text-muted-foreground">
                {t('directory.client_manager.comm_info')}
              </p>
            </div>

            {messageSetting === 'open' && (
              <div className="flex items-center gap-2 pt-1">
                <input
                  type="checkbox"
                  id="hasContactPersonCheck"
                  checked={hasContactPerson}
                  onChange={(e) => setHasContactPerson(e.target.checked)}
                  className="size-3.5 cursor-pointer rounded border-border"
                />
                <label
                  htmlFor="hasContactPersonCheck"
                  className="cursor-pointer text-xs text-foreground"
                >
                  {t('directory.client_manager.contact_person_label')}
                </label>
              </div>
            )}

            {(messageSetting === 'contact_person' ||
              (messageSetting === 'open' && hasContactPerson)) && (
                <div className="relative space-y-1 pt-1">
                  <label className="block text-[11px] font-medium text-muted-foreground">
                    {t('directory.client_manager.contact_email_label')}
                  </label>
                  <input
                    value={contactPersonEmail}
                    onChange={(e) => {
                      setContactPersonEmail(e.target.value)
                      setShowSuggestions(true)
                    }}
                    onFocus={() => setShowSuggestions(true)}
                    onBlur={() => setTimeout(() => setShowSuggestions(false), 200)}
                    placeholder={t('directory.client_manager.contact_email_placeholder')}
                    className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground outline-none focus:border-ring"
                  />
                  {showSuggestions && filteredUsers.length > 0 && (
                    <div className="absolute left-0 right-0 top-full z-10 mt-1 max-h-40 overflow-y-auto rounded-md border border-border bg-popover shadow-lg">
                      {filteredUsers.map((u) => (
                        <div
                          key={u.id}
                          onClick={() => handleSelectUser(u.email)}
                          className="flex cursor-pointer flex-col px-2.5 py-1.5 text-xs text-foreground hover:bg-secondary"
                        >
                          <span className="font-medium">{u.name}</span>
                          <span className="text-[10px] text-muted-foreground">{u.email}</span>
                        </div>
                      ))}
                    </div>
                  )}
                  <p className="text-[10px] text-muted-foreground">
                    {t('directory.client_manager.contact_info')}
                  </p>
                </div>
              )}
          </div>

          <div className="-mx-5 -mb-5 mt-5 flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
            <Button
              type="button"
              variant="ghost"
              onClick={onClose}
              className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
            >
              {t('common.cancel')}
            </Button>
            <Button
              type="submit"
              disabled={isSubmitting}
              className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
            >
              {isSubmitting ? (
                <Loader2 className="mr-1.5 size-3.5 animate-spin" />
              ) : (
                <Check className="mr-1.5 size-3" />
              )}
              <span>{t('directory.client_manager.save_button')}</span>
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
