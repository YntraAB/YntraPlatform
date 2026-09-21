import React, { useCallback, useMemo } from 'react'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'
import { useTranslation } from 'react-i18next'
import { useSetDynamicBreadcrumbs } from '@/contexts/BreadcrumbContext'
import {
  useMessagesPaginated,
  useMarkMessageAsRead,
  useSendMessage,
  useDeleteMessages,
} from '@/hooks/queries/useMessages'
import { useWorkspaceTeams, useWorkspaceUsers } from '@/hooks/queries/useWorkspaceData'
import { toast } from 'sonner'
import { useQueryClient } from '@tanstack/react-query'
import { queryKeys } from '@/lib/query-keys'

import { MessagesListPane } from './MessagesListPane'
import { MessageReadPane } from './MessageReadPane'
import { MessageComposePane } from './MessageComposePane'
import { useMessagesState } from '../hooks/useMessagesState'
import { transformMessages } from '../utils/messageTransformers'
import { useSearchParams } from 'react-router-dom'
import type { Message, Client } from '@/types'
import { supabase } from '@/lib/supabase'
import type { FetchMessagesParams } from '@/services/messageService'

interface MessagePayload {
  workspace_id: string
  sender_id: string
  receiver_id?: string
  target_team_id?: string
  subject: string
  body: string
  is_read: false
}

