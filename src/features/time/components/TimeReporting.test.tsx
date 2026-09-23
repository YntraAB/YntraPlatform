import { render, screen, fireEvent, act } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { ShiftSystem } from './ShiftSystem'
import { ToReportView } from './ToReportView'
import { ToAttestView } from './ToAttestView'
import { TimeHistoryView } from './TimeHistoryView'
import type { TimeReportUI } from '../types'
import type { User } from '@/types'

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback || key,
  }),
}))

const mockShifts: TimeReportUI[] = [
  {
    id: 's-1',
    employeeId: 'u-1',
    employee: 'Sara Nilsson',
    role: 'Assistent',
    teamId: 't-1',
    team: 'Team Norr',
    workspaceId: 'ws-1',
    date: '2026-09-20',
    start: '08:00',
    end: '16:00',
    duration: 8,
    break: 30,
    status: 'not_submitted',
    location: '',
    note: 'Förmiddagspass',
  },
  {
    id: 's-2',
    employeeId: 'u-1',
    employee: 'Sara Nilsson',
    role: 'Assistent',
    teamId: 't-1',
    team: 'Team Norr',
    workspaceId: 'ws-1',
    date: '2026-09-19',
    start: '16:00',
    end: '21:00',
    duration: 5,
    break: 0,
    status: 'pending_attest',
    location: '',
    note: 'Kvällspass',
  },
  {
    id: 's-3',
    employeeId: 'u-1',
    employee: 'Sara Nilsson',
    role: 'Assistent',
    teamId: 't-2',
    team: 'Team Söder',
    workspaceId: 'ws-1',
    date: '2026-09-18',
    start: '09:00',
    end: '17:00',
    duration: 8,
    break: 30,
    status: 'approved',
    location: '',
    note: 'Helgpass',
  },
]

const mockTeams = [
  { id: 't-1', name: 'Team Norr' },
  { id: 't-2', name: 'Team Söder' },
]

const mockUsers: User[] = [
  {
    id: 'u-1',
    email: 'sara@yntra.se',
    name: 'Sara Nilsson',
    full_name: 'Sara Nilsson',
    role: 'assistant',
  },
]

describe('Time Reporting & ShiftSystem', () => {
  it('renders unsubmitted shifts and quick report action', () => {
    const handleReport = vi.fn()
    const handleApprove = vi.fn()

    render(
      <ShiftSystem
        shifts={mockShifts}
        selectedContext={{ type: 'employee', id: 'u-1' }}
        searchQuery=""
        filterType="all"
        currentPage={1}
        selectedShifts={[]}
        listMode="current"
        isDeleteAlertOpen={false}
        hasApprovePermission={true}
        setListMode={vi.fn()}
        setFilterType={vi.fn()}
        setSearchQuery={vi.fn()}
        setCurrentPage={vi.fn()}
        setSelectedShifts={vi.fn()}
        setIsDeleteAlertOpen={vi.fn()}
        handleApprove={handleApprove}
        handleReport={handleReport}
        handleDelete={vi.fn()}
      />
    )

    expect(screen.getByText('Förmiddagspass')).toBeDefined()
    expect(screen.getByText('Kvällspass')).toBeDefined()
    expect(screen.getByText('Helgpass')).toBeDefined()

    const reportButtons = screen.getAllByRole('button', { name: /Rapportera/i })
    expect(reportButtons.length).toBeGreaterThan(0)

    fireEvent.click(reportButtons[0])
    expect(handleReport).toHaveBeenCalledWith(['s-1'])
  })

  it('renders inline Attestera action for admin on pending shift', () => {
    const handleApprove = vi.fn()

    render(
      <ShiftSystem
        shifts={mockShifts}
        selectedContext={{ type: 'employee', id: 'u-1' }}
        searchQuery=""
        filterType="all"
        currentPage={1}
        selectedShifts={[]}
        listMode="current"
        isDeleteAlertOpen={false}
        hasApprovePermission={true}
        setListMode={vi.fn()}
        setFilterType={vi.fn()}
        setSearchQuery={vi.fn()}
        setCurrentPage={vi.fn()}
        setSelectedShifts={vi.fn()}
        setIsDeleteAlertOpen={vi.fn()}
        handleApprove={handleApprove}
        handleReport={vi.fn()}
        handleDelete={vi.fn()}
      />
    )

    const approveButtons = screen.getAllByRole('button', { name: /Attestera/i })
    expect(approveButtons.length).toBeGreaterThan(0)

    fireEvent.click(approveButtons[0])
    expect(handleApprove).toHaveBeenCalledWith(['s-2'])
  })

  it('renders worker history with total reported hours and monthly summary', () => {
    render(
      <ShiftSystem
        shifts={mockShifts}
        selectedContext={{ type: 'employee', id: 'u-1' }}
        searchQuery=""
        filterType="all"
        currentPage={1}
        selectedShifts={[]}
        listMode="history"
        isDeleteAlertOpen={false}
        hasApprovePermission={false}
        setListMode={vi.fn()}
        setFilterType={vi.fn()}
        setSearchQuery={vi.fn()}
        setCurrentPage={vi.fn()}
        setSelectedShifts={vi.fn()}
        setIsDeleteAlertOpen={vi.fn()}
        handleApprove={vi.fn()}
        handleReport={vi.fn()}
        handleDelete={vi.fn()}
      />
    )

    expect(screen.getByText('13h')).toBeDefined()
    expect(screen.getAllByText('8h').length).toBeGreaterThan(0)
    expect(screen.getAllByText('5h').length).toBeGreaterThan(0)
    expect(screen.getByText('Kvällspass')).toBeDefined()
    expect(screen.getByText('Helgpass')).toBeDefined()
  })
})

