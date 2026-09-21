import { describe, it, expect } from 'vitest'
import {
  formatDateInput,
  parseLocalDate,
  isSameDay,
  generateTimeSlots,
  generateWeekDays,
  getStartOfWeek,
} from './utils'

describe('Date and Calendar Utilities', () => {
  it('formatDateInput formats dates as YYYY-MM-DD in local time without UTC day-shift', () => {
    const d = new Date(2026, 8, 20, 0, 30, 0) // Sep 20, 2026 at 00:30 local
    expect(formatDateInput(d)).toBe('2026-09-20')

    const dLate = new Date(2026, 8, 20, 23, 45, 0) // Sep 20, 2026 at 23:45 local
    expect(formatDateInput(dLate)).toBe('2026-09-20')
  })

  it('parseLocalDate correctly parses date and time into local Date', () => {
    const parsed = parseLocalDate('2026-09-20', '14:30')
    expect(parsed.getFullYear()).toBe(2026)
    expect(parsed.getMonth()).toBe(8) // 0-indexed September
    expect(parsed.getDate()).toBe(20)
    expect(parsed.getHours()).toBe(14)
    expect(parsed.getMinutes()).toBe(30)
  })

  it('parseLocalDate handles date without time', () => {
    const parsed = parseLocalDate('2026-09-20')
    expect(parsed.getFullYear()).toBe(2026)
    expect(parsed.getMonth()).toBe(8)
    expect(parsed.getDate()).toBe(20)
    expect(parsed.getHours()).toBe(0)
  })

  it('isSameDay correctly identifies identical calendar days regardless of time', () => {
    const d1 = new Date(2026, 8, 20, 8, 0, 0)
    const d2 = new Date(2026, 8, 20, 22, 30, 0)
    const d3 = new Date(2026, 8, 21, 8, 0, 0)

    expect(isSameDay(d1, d2)).toBe(true)
    expect(isSameDay(d1, d3)).toBe(false)
  })

  it('generateTimeSlots generates full 24-hour slots by default', () => {
    const slots = generateTimeSlots(0, 23)
    expect(slots).toHaveLength(24)
    expect(slots[0].label).toBe('00:00')
    expect(slots[23].label).toBe('23:00')
  })

  it('generateWeekDays creates 7 days starting from week start', () => {
    const weekStart = new Date(2026, 8, 14) // Monday
    const weekDays = generateWeekDays(weekStart, 'sv-SE')
    expect(weekDays).toHaveLength(7)
    expect(weekDays[0].dayOfMonth).toBe(14)
    expect(weekDays[6].dayOfMonth).toBe(20)
  })

  it('getStartOfWeek computes Monday correctly when week_start is 1', () => {
    const wednesday = new Date(2026, 8, 16)
    const monday = getStartOfWeek(wednesday, 1)
    expect(monday.getDay()).toBe(1) // Monday
    expect(monday.getDate()).toBe(14)
  })
})
