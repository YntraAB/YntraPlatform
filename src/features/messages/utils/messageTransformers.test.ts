import { describe, it, expect, vi } from 'vitest'
import { transformMessages } from './messageTransformers'
import type { Message, User } from '@/types'
import type { TFunction } from 'i18next'

describe('messageTransformers', () => {
  const mockT = vi.fn((key: string) => key)

  const mockUser: User = {
    id: 'user-1',
    email: 'user@example.com',
    name: 'Test User',
    full_name: 'Test User',
    role: 'assistant',
  }

  const mockTeams = [
    { id: 'team-1', name: 'Team Alpha' },
    { id: 'team-2', name: 'Team Beta' },
  ]

  const mockRawMessage: Message = {
    id: 'msg-1',
    workspace_id: 'ws-1',
    sender_id: 'user-2',
    receiver_id: 'user-1',
    subject: 'Hello',
    body: 'How are you?',
    created_at: '2024-03-20T10:00:00Z',
    is_read: false,
    sender: {
      id: 'user-2',
      name: 'Sender Name',
      full_name: 'Sender Name',
      email: 'sender@example.com',
      role: 'user',
    },
    receiver: {
      id: 'user-1',
      name: 'Test User',
      full_name: 'Test User',
      email: 'user@example.com',
      role: 'assistant',
    },
  }

  it('transforms a basic direct message correctly', () => {
    const results = transformMessages([mockRawMessage], mockTeams, mockUser, mockT as unknown as TFunction)

    expect(results).toHaveLength(1)
    expect(results[0].id).toBe('msg-1')
    expect(results[0].folderId).toBe('inbox')
    expect(results[0].to).toBe('messages.you')
    expect(results[0].sender.name).toBe('Sender Name')
    expect(results[0].snippet).toContain('How are you?')
  })

  it('transforms a sent message correctly', () => {
    const sentMessage: Message = {
      ...mockRawMessage,
      sender_id: 'user-1',
      receiver_id: 'user-2',
      sender: mockUser,
      receiver: {
        id: 'user-2',
        name: 'Recipient Name',
        full_name: 'Recipient Name',
        email: 'recipient@example.com',
        role: 'user',
      },
    }

    const results = transformMessages([sentMessage], mockTeams, mockUser, mockT as unknown as TFunction)

    expect(results[0].folderId).toBe('sent')
    expect(results[0].to).toBe('Recipient Name')
  })

  it('transforms a team message correctly', () => {
    const teamMessage = {
      ...mockRawMessage,
      target_team_id: 'team-1',
      receiver_id: null,
    }

    const results = transformMessages([teamMessage], mockTeams, mockUser, mockT as unknown as TFunction)

    expect(results[0].isTeamMessage).toBe(true)
    expect(results[0].to).toBe('Team Alpha')
  })

  it('handles missing subject and body', () => {
    const emptyMessage: Message = {
      ...mockRawMessage,
      subject: null,
      body: null,
    }

    const results = transformMessages([emptyMessage], mockTeams, mockUser, mockT as unknown as TFunction)

    expect(results[0].subject).toBe('messages.no_header')
    expect(results[0].snippet).toBe('')
  })
})
