import { describe, it, expect, vi, beforeEach } from 'vitest'
import { messageService } from './messageService'
import { supabase } from '@/lib/supabase'

// Mock supabase
vi.mock('@/lib/supabase', () => ({
  supabase: {
    from: vi.fn(() => ({
      select: vi.fn().mockReturnThis(),
      eq: vi.fn().mockReturnThis(),
      neq: vi.fn().mockReturnThis(),
      order: vi.fn().mockReturnThis(),
      range: vi.fn().mockReturnThis(),
      update: vi.fn().mockReturnThis(),
      insert: vi.fn().mockReturnThis(),
      delete: vi.fn().mockReturnThis(),
      in: vi.fn().mockReturnThis(),
      single: vi.fn(),
      or: vi.fn().mockReturnThis(),
    })),
  },
}))

describe('messageService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('fetchMessages', () => {
    it('calls supabase with correct parameters', async () => {
      const fromSpy = vi.spyOn(supabase, 'from')
      const mockData = [{ id: '1', subject: 'Test' }]

      // Setup mock return value
      const selectMock = {
        eq: vi.fn().mockReturnThis(),
        order: vi.fn().mockResolvedValue({ data: mockData, error: null }),
      }
      fromSpy.mockReturnValue({ select: vi.fn().mockReturnValue(selectMock) } as unknown as any)

      const result = await messageService.fetchMessages('ws-1')

      expect(fromSpy).toHaveBeenCalledWith('messages')
      expect(result).toEqual(mockData)
    })
  })

  describe('markAsRead', () => {
    it('updates is_read to true', async () => {
      const fromSpy = vi.spyOn(supabase, 'from')
      const updateSpy = vi.fn().mockReturnThis()
      const eqSpy = vi.fn().mockReturnThis()
      const selectSpy = vi.fn().mockReturnThis()
      const singleSpy = vi.fn().mockResolvedValue({ data: { id: '1', is_read: true }, error: null })

      fromSpy.mockReturnValue({
        update: updateSpy,
        eq: eqSpy,
        select: selectSpy,
        single: singleSpy,
      } as unknown as any)

      await messageService.markAsRead('msg-1')

      expect(updateSpy).toHaveBeenCalledWith({ is_read: true })
      expect(eqSpy).toHaveBeenCalledWith('id', 'msg-1')
    })
  })

  describe('sendMessage', () => {
    it('inserts the payload correctly', async () => {
      const payload = {
        workspace_id: 'ws-1',
        sender_id: 'user-1',
        subject: 'Hi',
        body: 'Test',
        is_read: false as const,
      }

      const fromSpy = vi.spyOn(supabase, 'from')
      const insertSpy = vi.fn().mockReturnThis()
      const selectSpy = vi.fn().mockReturnThis()
      const singleSpy = vi
        .fn()
        .mockResolvedValue({ data: { ...payload, id: 'msg-new' }, error: null })

      fromSpy.mockReturnValue({
        insert: insertSpy,
        select: selectSpy,
        single: singleSpy,
      } as unknown as any)

      const result = await messageService.sendMessage(payload)

      expect(insertSpy).toHaveBeenCalledWith(payload)
      expect(result.id).toBe('msg-new')
    })
  })
})
