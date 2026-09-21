import React, { createContext, useContext, useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { supabase } from '@/lib/supabase'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'
import { toast } from 'sonner'
import { queryKeys } from '@/lib/query-keys'
import { useTranslation } from 'react-i18next'
import { Bell } from 'lucide-react'

interface RealtimeContextValue {}

const RealtimeContext = createContext<RealtimeContextValue | undefined>(undefined)

export const RealtimeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { workspaceId } = useWorkspace()
  const { user } = useAuth()
  const queryClient = useQueryClient()
  const { t } = useTranslation()

  useEffect(() => {
    if (!workspaceId || !user) return


    const channel = supabase
      .channel(`global-workspace-${workspaceId}`)
      // Monitor Notes
      .on(
        'postgres_changes',
        {
          event: '*',
          schema: 'public',
          table: 'notes',
          filter: `workspace_id=eq.${workspaceId}`,
        },
        (payload) => {
          console.log('[Realtime] Note change detected:', payload)
          
          // Invalidate relevant queries
          queryClient.invalidateQueries({ queryKey: queryKeys.noteTeams(workspaceId) })
          queryClient.invalidateQueries({ queryKey: queryKeys.workspaceNotes(workspaceId) })
          
          if (payload.new && (payload.new as any).team_id) {
            queryClient.invalidateQueries({ queryKey: queryKeys.teamNotes((payload.new as any).team_id) })
          }

          // Show toast if not our own change
          const actorId = (payload.new as any)?.author_id || (payload.old as any)?.author_id
          if (actorId && actorId !== user.id && payload.eventType === 'INSERT') {
            toast(t('notifications.new_note_title'), {
              description: t('notifications.new_note_description'),
              icon: <Bell className="h-4 w-4 text-primary" />,
            })
          }
        }
      )
      // Monitor Messages
      .on(
        'postgres_changes',
        {
          event: 'INSERT',
          schema: 'public',
          table: 'messages',
          filter: `workspace_id=eq.${workspaceId}`,
        },
        (payload) => {
          queryClient.invalidateQueries({ queryKey: queryKeys.messages(workspaceId) })
          
          if (payload.new && (payload.new as any).sender_id !== user.id) {
            toast(t('notifications.new_message_title'), {
              description: t('notifications.new_message_description'),
              icon: <Bell className="h-4 w-4 text-primary" />,
            })
          }
        }
      )
      // Monitor Events/Schedule
      .on(
        'postgres_changes',
        {
          event: '*',
          schema: 'public',
          table: 'events',
          filter: `workspace_id=eq.${workspaceId}`,
        },
        (payload) => {
          queryClient.invalidateQueries({ queryKey: ['events'] }) // Generic invalidation for all event queries
          
          const actorId = (payload.new as any)?.created_by || (payload.old as any)?.created_by
          if (actorId && actorId !== user.id && payload.eventType === 'INSERT') {
            toast(t('notifications.new_event_title'), {
              description: t('notifications.new_event_description'),
              icon: <Bell className="h-4 w-4 text-primary" />,
            })
          }
        }
      )
      .subscribe()

    return () => {
      supabase.removeChannel(channel)
    }
  }, [workspaceId, user?.id, queryClient, t])

  return (
    <RealtimeContext.Provider value={{}}>
      {children}
    </RealtimeContext.Provider>
  )
}

export const useRealtime = () => {
  const context = useContext(RealtimeContext)
  if (context === undefined) {
    throw new Error('useRealtime must be used within a RealtimeProvider')
  }
  return context
}
