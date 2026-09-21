import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router-dom'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { supabase } from '@/lib/supabase'
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
import { Checkbox } from '@/components/ui/checkbox'
import { toast } from 'sonner'
import { Loader2 } from 'lucide-react'

interface ReportFormProps {
  onSuccess: () => void
}

export const ReportForm: React.FC<ReportFormProps> = ({ onSuccess }) => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const { workspaceId } = useWorkspace()
  const queryClient = useQueryClient()

  const [reportType, setReportType] = useState('complaint')
  const [subject, setSubject] = useState('')
  const [description, setDescription] = useState('')
  const [dateOfIncident, setDateOfIncident] = useState(new Date().toISOString().split('T')[0])
  const [isAnonymous, setIsAnonymous] = useState(false)
  const [searchParams, setSearchParams] = useSearchParams()

  React.useEffect(() => {
    const type = searchParams.get('type')
    if (type && ['complaint', 'work_injury', 'incident', 'deviation', 'whistleblower'].includes(type)) {
      setReportType(type)
      const newParams = new URLSearchParams(searchParams)
      newParams.delete('type')
      setSearchParams(newParams, { replace: true })
    }
  }, [searchParams, setSearchParams])

  const mutation = useMutation({
    mutationFn: async () => {
      if (!workspaceId || !user) throw new Error('Missing workspace or user context')

      const { error } = await supabase.from('reports').insert({
        workspace_id: workspaceId,
        user_id: user.id,
        type: reportType,
        is_anonymous: isAnonymous,
        content: {
          subject,
          description,
          date_of_incident: dateOfIncident,
        },
        status: 'pending',
      })

      if (error) throw error

      // Attempt to notify admins
      try {
        const { data: admins } = await supabase
          .from('users')
          .select('id')
          .eq('workspace_id', workspaceId)
          .in('role', ['admin', 'platform_admin'])

        if (admins && admins.length > 0) {
          const messagePayloads = admins.map(admin => ({
            workspace_id: workspaceId,
            sender_id: user.id,
            receiver_id: admin.id,
            subject: t('reporting.notifications.admin_subject', { type: t(`reporting.types.${reportType}`) }),
            body: t('reporting.notifications.admin_body', {
              type: t(`reporting.types.${reportType}`),
              subject
            }),
            is_read: false
          }))

          await supabase.from('messages').insert(messagePayloads)
        }
      } catch (e) {
        console.warn('Admin notification failed, but report was saved:', e)
      }
    },
    onSuccess: () => {
      toast.success(t('reporting.form.success'))
      setSubject('')
      setDescription('')
      queryClient.invalidateQueries({ queryKey: ['reports'] })
      onSuccess()
    },
    onError: (error) => {
      console.error('Report submission failed:', error)
      toast.error(t('reporting.form.error'))
    }
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    mutation.mutate()
  }

  return (
    <form onSubmit={handleSubmit} className="relative space-y-4">
      <div className="grid gap-3 md:grid-cols-2">
        <div className="space-y-1.5">
          <Label htmlFor="report-type" className="text-sm font-medium text-foreground/80">
            {t('reporting.list.type')}
          </Label>
          <Select value={reportType} onValueChange={setReportType}>
            <SelectTrigger id="report-type" className="h-10 rounded-md border-border/60 bg-background/50 transition-colors focus:ring-2 focus:ring-primary/20">
              <SelectValue placeholder={t('reporting.form.select_type')} />
            </SelectTrigger>
            <SelectContent className="rounded-md border-border/50 bg-popover">
              <SelectItem value="complaint">{t('reporting.types.complaint')}</SelectItem>
              <SelectItem value="work_injury">{t('reporting.types.work_injury')}</SelectItem>
              <SelectItem value="incident">{t('reporting.types.incident')}</SelectItem>
              <SelectItem value="deviation">{t('reporting.types.deviation')}</SelectItem>
              <SelectItem value="whistleblower">{t('reporting.types.whistleblower')}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="date" className="text-xs font-medium text-foreground/80">
            {t('reporting.form.date_of_incident')}
          </Label>
          <Input
            id="date"
            type="date"
            value={dateOfIncident}
            onChange={(e) => setDateOfIncident(e.target.value)}
            className="h-8 rounded-md border-border/60 bg-background/50 text-xs transition-colors focus:ring-2 focus:ring-primary/20 [color-scheme:dark]"
            required
          />
        </div>
      </div>

      <div className="space-y-1.5">
        <Label htmlFor="subject" className="text-xs font-medium text-foreground/80">
          {t('reporting.form.subject')}
        </Label>
        <Input
          id="subject"
          placeholder={t('reporting.form.subject_placeholder')}
          value={subject}
          onChange={(e) => setSubject(e.target.value)}
          className="h-8 rounded-md border-border/60 bg-background/50 text-xs transition-colors focus:ring-2 focus:ring-primary/20"
          required
        />
      </div>

      <div className="space-y-1.5">
        <Label htmlFor="description" className="text-xs font-medium text-foreground/80">
          {t('reporting.form.description')}
        </Label>
        <Textarea
          id="description"
          placeholder={t('reporting.form.description_placeholder')}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          className="min-h-[100px] rounded-md border-border/60 bg-background/50 text-xs transition-colors focus:ring-2 focus:ring-primary/20"
          required
        />
      </div>

      <div className="group flex items-center space-x-3 rounded-md border border-border/40 bg-muted/30 p-2.5 transition-colors hover:bg-muted/50">
        <Checkbox
          id="anonymous"
          checked={isAnonymous}
          onCheckedChange={(checked) => setIsAnonymous(checked === true)}
          className="h-4 w-4 rounded-sm"
        />
        <div className="grid gap-0.5 leading-none">
          <Label
            htmlFor="anonymous"
            className="text-xs font-medium leading-none text-foreground/90 transition-colors cursor-pointer"
          >
            {t('reporting.form.is_anonymous')}
          </Label>
          <p className="text-[11px] text-muted-foreground/80">
            {t('reporting.form.anonymous_hint')}
          </p>
        </div>
      </div>

      <Button 
        type="submit" 
        className="h-8 w-full md:w-auto px-4 rounded-md bg-primary text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90 transition-colors disabled:opacity-50" 
        disabled={mutation.isPending}
      >
        {mutation.isPending ? (
          <>
            <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
            {t('common.saving')}
          </>
        ) : (
          <span>
            {t('reporting.form.submit')}
          </span>
        )}
      </Button>
    </form>
  )
}
