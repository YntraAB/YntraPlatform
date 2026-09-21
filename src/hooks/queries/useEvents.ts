import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { eventService } from '@/services/eventService'
import type { CalendarEvent } from '@/types'
import { useMemo } from 'react'

export function useEvents(workspaceId: string | null, userId: string | null) {
  const queryClient = useQueryClient()
  const queryKey = useMemo(() => ['events', workspaceId], [workspaceId])


  const query = useQuery({
    queryKey,
    queryFn: async () => {
      if (!workspaceId) return []
      return eventService.getWorkspaceEvents(workspaceId)
    },
    enabled: !!workspaceId,
  })

  const addEvent = useMutation({
    mutationFn: async (event: Omit<CalendarEvent, 'id'>) => {
      if (!workspaceId || !userId) throw new Error('Missing context')
      return eventService.createEvent(workspaceId, userId, event)
    },
    onMutate: async (newEvent) => {
      await queryClient.cancelQueries({ queryKey })
      const previousEvents = queryClient.getQueryData<CalendarEvent[]>(queryKey)
      const optimisticEvent = { ...newEvent, id: Math.random().toString(36).substring(2, 11) } as CalendarEvent
      queryClient.setQueryData<CalendarEvent[]>(queryKey, (old) => [...(old || []), optimisticEvent])
      return { previousEvents }
    },
    onError: (_err, _newEvent, context) => {
      queryClient.setQueryData(queryKey, context?.previousEvents)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey })
    },
  })

  const updateEvent = useMutation({
    mutationFn: async ({
      eventId,
      updates,
    }: {
      eventId: string
      updates: Partial<CalendarEvent>
    }) => {
      return eventService.updateEvent(eventId, updates)
    },
    onMutate: async ({ eventId, updates }) => {
      await queryClient.cancelQueries({ queryKey })
      const previousEvents = queryClient.getQueryData<CalendarEvent[]>(queryKey)
      queryClient.setQueryData<CalendarEvent[]>(queryKey, (old) =>
        old?.map((event) => (event.id === eventId ? { ...event, ...updates } : event)),
      )
      return { previousEvents }
    },
    onError: (_err, _variables, context) => {
      queryClient.setQueryData(queryKey, context?.previousEvents)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey })
    },
  })

  const deleteEvent = useMutation({
    mutationFn: async (eventId: string) => {
      return eventService.deleteEvent(eventId)
    },
    onMutate: async (eventId) => {
      await queryClient.cancelQueries({ queryKey })
      const previousEvents = queryClient.getQueryData<CalendarEvent[]>(queryKey)
      queryClient.setQueryData<CalendarEvent[]>(queryKey, (old) =>
        old?.filter((event) => event.id !== eventId),
      )
      return { previousEvents }
    },
    onError: (_err, _eventId, context) => {
      queryClient.setQueryData(queryKey, context?.previousEvents)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey })
    },
  })

  return useMemo(
    () => ({
      ...query,
      events: query.data || [],
      addEvent: addEvent.mutateAsync,
      updateEvent: (eventId: string, updates: Partial<CalendarEvent>) =>
        updateEvent.mutateAsync({ eventId, updates }),
      deleteEvent: deleteEvent.mutateAsync,
    }),
    [query, addEvent, updateEvent, deleteEvent],
  )
}
