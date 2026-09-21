import React, { useState } from 'react'
import { Trash2, Loader2, Users, ChevronRight, ChevronLeft, Plus, Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { supabase } from '@/lib/supabase'
import { toast } from 'sonner'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { cn } from '@/lib/utils'

interface TeamManagerModalProps {
  isOpen: boolean
  onClose: () => void
  workspaceId: string | null
  selectedWorkspace: string | null
}

interface ClientForm {
  id?: string
  firstName: string
  lastName: string
  personalNumber: string
  messageSetting: string
  contactPersonEmail: string
}

export const TeamManagerModal: React.FC<TeamManagerModalProps> = ({
  isOpen,
  onClose,
  workspaceId,
  selectedWorkspace,
}) => {
  const { t } = useTranslation()
  const [step, setStep] = useState(1)
  const [isSubmitting, setIsSubmitting] = useState(false)

  // Team State
  const [newTeamName, setNewTeamName] = useState('')

  // Client State
  const [clients, setClients] = useState<ClientForm[]>([])
  const [currentClient, setCurrentClient] = useState({
    firstName: '',
    lastName: '',
    personalNumber: '',
    messageSetting: 'contact_person',
    contactPersonEmail: '',
  })

  const resetModal = () => {
    setStep(1)
    setNewTeamName('')
    setClients([])
    setCurrentClient({
      firstName: '',
      lastName: '',
      personalNumber: '',
      messageSetting: 'contact_person',
      contactPersonEmail: '',
    })
  }

  const handleAddClientToList = () => {
    if (!currentClient.firstName || !currentClient.lastName) {
      toast.error(t('common.fill_required_fields'))
      return
    }
    setClients([...clients, { ...currentClient, id: crypto.randomUUID() }])
    setCurrentClient({
      firstName: '',
      lastName: '',
      personalNumber: '',
      messageSetting: 'contact_person',
      contactPersonEmail: '',
    })
  }

  const handleRemoveClient = (id: string) => {
    setClients(clients.filter((c) => c.id !== id))
  }

  const handleCreateAll = async () => {
    const targetWS = selectedWorkspace || workspaceId
    if (!targetWS) return

    setIsSubmitting(true)
    try {
      // 1. Create Team
      const { data: teamData, error: teamError } = await supabase
        .from('teams')
        .insert([{ workspace_id: targetWS, name: newTeamName }])
        .select()
        .single()

      if (teamError) throw teamError

      const teamId = teamData.id

      // 2. Prepare all clients to insert
      const clientsToInsert = [...clients]
      // Include current client if they started filling it out
      if (currentClient.firstName && currentClient.lastName) {
        clientsToInsert.push({ ...currentClient, id: crypto.randomUUID() })
      }

      if (clientsToInsert.length > 0) {
        const { error: clientsError } = await supabase.from('clients').insert(
          clientsToInsert.map((c) => ({
            workspace_id: targetWS,
            first_name: c.firstName,
            last_name: c.lastName,
            personal_number: c.personalNumber,
            team_id: teamId,
            message_settings: {
              allowed_contacts: c.messageSetting,
              contact_person_email: c.contactPersonEmail || null,
            },
          })),
        )

        if (clientsError) throw clientsError

        for (const c of clientsToInsert) {
          if (c.contactPersonEmail) {
            // Here we could invoke the same invite_user function if needed,
            // but following ClientManagerModal, it's just stored in message_settings.
            // If the user explicitly asked for "invited", we might want to call the function.
            await supabase.functions.invoke('invite_user', {
              body: {
                email: c.contactPersonEmail,
                role: 'user', // Clients might be 'user' or a specific 'client' role
                workspaceId: targetWS,
              },
            })
          }
        }
      }

      toast.success(t('directory.create_team.success'))
      onClose()
      resetModal()
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : t('directory.members.delete_error')
      toast.error(message)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        className={cn(
          'overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl transition-all duration-300',
          step === 1 ? 'sm:max-w-md' : 'sm:max-w-lg',
        )}
      >
        <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <Users className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>
              {step === 1
                ? t('directory.create_team.submit')
                : t('directory.client_manager.title')}
            </span>
          </DialogTitle>
          <span className="text-[11px] font-normal text-muted-foreground tabular-nums">
            {step} / 2
          </span>
        </DialogHeader>

        <div className="p-5">
          {step === 1 ? (
            <div className="space-y-4 duration-300 animate-in fade-in slide-in-from-right-4">
              <div className="space-y-1">
                <Label htmlFor="teamName" className="text-[11px] font-medium text-muted-foreground">
                  {t('directory.create_team.name_label')} *
                </Label>
                <Input
                  id="teamName"
                  value={newTeamName}
                  onChange={(e) => setNewTeamName(e.target.value)}
                  className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                  placeholder={t('directory.create_team.name_placeholder')}
                  autoFocus
                />
              </div>
              <div className="-mx-5 -mb-5 mt-5 flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
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
                  onClick={() => setStep(2)}
                  disabled={!newTeamName}
                  className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
                >
                  <span>{t('common.next')}</span>
                  <ChevronRight className="ml-1 size-3" />
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-4 duration-300 animate-in fade-in slide-in-from-left-4">
              {/* Added Clients List */}
              {clients.length > 0 && (
                <div className="space-y-1.5">
                  <Label className="text-[11px] font-medium text-muted-foreground">
                    {t('directory.members.team_title')} ({clients.length})
                  </Label>
                  <div className="grid max-h-32 gap-1.5 overflow-y-auto pr-1">
                    {clients.map((client) => (
                      <div
                        key={client.id}
                        className="flex items-center justify-between rounded-md border border-border bg-secondary/30 px-3 py-1.5"
                      >
                        <div className="flex flex-col">
                          <span className="text-xs font-medium text-foreground">
                            {client.firstName} {client.lastName}
                          </span>
                          <span className="text-[10px] text-muted-foreground">
                            {client.contactPersonEmail || t('directory.detail.private')}
                          </span>
                        </div>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => client.id && handleRemoveClient(client.id)}
                          className="size-6 rounded-md text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                        >
                          <Trash2 className="size-3" />
                        </Button>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Client Form */}
              <div className="space-y-3 rounded-md border border-border bg-secondary/20 p-3">
                <div className="flex items-center justify-between">
                  <h4 className="text-xs font-medium text-foreground">
                    {t('directory.client_manager.title')} (Valfritt)
                  </h4>
                  {currentClient.firstName && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleAddClientToList}
                      className="h-6 rounded-md border border-border bg-background px-2.5 text-[11px] font-medium text-foreground hover:bg-secondary"
                    >
                      <Plus className="mr-1 size-3" />
                      <span>{t('common.add')}</span>
                    </Button>
                  )}
                </div>

                <div className="grid grid-cols-2 gap-2.5">
                  <div className="space-y-1">
                    <Label className="text-[11px] font-medium text-muted-foreground">
                      {t('common.first_name')}
                    </Label>
                    <Input
                      value={currentClient.firstName}
                      onChange={(e) =>
                        setCurrentClient({ ...currentClient, firstName: e.target.value })
                      }
                      className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                    />
                  </div>
                  <div className="space-y-1">
                    <Label className="text-[11px] font-medium text-muted-foreground">{t('common.last_name')}</Label>
                    <Input
                      value={currentClient.lastName}
                      onChange={(e) =>
                        setCurrentClient({ ...currentClient, lastName: e.target.value })
                      }
                      className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                    />
                  </div>
                </div>

                <div className="space-y-1">
                  <Label className="text-[11px] font-medium text-muted-foreground">
                    {t('directory.client_manager.contact_email_label')}
                  </Label>
                  <Input
                    type="email"
                    value={currentClient.contactPersonEmail}
                    onChange={(e) =>
                      setCurrentClient({ ...currentClient, contactPersonEmail: e.target.value })
                    }
                    placeholder="E-post för inbjudan..."
                    className="h-8 rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus-visible:ring-1 focus-visible:ring-ring"
                  />
                </div>

                <div className="space-y-1">
                  <Label className="text-[11px] font-medium text-muted-foreground">
                    {t('directory.client_manager.comm_level_label')}
                  </Label>
                  <Select
                    value={currentClient.messageSetting}
                    onValueChange={(val) =>
                      setCurrentClient({ ...currentClient, messageSetting: val })
                    }
                  >
                    <SelectTrigger className="h-8 w-full rounded-md border border-border bg-background px-2.5 text-xs text-foreground focus:ring-1 focus:ring-ring">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent className="rounded-md border border-border bg-popover text-xs shadow-lg">
                      <SelectItem value="admin_only" className="text-xs">
                        {t('directory.client_manager.comm_admin_only')}
                      </SelectItem>
                      <SelectItem value="contact_person" className="text-xs">
                        {t('directory.client_manager.comm_contact')}
                      </SelectItem>
                      <SelectItem value="open" className="text-xs">
                        {t('directory.client_manager.comm_open')}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="-mx-5 -mb-5 mt-5 flex items-center justify-between border-t border-border bg-secondary/40 px-5 py-2.5">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setStep(1)}
                  className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
                >
                  <ChevronLeft className="mr-1 size-3" />
                  <span>{t('common.back')}</span>
                </Button>
                <div className="flex items-center gap-2">
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
                    onClick={handleCreateAll}
                    disabled={isSubmitting}
                    className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
                  >
                    {isSubmitting ? (
                      <Loader2 className="mr-1.5 size-3.5 animate-spin" />
                    ) : (
                      <Check className="mr-1.5 size-3" />
                    )}
                    <span>{t('directory.create_team.submit')}</span>
                  </Button>
                </div>
              </div>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
