import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import {
  messageService,
  type SendMessagePayload,
  type FetchMessagesParams,
} from '@/services/messageService'
import { useEffect } from 'react'
import { queryKeys } from '@/lib/query-keys'
import { toast } from 'sonner'
import type { Message } from '@/types'

export const useMessages = <TData = Message[]>(
  workspaceId: string | undefined,
  options?: { select?: (data: Message[]) => TData },
) => {
  const queryKey = queryKeys.messages(workspaceId)


  return useQuery({
    queryKey,
    queryFn: () => messageService.fetchMessages(workspaceId!),
    enabled: !!workspaceId,
    ...options,
  })
}

export const useMessagesPaginated = <TData = { messages: Message[]; count: number }>(
  params: FetchMessagesParams,
  options?: {
    select?: (data: { messages: Message[]; count: number }) => TData
    enabled?: boolean
  },
) => {
  const queryClient = useQueryClient()
  const { enabled = true } = options || {}
  const { workspaceId, from, to, searchQuery, filterType, userId, teamId } = params

  const queryKey = ['messages', workspaceId, { from, to, searchQuery, filterType, userId, teamId }]

  useEffect(() => {
    if (enabled && workspaceId && to !== undefined && from !== undefined) {
      const pageSize = to - from + 1

      const nextFrom = from + pageSize
      const nextTo = to + pageSize
      queryClient.prefetchQuery({
        queryKey: ['messages', workspaceId, { ...params, from: nextFrom, to: nextTo, searchQuery }],
        queryFn: () =>
          messageService.fetchMessagesPaginated({
            ...params,
            from: nextFrom,
            to: nextTo,
            searchQuery,
          }),
      })

      if (from >= pageSize) {
        const prevFrom = from - pageSize
        const prevTo = to - pageSize
        queryClient.prefetchQuery({
          queryKey: [
            'messages',
            workspaceId,
            { ...params, from: prevFrom, to: prevTo, searchQuery },
          ],
          queryFn: () =>
            messageService.fetchMessagesPaginated({
              ...params,
              from: prevFrom,
              to: prevTo,
              searchQuery,
            }),
        })
      }
    }
  }, [from, to, searchQuery, params, queryClient, workspaceId, enabled])


  return useQuery({
    queryKey,
    queryFn: () => messageService.fetchMessagesPaginated(params),
    placeholderData: keepPreviousData,
    ...options,
    enabled: !!workspaceId && enabled,
  })
}

export const useMarkMessageAsRead = (workspaceId: string | undefined) => {
  const queryClient = useQueryClient()
  const { t } = useTranslation()
  const queryKey = ['messages', workspaceId]

  return useMutation({
    mutationFn: (messageId: string) => messageService.markAsRead(messageId),
    onMutate: async (messageId) => {
      await queryClient.cancelQueries({ queryKey })
      const previousData = queryClient.getQueryData<Message[]>(queryKey)
      queryClient.setQueryData<Message[]>(queryKey, (old) =>
        old?.map((m) => (m.id === messageId ? { ...m, is_read: true } : m)),
      )
      return { previousData }
    },
    onError: (error: Error, _messageId, context) => {
      queryClient.setQueryData(queryKey, context?.previousData)
      toast.error(`${t('messages.error_marking_read')}: ${error.message}`)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey })
    },
  })
}

export const useSendMessage = (workspaceId: string | undefined) => {
  const queryClient = useQueryClient()
  const { t } = useTranslation()
  const queryKey = ['messages', workspaceId]

  return useMutation({
    mutationFn: (payload: SendMessagePayload) => messageService.sendMessage(payload),
    onMutate: async (payload) => {
      await queryClient.cancelQueries({ queryKey })
      const previousData = queryClient.getQueryData<Message[]>(queryKey)
      const newMessage: Message = {
        id: Math.random().toString(36).substring(2, 11),
        workspace_id: workspaceId || '',
        sender_id: payload.sender_id,
        receiver_id: payload.receiver_id || null,
        target_team_id: payload.target_team_id || null,
        subject: payload.subject,
        body: payload.body,
        is_read: false,
        created_at: new Date().toISOString(),
      }
      queryClient.setQueryData<Message[]>(queryKey, (old) => [newMessage, ...(old || [])])
      return { previousData }
    },
    onError: (error: Error, _payload, context) => {
      queryClient.setQueryData(queryKey, context?.previousData)
      toast.error(`${t('messages.error_sending')}: ${error.message}`)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey })
    },
  })
}

export const useDeleteMessages = (workspaceId: string | undefined) => {
  const queryClient = useQueryClient()
  const { t } = useTranslation()
  const queryKey = ['messages', workspaceId]

  return useMutation({
    mutationFn: (messageIds: string[]) => messageService.deleteMessages(messageIds),
    onMutate: async (messageIds) => {
      await queryClient.cancelQueries({ queryKey })
      const previousData = queryClient.getQueryData<Message[]>(queryKey)
      queryClient.setQueryData<Message[]>(queryKey, (old) =>
        old?.filter((m) => !messageIds.includes(m.id)),
      )
      return { previousData }
    },
    onError: (error: Error, _messageIds, context) => {
      queryClient.setQueryData(queryKey, context?.previousData)
      toast.error(`${t('messages.error_deleting')}: ${error.message}`)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey })
    },
  })
}
