import { renderHook, waitFor } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useTeamPermissions } from './useTeamPermissions'
import { supabase } from '@/lib/supabase'
import { useAuth } from './useAuth'

// Mock the dependencies
vi.mock('@/lib/supabase', () => ({
  supabase: {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          eq: vi.fn(() => ({
            single: vi.fn(),
          })),
          single: vi.fn(),
        })),
      })),
    })),
  },
}))

vi.mock('./useAuth', () => ({
  useAuth: vi.fn(),
}))

describe('useTeamPermissions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns default permissions when no user is logged in', async () => {
    vi.mocked(useAuth).mockReturnValue({ user: null } as any)

    const { result } = renderHook(() => useTeamPermissions('team-1'))

    expect(result.current.loading).toBe(false)
    expect(result.current.permissions.is_admin).toBe(false)
    expect(result.current.permissions.can_manage_schedule).toBe(false)
  })

  it('returns full permissions for platform_admin', async () => {
    vi.mocked(useAuth).mockReturnValue({ user: { id: 'user-1', role: 'platform_admin' } } as any)

    const { result } = renderHook(() => useTeamPermissions('team-1'))

    expect(result.current.loading).toBe(false)
    expect(result.current.permissions.is_admin).toBe(true)
    expect(result.current.permissions.can_manage_schedule).toBe(true)
  })

  it('fetches team-specific permissions for staff role', async () => {
    const mockUser = { id: 'user-1', role: 'staff' }
    vi.mocked(useAuth).mockReturnValue({ user: mockUser } as any)

    const mockRoleData = {
      permissions: {
        can_manage_schedule: true,
        can_manage_notes: false,
        can_approve_time_reports: true,
      },
    }

    // Mock supabase responses
    const fromSpy = vi.spyOn(supabase, 'from')
    fromSpy
      .mockReturnValueOnce({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue({ data: { role_id: 'role-1' }, error: null }),
      } as unknown as any)
      .mockReturnValueOnce({
        select: vi.fn().mockReturnThis(),
        eq: vi.fn().mockReturnThis(),
        single: vi.fn().mockResolvedValue({ data: mockRoleData, error: null }),
      } as unknown as any)

    const { result } = renderHook(() => useTeamPermissions('team-1'))

    await waitFor(() => expect(result.current.loading).toBe(false))

    expect(result.current.permissions.can_manage_schedule).toBe(true)
    expect(result.current.permissions.can_manage_notes).toBe(false)
    expect(result.current.permissions.can_approve_time_reports).toBe(true)
    expect(result.current.permissions.is_admin).toBe(false)
  })
})
