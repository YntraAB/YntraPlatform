import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '@/hooks/useAuth'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Loader2, Calendar, Send } from 'lucide-react'
import { toast } from 'sonner'
import { useCreateReport } from '@/hooks/queries/useReporting'
import { userService } from '@/services/userService'
import { messageService, type SendMessagePayload } from '@/services/messageService'
import { formatDateInput } from '@/lib/utils'

interface TimeOffRequestModalProps {
  isOpen: boolean
  onClose: () => void
}

export const TimeOffRequestModal: React.FC<TimeOffRequestModalProps> = ({ isOpen, onClose }) => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const { workspaceId } = useWorkspace()
  
  const [leaveType, setLeaveType] = useState('vacation')
  const [startDate, setStartDate] = useState(formatDateInput(new Date()))
  const [endDate, setEndDate] = useState(formatDateInput(new Date()))
  const [reason, setReason] = useState('')

  const createReportMutation = useCreateReport()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!workspaceId || !user) return

    try {
      await createReportMutation.mutateAsync({
        workspace_id: workspaceId,
        user_id: user.id,
        type: 'leave_request',
        is_anonymous: false,
        content: {
          leave_type: leaveType,
          start_date: startDate,
          end_date: endDate,
          description: reason,
          subject: `${t(`reporting.leave_types.${leaveType}`)}: ${startDate} - ${endDate}`,
        },
        status: 'pending',
      })

      try {
        const admins = await userService.getWorkspaceAdmins(workspaceId)

        if (admins && admins.length > 0) {
          const messagePayloads: SendMessagePayload[] = admins.map(admin => ({
            workspace_id: workspaceId,
            sender_id: user.id,
            receiver_id: admin.id,
            subject: t('reporting.notifications.admin_subject', { type: t('reporting.types.leave_request') }),
            body: t('reporting.notifications.admin_body', {
              type: t('reporting.types.leave_request'),
              subject: `${t(`reporting.leave_types.${leaveType}`)} (${startDate} till ${endDate})`
            }),
            is_read: false
          }))

          await messageService.sendMessages(messagePayloads)
        }
      } catch (msgError) {
        console.error('Failed to notify admins:', msgError)
      }

      toast.success(t('reporting.form.success'))
      onClose()
    } catch (error: any) {
      console.error('Error sending leave request:', error)
      toast.error(t('reporting.form.error'))
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-md">
        <DialogHeader className="flex h-12 flex-row items-center justify-between border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <Calendar className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>{t('reporting.types.leave_request')}</span>
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 p-5">
          <div className="space-y-1">
            <Label htmlFor="leave-type" className="text-[11px] font-medium text-muted-foreground">
              {t('reporting.list.type')}
            </Label>
            <Select value={leaveType} onValueChange={setLeaveType}>
              <SelectTrigger id="leave-type" className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                <SelectItem value="vacation" className="text-xs">{t('reporting.leave_types.vacation')}</SelectItem>
                <SelectItem value="sick_leave" className="text-xs">{t('reporting.leave_types.sick_leave')}</SelectItem>
                <SelectItem value="care_of_child" className="text-xs">{t('reporting.leave_types.care_of_child')}</SelectItem>
                <SelectItem value="other" className="text-xs">{t('reporting.leave_types.other')}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <Label htmlFor="start-date" className="text-[11px] font-medium text-muted-foreground">
                {t('common.start_date')}
              </Label>
              <Input
                id="start-date"
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
                required
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="end-date" className="text-[11px] font-medium text-muted-foreground">
                {t('common.end_date')}
              </Label>
              <Input
                id="end-date"
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
                className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground"
                required
              />
            </div>
          </div>

          <div className="space-y-1">
            <Label htmlFor="reason" className="text-[11px] font-medium text-muted-foreground">
              {t('reporting.form.description')}
            </Label>
            <Textarea
              id="reason"
              placeholder={t('scheduler.description_placeholder')}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              className="min-h-[80px] resize-none rounded-md border border-border bg-background px-2.5 py-2 text-xs text-foreground outline-none focus:border-ring"
            />
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
              disabled={createReportMutation.isPending}
              className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
            >
              {createReportMutation.isPending ? (
                <>
                  <Loader2 className="mr-1.5 size-3.5 animate-spin" />
                  <span>{t('common.saving')}</span>
                </>
              ) : (
                <>
                  <Send className="mr-1.5 size-3" />
                  <span>{t('reporting.form.submit')}</span>
                </>
              )}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