export const MessagesPage: React.FC = () => {
  const { workspaceId } = useWorkspace()
  const { user } = useAuth()
  const { t } = useTranslation()
  const [searchParams] = useSearchParams()

  const queryClient = useQueryClient()
  const deleteTimeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null)

  const { data: teams = [] } = useWorkspaceTeams(workspaceId || null)
  const { data: users = [] } = useWorkspaceUsers(workspaceId || null)

  const {
    filterType,
    setFilterType,
    activeMessageId,
    setActiveMessageId,
    searchQuery,
    setSearchQuery,
    currentPage,
    setCurrentPage,
    selectedMsgs,
    setSelectedMsgs,
    isComposing,
    setIsComposing,
    composeData,
    setComposeData,
  } = useMessagesState([])

  const { selectedTeamId } = useWorkspace()
  const itemsPerPage = 10
  const from = (currentPage - 1) * itemsPerPage
  const to = from + itemsPerPage - 1

  const [debouncedSearchQuery, setDebouncedSearchQuery] = React.useState(searchQuery)

  React.useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearchQuery(searchQuery)
    }, 300)
    return () => clearTimeout(timer)
  }, [searchQuery])

  const { data: paginatedData, isLoading: isLoadingMessages } = useMessagesPaginated(
    {
      workspaceId,
      from,
      to,
      searchQuery: debouncedSearchQuery,
      filterType: filterType as FetchMessagesParams['filterType'],
      userId: user?.id,
      teamId: selectedTeamId,
    },
    {
      select: useCallback(
        (data: { messages: Message[]; count: number }) => ({
          messages: transformMessages(data.messages, teams, user, t),
          count: data.count,
        }),
        [teams, user, t],
      ),
      enabled: !!workspaceId && !!user,
    },
  )

  const processedMessages = useMemo(() => paginatedData?.messages || [], [paginatedData])
  const totalCount = paginatedData?.count || 0
  const totalPages = Math.max(1, Math.ceil(totalCount / itemsPerPage))

  const activeMessage = useMemo(() => {
    return processedMessages.find((m) => m.id === activeMessageId) || null
  }, [processedMessages, activeMessageId])

  const setDynamicBreadcrumbs = useSetDynamicBreadcrumbs()

  const isReply = composeData?.quote?.type === 'reply'
  const isForward = composeData?.quote?.type === 'forward'
  const activeMessageSubject = activeMessage?.subject

  React.useEffect(() => {
    const breadcrumbs = []
    if (activeMessage) {
      breadcrumbs.push({
        label: activeMessageSubject || t('messages.read.title', 'Meddelande'),
      })
    } else if (isComposing) {
      breadcrumbs.push({
        label: isReply
          ? t('messages.compose.reply_title', 'Svara')
          : isForward
            ? t('messages.compose.forward_title', 'Vidarebefordra')
            : t('messages.compose.new_title', 'Nytt meddelande'),
      })
    }
    setDynamicBreadcrumbs(breadcrumbs)
  }, [
    activeMessage?.id,
    activeMessageSubject,
    isComposing,
    isReply,
    isForward,
    setDynamicBreadcrumbs,
    t,
  ])

  React.useEffect(() => {
    return () => {
      setDynamicBreadcrumbs([])
    }
  }, [setDynamicBreadcrumbs])

  React.useEffect(() => {
    const handleReset = (e: Event) => {
      const customEvent = e as CustomEvent<{ path: string }>
      if (customEvent.detail?.path === '/messages') {
        setActiveMessageId(null)
        setIsComposing(false)
      }
    }
    window.addEventListener('yntra:breadcrumb-navigate', handleReset)
    return () => {
      window.removeEventListener('yntra:breadcrumb-navigate', handleReset)
    }
  }, [setActiveMessageId, setIsComposing])

  const markAsReadMutation = useMarkMessageAsRead(workspaceId || undefined)
  const sendMessageMutation = useSendMessage(workspaceId || undefined)
  const deleteMessagesMutation = useDeleteMessages(workspaceId || undefined)

  const [clientData, setClientData] = React.useState<Client | null>(null)
  const [teamMembers, setTeamMembers] = React.useState<string[]>([])
  const [currentUserTeams, setCurrentUserTeams] = React.useState<string[]>([])
  const [accessibleColleagues, setAccessibleColleagues] = React.useState<string[]>([])

  const [allClients, setAllClients] = React.useState<Client[]>([])

  React.useEffect(() => {
    const fetchRelations = async () => {
      // 1. Fetch all clients so assistants can evaluate permissions
      const { data: clientsData } = await supabase.from('clients').select('*')
      if (clientsData) setAllClients(clientsData)

      // 2. Client logic
      if (user?.role === 'client') {
        const { data: userData } = await supabase
          .from('users')
          .select('client_id')
          .eq('id', user.id)
          .single()
        if (userData?.client_id) {
          const { data: client } = await supabase
            .from('clients')
            .select('*')
            .eq('id', userData.client_id)
            .single()
          if (client) {
            setClientData(client)
            if (client.team_id) {
              const { data: tm } = await supabase
                .from('team_members')
                .select('user_id')
                .eq('team_id', client.team_id)
              if (tm) setTeamMembers(tm.map((t: { user_id: string }) => t.user_id))
            }
          }
        }
      }
      // 3. Assistant logic
      else if (user?.role === 'assistant') {
        const { data: myTeams } = await supabase
          .from('team_members')
          .select('team_id')
          .eq('user_id', user.id)
        if (myTeams && myTeams.length > 0) {
          const teamIds = myTeams.map((t: { team_id: string }) => t.team_id)
          setCurrentUserTeams(teamIds)

          const { data: colleagues } = await supabase
            .from('team_members')
            .select('user_id')
            .in('team_id', teamIds)
          if (colleagues) {
            setAccessibleColleagues(Array.from(new Set(colleagues.map((c) => c.user_id))))
          }
        }
      }
    }
    fetchRelations()
  }, [user])

  const availableUsers = useMemo(() => {
    if (user?.role === 'admin' || user?.role === 'platform_admin') return users
    if (user?.role === 'assistant') {
      return users.filter((u) => {
        if (u.role === 'admin' || u.role === 'platform_admin') return true
        if (accessibleColleagues.includes(u.id)) return true

        if (u.role === 'client' && u.client_id) {
          const c = allClients.find((c) => c.id === u.client_id)
          if (c) {
            const mode = c.message_settings?.allowed_contacts
            if (mode === 'open' && c.team_id && currentUserTeams.includes(c.team_id)) return true
            if (
              mode === 'contact_person' &&
              c.message_settings?.contact_person_email?.toLowerCase() === user.email?.toLowerCase()
            )
              return true
          }
        }
        return false
      })
    }

    if (user?.role === 'client') {
      if (!clientData) return []

      const mode = clientData.message_settings?.allowed_contacts || 'admin_only'
      const contactPersonEmail = clientData.message_settings?.contact_person_email?.toLowerCase()

      return users.filter((u) => {
        if (u.role === 'admin' || u.role === 'platform_admin') return true

        if (mode === 'contact_person' || mode === 'open') {
          if (contactPersonEmail && u.email.toLowerCase() === contactPersonEmail) return true
        }

        if (mode === 'open') {
          if (teamMembers.includes(u.id)) return true
        }
        return false
      })
    }

    return []
  }, [users, user, clientData, teamMembers, accessibleColleagues, allClients, currentUserTeams])

  const availableTeams = useMemo(() => {
    if (user?.role === 'admin' || user?.role === 'platform_admin') return teams
    if (user?.role === 'assistant') {
      return teams.filter((t) => currentUserTeams.includes(t.id))
    }

    if (user?.role === 'client') {
      if (!clientData || !clientData.team_id) return []

      const mode = clientData.message_settings?.allowed_contacts
      if (mode === 'open') {
        return teams.filter((t) => t.id === clientData.team_id)
      }
      return []
    }

    return []
  }, [teams, user, clientData, currentUserTeams])

  React.useEffect(() => {
    const messageId = searchParams.get('messageId')
    if (messageId && processedMessages.length > 0) {
      const msg = processedMessages.find((m) => m.id === messageId)
      if (msg) {
        setIsComposing(false)
        setActiveMessageId(messageId)
        if (msg.unread) {
          markAsReadMutation.mutate(messageId)
        }
      }
    }
  }, [
    searchParams,
    processedMessages.length,
    setIsComposing,
    setActiveMessageId,
    processedMessages,
    markAsReadMutation,
  ])

  const currentMessages = processedMessages

  React.useEffect(() => {
    setCurrentPage(1)
  }, [filterType, searchQuery, setCurrentPage])

  const handleSelectMessage = useCallback(
    (id: string) => {
      setIsComposing(false)
      setActiveMessageId(id)
      const msg = processedMessages.find((m) => m.id === id)
      if (msg && msg.unread) {
        markAsReadMutation.mutate(id)
      }
    },
    [processedMessages, setIsComposing, setActiveMessageId, markAsReadMutation],
  )

  const handleCompose = useCallback(() => {
    setActiveMessageId(null)
    setComposeData({ targetType: 'user', targetId: '', subject: '', content: '', quote: null })
    setIsComposing(true)
  }, [setActiveMessageId, setComposeData, setIsComposing])

  const handleSendMessage = useCallback(() => {
    if (!workspaceId || !user || !composeData.targetId || !composeData.content) {
      toast.error(t('messages.fill_required_fields'))
      return
    }

    let finalBody = composeData.content.trim()
    if (composeData.quote) {
      if (composeData.quote.type === 'forward') {
        finalBody += `\n\n--- ${t('messages.forwarded_message', 'Vidarebefordrat meddelande')} ---\n${t('messages.from', 'Från')}: ${composeData.quote.sender}\n${t('messages.date', 'Datum')}: ${composeData.quote.date} ${composeData.quote.timestamp}\n${t('messages.subject', 'Ämne')}: ${composeData.quote.subject}\n\n${composeData.quote.content}`
      } else {
        finalBody += `\n\n--- ${composeData.quote.date} ${composeData.quote.timestamp}, ${composeData.quote.sender} ${t('messages.wrote', 'skrev')}: ---\n${composeData.quote.content}`
      }
    }

    const payload: MessagePayload = {
      workspace_id: workspaceId,
      sender_id: user.id,
      subject: composeData.subject || t('messages.no_header'),
      body: finalBody,
      is_read: false,
    }

    if (composeData.targetType === 'user') {
      payload.receiver_id = composeData.targetId
    } else {
      payload.target_team_id = composeData.targetId
    }

    sendMessageMutation.mutate(payload, {
      onSuccess: () => {
        setIsComposing(false)
        toast.success(t('messages.message_sent'))
      },
      onError: (error) => {
        toast.error(t('messages.error_sending') + ': ' + error.message)
      },
    })
  }, [workspaceId, user, composeData, t, sendMessageMutation, setIsComposing])

  const handleDeleteSelected = useCallback(() => {
    if (selectedMsgs.length === 0) return

    const msgsToDelete = [...selectedMsgs]
    const queryKey = queryKeys.messages(workspaceId || undefined)
    const previousMessages = queryClient.getQueryData(queryKey)

    queryClient.setQueryData(queryKey, (old: Message[] = []) =>
      old.filter((m) => !msgsToDelete.includes(m.id)),
    )

    setSelectedMsgs([])

    if (deleteTimeoutRef.current) {
      clearTimeout(deleteTimeoutRef.current)
    }

    toast.success(t('messages.messages_deleted'), {
      duration: 5000,
      action: {
        label: t('common.undo'),
        onClick: () => {
          if (deleteTimeoutRef.current) {
            clearTimeout(deleteTimeoutRef.current)
            deleteTimeoutRef.current = null
          }
          queryClient.setQueryData(queryKey, previousMessages)
          toast.dismiss()
          toast.info(t('messages.undo_successful'))
        },
      },
    })

    deleteTimeoutRef.current = setTimeout(() => {
      deleteMessagesMutation.mutate(msgsToDelete, {
        onError: (error) => {
          toast.error(t('messages.error_deleting') + ': ' + error.message)
          queryClient.setQueryData(queryKey, previousMessages)
        },
      })
      deleteTimeoutRef.current = null
    }, 5000)
  }, [workspaceId, queryClient, selectedMsgs, setSelectedMsgs, t, deleteMessagesMutation])

  const handleReply = useCallback(() => {
    if (activeMessage) {
      setComposeData({
        targetType: activeMessage.isTeamMessage ? 'team' : 'user',
        targetId: activeMessage.sender_id,
        subject: activeMessage.subject.startsWith('Svar:')
          ? activeMessage.subject
          : `Svar: ${activeMessage.subject}`,
        content: '',
        quote: {
          type: 'reply',
          sender: activeMessage.sender.name,
          date: activeMessage.date,
          timestamp: activeMessage.timestamp,
          subject: activeMessage.subject,
          content: activeMessage.content,
        },
      })
      setActiveMessageId(null)
      setIsComposing(true)
    }
  }, [activeMessage, setComposeData, setActiveMessageId, setIsComposing])

  const handleForward = useCallback(() => {
    if (activeMessage) {
      setComposeData({
        targetType: 'user',
        targetId: '',
        subject: activeMessage.subject.startsWith('VB:')
          ? activeMessage.subject
          : `VB: ${activeMessage.subject}`,
        content: '',
        quote: {
          type: 'forward',
          sender: activeMessage.sender.name,
          date: activeMessage.date,
          timestamp: activeMessage.timestamp,
          subject: activeMessage.subject,
          content: activeMessage.content,
        },
      })
      setActiveMessageId(null)
      setIsComposing(true)
    }
  }, [activeMessage, setComposeData, setActiveMessageId, setIsComposing])

  const handleDeleteActiveMessage = useCallback(() => {
    if (activeMessageId) {
      const msgsToDelete = [activeMessageId]
      setActiveMessageId(null)
      deleteMessagesMutation.mutate(msgsToDelete, {
        onSuccess: () => {
          toast.success(t('messages.messages_deleted'))
        },
        onError: (error) => {
          toast.error(t('messages.error_deleting') + ': ' + error.message)
        },
      })
    }
  }, [activeMessageId, setActiveMessageId, deleteMessagesMutation, t])

  if (isComposing) {
    return (
      <MessageComposePane
        composeData={composeData}
        setComposeData={setComposeData}
        isSending={sendMessageMutation.isPending}
        handleSendMessage={handleSendMessage}
        setIsComposing={setIsComposing}
        users={availableUsers}
        teams={availableTeams}
        currentUserId={user?.id}
      />
    )
  }

  if (activeMessageId && activeMessage) {
    return (
      <MessageReadPane
        activeMessage={activeMessage}
        currentUserId={user?.id}
        setActiveMessageId={setActiveMessageId}
        handleReply={handleReply}
        handleForward={handleForward}
        handleDelete={handleDeleteActiveMessage}
      />
    )
  }

  return (
    <MessagesListPane
      messages={currentMessages}
      isLoading={isLoadingMessages}
      filterType={filterType}
      setFilterType={setFilterType}
      searchQuery={searchQuery}
      setSearchQuery={setSearchQuery}
      currentPage={currentPage}
      setCurrentPage={setCurrentPage}
      totalPages={totalPages}
      handleSelectMessage={handleSelectMessage}
      handleCompose={handleCompose}
      selectedMsgs={selectedMsgs}
      setSelectedMsgs={setSelectedMsgs}
      handleDeleteSelected={handleDeleteSelected}
    />
  )
}
