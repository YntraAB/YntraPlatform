import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { Sidebar } from './Sidebar'
import { MemoryRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
    },
  },
})

vi.mock('@/hooks/useUnreadNotes', () => ({
  useUnreadNotes: () => ({ total: 5 }),
}))

vi.mock('@/hooks/useAuth', () => ({
  useAuth: () => ({ user: { id: 'user-1', role: 'admin', name: 'Test Admin' } }),
}))

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    modules: { assistance: true, messaging: true, notes: true, scheduling: true },
    workspaceId: 'ws-1',
    setAdminWorkspace: vi.fn(),
  }),
}))

vi.mock('@/lib/supabase', () => ({
  supabase: {
    from: vi.fn(() => ({
      select: vi.fn().mockReturnThis(),
      eq: vi.fn().mockReturnThis(),
      neq: vi.fn().mockReturnThis(),
      then: vi.fn().mockImplementation((cb) => cb({ data: [], count: 0 })),
    })),
    channel: vi.fn(() => ({
      on: vi.fn().mockReturnThis(),
      subscribe: vi.fn().mockReturnThis(),
      unsubscribe: vi.fn(),
    })),
    removeChannel: vi.fn(),
  },
}))

const renderSidebar = (activeSection = 'schedule', onSectionChange = vi.fn()) => {
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <Sidebar activeSection={activeSection} onSectionChange={onSectionChange} />
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

describe('Sidebar Component', () => {
  it('renders navigation items correctly', () => {
    renderSidebar()
    expect(screen.getByTestId('sidebar-item-inbox')).toBeInTheDocument()
    expect(screen.getByTestId('sidebar-item-notes')).toBeInTheDocument()
  })

  it('displays unread badges when applicable', () => {
    renderSidebar()
    expect(screen.getByText('5')).toBeInTheDocument()
  })

  it('calls onSectionChange when a menu item is clicked', () => {
    const onSectionChange = vi.fn()
    renderSidebar('schedule', onSectionChange)

    const inboxItem = screen.getByTestId('sidebar-item-inbox')
    fireEvent.click(inboxItem)

    expect(onSectionChange).toHaveBeenCalledWith('inbox')
  })

  it('shows Yntra logo', () => {
    renderSidebar()
    expect(screen.getByText('Yntra Platform')).toBeInTheDocument()
  })
})
