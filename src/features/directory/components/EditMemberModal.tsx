import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { supabase } from '@/lib/supabase'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { toast } from 'sonner'
import { User, Check } from 'lucide-react'
import type { MemberItem } from '../hooks/useDirectoryData'

interface EditMemberModalProps {
  member: MemberItem | null
  isOpen: boolean
  onClose: () => void
  onSave: () => void
}

export const EditMemberModal: React.FC<EditMemberModalProps> = ({
  member,
  isOpen,
  onClose,
  onSave,
}) => {
  const { t } = useTranslation()
  const [loading, setLoading] = useState(false)

  // Form states
  const [name, setName] = useState('')
  const [phone, setPhone] = useState('')
  const [location, setLocation] = useState('')

  // Patient specific states
  const [firstName, setFirstName] = useState('')
  const [lastName, setLastName] = useState('')
  const [ssn, setSsn] = useState('')
  const [address, setAddress] = useState('')
  const [notes, setNotes] = useState('')

  useEffect(() => {
    if (member) {
      if (member.role === 'Patient') {
        const parts = member.name.split(' ')
        setFirstName(parts[0] || '')
        setLastName(parts.slice(1).join(' ') || '')
        setSsn(member.ssn || '')
        setAddress(member.address || '')
        setNotes(member.notes || '')
      } else {
        setName(member.name || '')
        setPhone(member.phone || '')
        setLocation(member.location || '')
      }
    }
  }, [member])

  const handleSave = async () => {
    if (!member) return
    setLoading(true)

    try {
      if (member.role === 'Patient') {
        const { error } = await supabase
          .from('clients')
          .update({
            first_name: firstName,
            last_name: lastName,
            personal_number: ssn,
            address,
            notes,
          })
          .eq('id', member.id)
        if (error) throw error
      } else {
        const { error } = await supabase
          .from('users')
          .update({
            full_name: name,
            phone,
            location,
          })
          .eq('id', member.id)
        if (error) throw error
      }

      toast.success(t('settings.success_update'))
      onSave()
      onClose()
    } catch (error: unknown) {
      toast.error(error instanceof Error ? error.message : t('directory.members.delete_error'))
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-md">
        <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <User className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>
              {t('directory.detail.edit_button', {
                type:
                  member?.role === 'Patient'
                    ? t('directory.detail.patient')
                    : t('directory.detail.staff'),
              })}
            </span>
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-3 p-5">
          {member?.role === 'Patient' ? (
            <>
              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-1">
                  <Label className="text-[11px] font-medium text-muted-foreground">
                    {t('common.first_name')}
                  </Label>
                  <Input
                    value={firstName}
                    onChange={(e) => setFirstName(e.target.value)}
                    className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                  />
                </div>
                <div className="space-y-1">
                  <Label className="text-[11px] font-medium text-muted-foreground">
                    {t('common.last_name')}
                  </Label>
                  <Input
                    value={lastName}
                    onChange={(e) => setLastName(e.target.value)}
                    className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                  />
                </div>
              </div>
              <div className="space-y-1">
                <Label className="text-[11px] font-medium text-muted-foreground">{t('common.ssn')}</Label>
                <Input
                  value={ssn}
                  onChange={(e) => setSsn(e.target.value)}
                  placeholder={t('directory.client_manager.ssn_placeholder')}
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
              <div className="space-y-1">
                <Label className="text-[11px] font-medium text-muted-foreground">{t('common.address')}</Label>
                <Input
                  value={address}
                  onChange={(e) => setAddress(e.target.value)}
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
              <div className="space-y-1">
                <Label className="text-[11px] font-medium text-muted-foreground">{t('common.notes')}</Label>
                <Textarea
                  value={notes}
                  onChange={(e) => setNotes(e.target.value)}
                  rows={3}
                  className="resize-none rounded-md border border-border bg-background px-2.5 py-2 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
            </>
          ) : (
            <>
              <div className="space-y-1">
                <Label className="text-[11px] font-medium text-muted-foreground">{t('common.name')}</Label>
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
              <div className="space-y-1">
                <Label className="text-[11px] font-medium text-muted-foreground">{t('settings.account.phone')}</Label>
                <Input
                  value={phone}
                  onChange={(e) => setPhone(e.target.value)}
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
              <div className="space-y-1">
                <Label className="text-[11px] font-medium text-muted-foreground">{t('settings.account.location')}</Label>
                <Input
                  value={location}
                  onChange={(e) => setLocation(e.target.value)}
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
            </>
          )}
        </div>
        <DialogFooter className="flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
          <Button
            variant="outline"
            size="sm"
            onClick={onClose}
            className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
          >
            {t('common.cancel')}
          </Button>
          <Button
            size="sm"
            onClick={handleSave}
            disabled={loading}
            className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
          >
            {loading ? (
              t('common.saving')
            ) : (
              <>
                <Check className="mr-1.5 size-3" />
                <span>{t('common.save')}</span>
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
