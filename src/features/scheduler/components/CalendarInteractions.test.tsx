import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { WeekView } from './calendar/WeekView'
import { DayView } from './calendar/DayView'
import { MonthView } from './calendar/MonthView'
import { MiniCalendar } from './MiniCalendar'
import { CalendarView } from './CalendarView'
import type { CalendarEvent, User } from '@/types'

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    settings: {
      business_hours: { start: 0, end: 23 },
      language: 'sv',
      week_start: 1,
      default_calendar_view: 'week',
    },
    preferences: {
      calendar_density: 'comfortable',
    },
    modules: { scheduling: true },
  }),
}))

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback || key,
  }),
}))

const mockUsers: User[] = [
  {
    id: 'user-1',
    name: 'Anna Andersson',
    email: 'anna@example.com',
    role: 'user',
  },
]

const mockEvents: CalendarEvent[] = []

describe('Calendar Navigation & Day Switching', () => {
  it('WeekView allows clicking a day header to drill down into Day view', () => {
    const handleDateChange = vi.fn()
    const handleViewChange = vi.fn()
    const selectedDate = new Date(2026, 8, 16) // Wednesday Sep 16, 2026

    render(
      <WeekView
        selectedDate={selectedDate}
        events={mockEvents}
        users={mockUsers}
        onEventClick={vi.fn()}
        onDateChange={handleDateChange}
        onViewChange={handleViewChange}
        hourHeight={60}
      />,
    )

    // Find the day header buttons
    const buttons = screen.getAllByRole('button')
    // Click one of the day headers
    expect(buttons.length).toBeGreaterThan(0)
    fireEvent.click(buttons[0])

    expect(handleDateChange).toHaveBeenCalled()
    expect(handleViewChange).toHaveBeenCalledWith('day')
  })

  it('MonthView allows clicking a day cell to drill down into Day view', () => {
    const handleDateChange = vi.fn()
    const handleViewChange = vi.fn()
    const selectedDate = new Date(2026, 8, 16)

    render(
      <MonthView
        selectedDate={selectedDate}
        events={mockEvents}
        users={mockUsers}
        onEventClick={vi.fn()}
        onDateChange={handleDateChange}
        onViewChange={handleViewChange}
      />,
    )

    // Month view renders day cells with titles
    const dayCells = screen.getAllByTitle(/dagsöversikt/i)
    expect(dayCells.length).toBeGreaterThan(0)

    fireEvent.click(dayCells[10])
    expect(handleDateChange).toHaveBeenCalled()
    expect(handleViewChange).toHaveBeenCalledWith('day')
  })

  it('MiniCalendar browses months independently and selects dates on click', () => {
    const handleSelectDate = vi.fn()
    const selectedDate = new Date(2026, 8, 16) // September 2026

    const { container } = render(
      <MiniCalendar
        selectedDate={selectedDate}
        onSelectDate={handleSelectDate}
      />,
    )

    // Heading should show September 2026
    expect(screen.getByText(/september 2026/i)).toBeDefined()

    // Find chevron buttons for browsing months
    const prevNextButtons = container.querySelectorAll('button')
    const prevBtn = prevNextButtons[0]
    const nextBtn = prevNextButtons[1]

    // Clicking next month button changes the displayed month without triggering onSelectDate
    fireEvent.click(nextBtn)
    expect(screen.getByText(/oktober 2026/i)).toBeDefined()
    expect(handleSelectDate).not.toHaveBeenCalled()

    // Clicking previous month button goes back
    fireEvent.click(prevBtn)
    expect(screen.getByText(/september 2026/i)).toBeDefined()
    expect(handleSelectDate).not.toHaveBeenCalled()

    // Clicking a date number button triggers onSelectDate
    const dateBtn = screen.getByText('15')
    fireEvent.click(dateBtn)
    expect(handleSelectDate).toHaveBeenCalled()
  })

  it('WeekView renders full-width current time line spanning across all days when current week is shown', () => {
    const today = new Date()

    const { container } = render(
      <WeekView
        selectedDate={today}
        events={mockEvents}
        users={mockUsers}
        onEventClick={vi.fn()}
        hourHeight={60}
      />,
    )

    // The current time line container should span left-0 right-0 across the day columns container
    const fullWidthLine = container.querySelector('.pointer-events-none.absolute.left-0.right-0.z-20')
    expect(fullWidthLine).toBeDefined()
    expect(fullWidthLine).not.toBeNull()

    // It should have the continuous full-width line
    const redBar = fullWidthLine?.querySelector('.bg-red-500')
    expect(redBar).toBeDefined()
  })

  it('WeekView displays current time in the time gutter and turns red on hover', () => {
    const today = new Date()

    render(
      <WeekView
        selectedDate={today}
        events={mockEvents}
        users={mockUsers}
        onEventClick={vi.fn()}
        hourHeight={60}
      />,
    )

    // Current time indicator badge should exist in the gutter
    const nowStr = today.toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit', hour12: false })
    const matchingElements = screen.getAllByText(nowStr)
    const gutterBadge = matchingElements.find((el) => el.className.includes('text-red-500')) || matchingElements[0]
    expect(gutterBadge).toBeDefined()
    expect(gutterBadge.className).toContain('text-red-500/40')

    // Hover over the timeline track
    const lineTrack = screen.getByTestId('timeline-track')
    const lineBar = lineTrack.querySelector('div')
    expect(lineBar?.className).toContain('bg-red-500/30')

    fireEvent.mouseEnter(lineTrack)
    // Turns solid red on hover
    expect(lineBar?.className).toContain('bg-red-500')
    expect(gutterBadge.className).toContain('text-red-600')

    // Leaving reverts to translucent
    fireEvent.mouseLeave(lineTrack)
    expect(lineBar?.className).toContain('bg-red-500/30')
  })

  it('DayView displays current time in the time gutter and turns red on hover', () => {
    const today = new Date()

    render(
      <DayView
        selectedDate={today}
        events={mockEvents}
        users={mockUsers}
        onEventClick={vi.fn()}
        hourHeight={60}
      />,
    )

    // Current time indicator badge should exist in the gutter
    const nowStr = today.toLocaleTimeString('sv-SE', { hour: '2-digit', minute: '2-digit', hour12: false })
    const matchingElements = screen.getAllByText(nowStr)
    const gutterBadge = matchingElements.find((el) => el.className.includes('text-red-500')) || matchingElements[0]
    expect(gutterBadge).toBeDefined()
    expect(gutterBadge.className).toContain('text-red-500/40')

    // Hover over the timeline track
    const lineTrack = screen.getByTestId('timeline-track')
    const lineBar = lineTrack.querySelector('div')
    expect(lineBar?.className).toContain('bg-red-500/30')

    fireEvent.mouseEnter(lineTrack)
    // Turns solid red on hover
    expect(lineBar?.className).toContain('bg-red-500')
    expect(gutterBadge.className).toContain('text-red-600')

    // Leaving reverts to translucent
    fireEvent.mouseLeave(lineTrack)
    expect(lineBar?.className).toContain('bg-red-500/30')
  })

  it('WeekView timeline and gutter time turn red when hovering over a pass (calendar event)', () => {
    const today = new Date()
    const testEvents: CalendarEvent[] = [
      {
        id: 'shift-1',
        title: 'Morgonpass',
        startTime: new Date(today.getFullYear(), today.getMonth(), today.getDate(), 8, 0),
        endTime: new Date(today.getFullYear(), today.getMonth(), today.getDate(), 16, 0),
        category: 'assistance_time',
      },
    ]

    const { container } = render(
      <WeekView
        selectedDate={today}
        events={testEvents}
        users={mockUsers}
        onEventClick={vi.fn()}
        hourHeight={60}
      />,
    )

    // Initially, timeline is translucent (bg-red-500/30) and gutter time has no solid background
    const lineBar = container.querySelector('[data-testid="timeline-track"] > div')
    expect(lineBar?.className).toContain('bg-red-500/30')

    // Find the event card ("pass")
    const eventCards = screen.getAllByText('Morgonpass')
    expect(eventCards.length).toBeGreaterThan(0)
    const eventCard = eventCards[0]

    // Hover over the pass
    const cardEl = eventCard.closest('.group') || eventCard
    fireEvent.mouseEnter(cardEl)

    // The timeline should turn solid red (bg-red-500)
    expect(lineBar?.className).toContain('bg-red-500')
    expect(lineBar?.className).not.toContain('bg-red-500/30')

    // Leaving the pass reverts the timeline to translucent
    fireEvent.mouseLeave(cardEl)
    expect(lineBar?.className).toContain('bg-red-500/30')
  })

  it('CalendarView renders a clean toolbar with navigation and view switcher without bloat', () => {
    const today = new Date(2026, 8, 20)

    render(
      <CalendarView
        selectedDate={today}
        view="week"
        events={mockEvents}
        users={mockUsers}
        onDateChange={vi.fn()}
        onViewChange={vi.fn()}
        onEventClick={vi.fn()}
        onEventUpdate={vi.fn()}
        onNext={vi.fn()}
        onPrevious={vi.fn()}
        onToday={vi.fn()}
      />,
    )

    // The clean toolbar should display the date range and view buttons without extra bloated widgets
    expect(screen.getByText('common.today')).toBeDefined()
    expect(screen.getByText('scheduler.views.week')).toBeDefined()
  })
})

