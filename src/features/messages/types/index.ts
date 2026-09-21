export interface MessageSender {
  name: string
  avatar?: string
  full_name?: string
  email?: string
}

export interface ProcessedMessage {
  id: string
  sender_id: string
  receiver_id?: string
  target_team_id?: string
  folderId: 'inbox' | 'sent' | 'archive' | 'trash'
  sender: MessageSender
  to: string
  isTeamMessage: boolean
  subject: string
  snippet: string
  content: string
  timestamp: string
  date: string
  unread: boolean
}

export interface QuoteData {
  type: 'reply' | 'forward'
  sender: string
  date: string
  timestamp: string
  subject: string
  content: string
}

export interface ComposeData {
  targetType: 'user' | 'team'
  targetId: string
  subject: string
  content: string
  quote: QuoteData | null
}