describe('ToReportView', () => {
  it('renders not_submitted shifts, supports single and batch reporting', async () => {
    const handleReportSingle = vi.fn()
    const handleReportBatch = vi.fn()
    const handleManualReport = vi.fn().mockResolvedValue(true)

    render(
      <ToReportView
        shifts={mockShifts}
        teams={mockTeams}
        userId="u-1"
        onReportSingle={handleReportSingle}
        onReportBatch={handleReportBatch}
        onManualReport={handleManualReport}
      />
    )

    // Check only unsubmitted shift is displayed
    expect(screen.getByText('Förmiddagspass')).toBeDefined()
    expect(screen.queryByText('Kvällspass')).toBeNull()

    // Check single report button
    const singleReportBtn = screen.getByRole('button', { name: /Rapportera/i })
    fireEvent.click(singleReportBtn)
    expect(handleReportSingle).toHaveBeenCalledWith('s-1')

    // Check select all
    const selectAllCheckbox = screen.getByRole('checkbox', { name: /Markera alla/i })
    fireEvent.click(selectAllCheckbox)

    // Batch button appears
    const batchReportBtn = screen.getByRole('button', { name: /Godkänn och rapportera alla/i })
    fireEvent.click(batchReportBtn)
    expect(handleReportBatch).toHaveBeenCalledWith(['s-1'])
  })
})

describe('ToAttestView', () => {
  it('renders pending_attest shifts and handles single and batch attestation', async () => {
    const handleApproveSingle = vi.fn()
    const handleApproveBatch = vi.fn()

    render(
      <ToAttestView
        shifts={mockShifts}
        teams={mockTeams}
        users={mockUsers}
        onApproveSingle={handleApproveSingle}
        onApproveBatch={handleApproveBatch}
      />
    )

    // Only s-2 is pending_attest
    expect(screen.getByText('Kvällspass')).toBeDefined()
    expect(screen.queryByText('Förmiddagspass')).toBeNull()

    // Check single attest button
    const attestButton = screen.getByRole('button', { name: /Attestera/i })
    await act(async () => {
      fireEvent.click(attestButton)
    })
    expect(handleApproveSingle).toHaveBeenCalledWith('s-2')

    // Select and batch attest
    const selectAllCheckbox = screen.getByRole('checkbox', { name: /Markera alla/i })
    await act(async () => {
      fireEvent.click(selectAllCheckbox)
    })

    const batchButton = screen.getByRole('button', { name: /Godkänn & attestera alla/i })
    await act(async () => {
      fireEvent.click(batchButton)
    })
    expect(handleApproveBatch).toHaveBeenCalledWith(['s-2'])
  })

  it('supports multiple months in pending attest shifts', () => {
    const multiMonthShifts: TimeReportUI[] = [
      {
        ...mockShifts[1],
        id: 's-aug',
        date: '2026-08-25',
        note: 'Augustipass',
        status: 'pending_attest',
      },
      {
        ...mockShifts[1],
        id: 's-sep',
        date: '2026-09-15',
        note: 'Septemberpass',
        status: 'pending_attest',
      },
    ]

    render(
      <ToAttestView
        shifts={multiMonthShifts}
        teams={mockTeams}
        users={mockUsers}
        onApproveSingle={vi.fn()}
        onApproveBatch={vi.fn()}
      />
    )

    // Both shifts from different months are listed cleanly without text wrapping
    expect(screen.getByText('Augustipass')).toBeDefined()
    expect(screen.getByText('Septemberpass')).toBeDefined()
  })
})

describe('TimeHistoryView', () => {
  it('renders metrics and filters reported shifts', () => {
    render(
      <TimeHistoryView
        shifts={mockShifts}
        teams={mockTeams}
        users={mockUsers}
        activeRole="admin"
        currentUserId="u-1"
      />
    )

    // Total reported is s-2 (5h) + s-3 (8h) = 13h
    expect(screen.getByText('13h')).toBeDefined()
    // Approved is 8h (in metric card and row)
    expect(screen.getAllByText('8h').length).toBeGreaterThan(0)
    // Pending is 5h (in metric card and row)
    expect(screen.getAllByText('5h').length).toBeGreaterThan(0)

    // Shifts present in history
    expect(screen.getByText('Kvällspass')).toBeDefined()
    expect(screen.getByText('Helgpass')).toBeDefined()
    expect(screen.queryByText('Förmiddagspass')).toBeNull()
  })
})
