import React, { useState, useEffect } from 'react'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { supabase } from '@/lib/supabase'
import { useTranslation } from 'react-i18next'
import { User, FileText, Pill } from 'lucide-react'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import { PageSkeleton } from '@/components/layout/PageSkeleton'
import { JournalTab } from './JournalTab'
import { MedicationTab } from './MedicationTab'

interface Client {
  id: string
  first_name: string
  last_name: string
  personal_number?: string
  care_level?: string
}

export const ClientOverviewPage: React.FC = () => {
  const { selectedTeamId: activeTeam } = useWorkspace()
  const { t } = useTranslation()
  const [client, setClient] = useState<Client | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<'journal' | 'medication'>('journal')

  useEffect(() => {
    const fetchClient = async () => {
      if (!activeTeam) {
        setClient(null)
        setIsLoading(false)
        return
      }
      setIsLoading(true)
      try {
        const { data } = await supabase
          .from('clients')
          .select('*')
          .eq('team_id', activeTeam)
          .single()

        if (data) setClient(data)
        else setClient(null)
      } catch (e) {
        console.error(e)
      } finally {
        setIsLoading(false)
      }
    }
    fetchClient()
  }, [activeTeam])

  if (isLoading) {
    return <PageSkeleton cardsCount={2} rowsCount={3} />
  }

  if (!client) {
    return (
      <div className="mx-auto flex h-full max-w-4xl flex-col items-center justify-center p-8 text-center">
        <User className="mb-4 h-16 w-16 text-muted-foreground/30" />
        <h2 className="mb-2 text-xl font-bold">
          {t('assistance.no_client_linked')}
        </h2>
        <p className="mx-auto max-w-md text-muted-foreground">
          {t('assistance.no_client_description')}
        </p>
      </div>
    )
  }

  return (
    <div className="mx-auto h-full w-full max-w-5xl flex-1 overflow-y-auto p-4 md:p-8">
      <div className="mb-6 flex items-start gap-5 rounded-lg border border-border bg-card p-5 shadow-sm">
        <div className="flex h-16 w-16 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-primary/10 text-2xl font-bold text-primary shadow-inner">
          {client.first_name.charAt(0)}
          {client.last_name.charAt(0)}
        </div>
        <div>
          <h1 className="text-xl font-bold tracking-tight text-foreground">
            {client.first_name} {client.last_name}
          </h1>
          <p className="mt-0.5 text-xs font-medium text-muted-foreground">
            {t('assistance.personal_number')}:{' '}
            {client.personal_number || t('common.unknown')}
          </p>
          <div className="mt-3 flex gap-2">
            <span className="rounded-md border border-border bg-muted px-2.5 py-0.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
              {t('assistance.care_level')}:{' '}
              {client.care_level
                ? t(`directory.client_manager.care_level_${client.care_level}`)
                : t('common.unknown')}
            </span>
          </div>
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={(val) => setActiveTab(val as 'journal' | 'medication')} className="w-full">
        <TabsList className="mb-6">
          <TabsTrigger value="journal" className="gap-2">
            <FileText className="h-4 w-4" />
            {t('assistance.daily_notes')}
          </TabsTrigger>
          <TabsTrigger value="medication" className="gap-2">
            <Pill className="h-4 w-4" />
            {t('assistance.active_medication_list')}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="journal">
          <JournalTab clientId={client.id} />
        </TabsContent>
        <TabsContent value="medication">
          <MedicationTab clientId={client.id} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
