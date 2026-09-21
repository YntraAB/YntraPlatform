import type { TFunction } from 'i18next'
import type { ProcessedMessage } from '../types'
import type { User, Message } from '@/types'

interface SimpleTeam {
  id: string
  name: string
}

export const transformMessages = (
  rawMessages: Message[],
  teams: SimpleTeam[],
  user: User | null,
  t: TFunction,
): ProcessedMessage[] => {
  return rawMessages.map((m) => {
    let toName = t('messages.you')
    if (m.target_team_id) {
      toName = teams.find((team) => team.id === m.target_team_id)?.name || t('common.team')
    } else if (m.receiver_id === user?.id) {
      toName = t('messages.you')
    } else {
      const rcvrData = Array.isArray(m.receiver) ? m.receiver[0] : m.receiver
      toName = rcvrData
        ? rcvrData.full_name || rcvrData.email || t('messages.anonymous')
        : t('messages.anonymous')
    }

    const sndrData = Array.isArray(m.sender) ? m.sender[0] : m.sender
    const senderName = sndrData
      ? sndrData.full_name || sndrData.email || t('messages.system')
      : t('messages.system')

    return {
      id: m.id,
      sender_id: m.sender_id,
      receiver_id: m.receiver_id || undefined,
      target_team_id: m.target_team_id || undefined,
      folderId: m.sender_id === user?.id ? 'sent' : ('inbox' as const),
      sender: { name: senderName, avatar: '' },
      to: toName,
      isTeamMessage: !!m.target_team_id,
      subject: m.subject || t('messages.no_header'),
      snippet: m.body ? m.body.substring(0, 40) + '...' : '',
      content: m.body || '',
      timestamp: new Date(m.created_at).toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
      }),
      date: new Date(m.created_at).toLocaleDateString(),
      unread: !m.is_read,
    }
  })
}
