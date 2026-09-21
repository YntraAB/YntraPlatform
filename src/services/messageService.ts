import { supabase } from '@/lib/supabase'

export interface SendMessagePayload {
  workspace_id: string
  sender_id: string
  subject: string
  body: string
  is_read: false
  receiver_id?: string
  target_team_id?: string
}

export interface FetchMessagesParams {
  workspaceId?: string | null
  from?: number
  to?: number
  searchQuery?: string
  filterType?: 'inbox' | 'unread' | 'sent' | 'archive' | 'trash'
  userId?: string | null
  teamId?: string | null
}

export const messageService = {
  async fetchMessages(workspaceId: string) {
    const { data: messages, error } = await supabase
      .from('messages')
      .select(
        '*, sender:users!messages_sender_id_fkey(id, full_name, email), receiver:users!messages_receiver_id_fkey(id, full_name, email)',
      )
      .eq('workspace_id', workspaceId)
      .order('created_at', { ascending: false })

    if (error) throw error
    return messages
  },

  async fetchMessagesPaginated({
    workspaceId,
    from = 0,
    to = 9,
    searchQuery = '',
    filterType = 'inbox',
    userId,
    teamId,
  }: FetchMessagesParams) {
    if (!workspaceId) return { messages: [], count: 0 }

    let query = supabase
      .from('messages')
      .select(
        '*, sender:users!messages_sender_id_fkey(id, full_name, email), receiver:users!messages_receiver_id_fkey(id, full_name, email)',
        { count: 'exact' },
      )
      .eq('workspace_id', workspaceId)

    if (filterType === 'sent') {
      query = query.eq('sender_id', userId)
    } else {
      query = query.neq('sender_id', userId)

      if (filterType === 'unread') {
        query = query.eq('is_read', false)
      }
    }

    if (teamId) {
      query = query.eq('target_team_id', teamId)
    }

    if (searchQuery) {
      // We search in subject and body.
      // Note: Searching in sender name would require complex joins/subqueries in PostgREST
      // TODO: if we want to stay within one query. For now, subject and body are most important.
      query = query.or(`subject.ilike.%${searchQuery}%,body.ilike.%${searchQuery}%`)
    }

    const {
      data: messages,
      error,
      count,
    } = await query.order('created_at', { ascending: false }).range(from, to)

    if (error) throw error
    return { messages, count: count || 0 }
  },

  async markAsRead(messageId: string) {
    const { data, error } = await supabase
      .from('messages')
      .update({ is_read: true })
      .eq('id', messageId)
      .select()
      .single()

    if (error) throw error
    return data
  },

  async sendMessage(payload: SendMessagePayload) {
    const { data, error } = await supabase.from('messages').insert(payload).select().single()

    if (error) throw error
    return data
  },

  async sendMessages(payloads: SendMessagePayload[]) {
    const { data, error } = await supabase.from('messages').insert(payloads).select()

    if (error) throw error
    return data
  },

  async deleteMessages(messageIds: string[]) {
    const { error } = await supabase.from('messages').delete().in('id', messageIds)

    if (error) throw error
    return true
  },
}
