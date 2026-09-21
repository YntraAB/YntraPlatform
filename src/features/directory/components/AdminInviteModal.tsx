import React, { useState } from 'react'
import { Loader2, ShieldCheck, Send } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { supabase } from '@/lib/supabase'
import { toast } from 'sonner'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

interface AdminInviteModalProps {
  isOpen: boolean
  onClose: () => void
  workspaceId: string | null
  selectedWorkspace: string | null
}

export const AdminInviteModal: React.FC<AdminInviteModalProps> = ({
  isOpen,
  onClose,
  workspaceId,
  selectedWorkspace,
}) => {
  const { t } = useTranslation()
  const [email, setEmail] = useState('')
  const [isLoading, setIsLoading] = useState(false)

  const handleInvite = async () => {
    const targetWS = selectedWorkspace || workspaceId
    if (!targetWS) return

    setIsLoading(true)
    try {
      const { data, error } = await supabase.functions.invoke('invite_user', {
        body: {
          email: email,
          role: 'admin',
          workspaceId: targetWS,
        },
      })

      if (error) throw error
      if (data && data.success === false) throw new Error(data.error)

      toast.success(t('directory.admin_invite.success'))
      onClose()
      setEmail('')
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : t('directory.members.delete_error')
      toast.error(message)
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-md">
        <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <ShieldCheck className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>{t('directory.admin_invite.title')}</span>
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-3 p-5">
          <p className="text-xs leading-relaxed text-muted-foreground">{t('directory.admin_invite.desc')}</p>
          <div className="space-y-1">
            <Label htmlFor="adminEmail" className="text-[11px] font-medium text-muted-foreground">
              {t('directory.invite.email_label')}
            </Label>
            <Input
              id="adminEmail"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder={t('directory.invite.email_placeholder')}
              className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
              autoFocus
            />
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
            onClick={handleInvite}
            disabled={isLoading || !email}
            className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
          >
            {isLoading ? (
              <Loader2 className="mr-1.5 size-3.5 animate-spin" />
            ) : (
              <Send className="mr-1.5 size-3" />
            )}
            <span>{t('directory.invite.send_button')}</span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
