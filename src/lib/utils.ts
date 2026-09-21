/**
 * This file contains utility functions used throughout the application.
 * These include date formatting, class name merging, and other helper
 * functions.
 */

import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'
import type { EventCategory, CategoryConfig, TimeSlot, DayInfo } from '@/types'

/**
 * Merge Tailwind CSS classes with proper precedence
 * Uses clsx for conditional classes and tailwind-merge to resolve conflicts
 *
 * @param inputs - Class values to merge
 * @returns Merged class string
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Format a date to a localized string
 *
 * @param date - Date to format
 * @param options - Intl.DateTimeFormatOptions
 * @returns Formatted date string
 */
export function formatDate(
  date: Date,
  options: Intl.DateTimeFormatOptions = {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
  },
  locale: string = 'en-US',
): string {
  return new Intl.DateTimeFormat(locale, options).format(date)
}

/**
 * Format a date to show month and year (e.g., "February 2025")
 *
 * @param date - Date to format
 * @returns Formatted month and year string
 */
export function formatMonthYear(date: Date, locale: string = 'en-US'): string {
  return new Intl.DateTimeFormat(locale, {
    month: 'long',
    year: 'numeric',
  }).format(date)
}

/**
 * Format time from a Date object (e.g., "09:00")
 *
 * @param date - Date to extract time from
 * @returns Formatted time string (24-hour format)
 */
export function formatTime(date: Date, locale: string = 'en-US'): string {
  return new Intl.DateTimeFormat(locale, {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(date)
}

/**
 * Format a Date object to 'YYYY-MM-DD' using local timezone
 * (immune to the UTC day-shift bug of date.toISOString().split('T')[0])
 */
export function formatDateInput(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

/**
 * Parse a 'YYYY-MM-DD' date string and optional 'HH:mm' time string into a local Date
 * (immune to UTC date constructor timezone shifting)
 */
export function parseLocalDate(dateStr: string, timeStr?: string): Date {
  const [year, month, day] = dateStr.split('-').map(Number)
  if (timeStr) {
    const [hours, minutes] = timeStr.split(':').map(Number)
    return new Date(year, month - 1, day, hours || 0, minutes || 0, 0, 0)
  }
  return new Date(year, month - 1, day, 0, 0, 0, 0)
}

/**
 * Format time range (e.g., "09:00 - 10:30")
 *
 * @param startTime - Start date/time
 * @param endTime - End date/time
 * @returns Formatted time range string
 */
export function formatTimeRange(startTime: Date, endTime: Date, locale: string = 'en-US'): string {
  return `${formatTime(startTime, locale)} - ${formatTime(endTime, locale)}`
}

/**
 * Get short day name (e.g., "Mon", "Tue")
 *
 * @param date - Date to get day name from
 * @returns Short day name
 */
export function getShortDayName(date: Date, locale: string = 'en-US'): string {
  return new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(date)
}

/**
 * Get full day name (e.g., "Monday")
 *
 * @param date - Date to get day name from
 * @returns Full day name
 */
export function getFullDayName(date: Date, locale: string = 'en-US'): string {
  return new Intl.DateTimeFormat(locale, { weekday: 'long' }).format(date)
}

/**
 * Check if two dates are the same day
 *
 * @param date1 - First date
 * @param date2 - Second date
 * @returns True if same day
 */
export function isSameDay(date1: Date, date2: Date): boolean {
  return (
    date1.getDate() === date2.getDate() &&
    date1.getMonth() === date2.getMonth() &&
    date1.getFullYear() === date2.getFullYear()
  )
}

/**
 * Check if a date is today
 *
 * @param date - Date to check
 * @returns True if today
 */
export function isToday(date: Date): boolean {
  return isSameDay(date, new Date())
}

/**
 * -----------------------------------------------------------------------------
 * CALENDAR GRID UTILITIES
 * -----------------------------------------------------------------------------
 */

/**
 * Generate time slots for the calendar grid
 * Creates slots from 00:00 to 23:00
 *
 * @returns Array of time slot objects
 */
export function generateTimeSlots(startHour: number = 0, endHour: number = 23): TimeSlot[] {
  const slots: TimeSlot[] = []
  for (let hour = startHour; hour <= endHour; hour++) {
    slots.push({
      hour,
      label: `${hour.toString().padStart(2, '0')}:00`,
    })
  }
  return slots
}

/**
 * Generate days for a week view
 *
 * @param startDate - Start date of the week (typically Sunday or Monday)
 * @returns Array of day info objects
 */
export function generateWeekDays(startDate: Date, locale: string = 'en-US'): DayInfo[] {
  const days: DayInfo[] = []
  const currentDate = new Date(startDate)

  // StartDate is already presumed to be the "start of week" from getStartOfWeek

  for (let i = 0; i < 7; i++) {
    const date = new Date(currentDate)
    date.setDate(date.getDate() + i)

    const dayOfWeek = date.getDay()
    const isWeekend = dayOfWeek === 0 || dayOfWeek === 6

    days.push({
      date,
      name: getShortDayName(date, locale),
      dayOfMonth: date.getDate(),
      isToday: isToday(date),
      isWeekend: isWeekend,
    })
  }

  return days
}

/**
 * Get the start of the week for a given date
 *
 * @param date - Date to find week start for
 * @returns Date representing start of week (Monday)
 */
export function getStartOfWeek(date: Date, weekStart: number = 1): Date {
  const result = new Date(date)
  const dayOfWeek = result.getDay()

  const diff = (dayOfWeek < weekStart ? 7 : 0) + dayOfWeek - weekStart

  result.setDate(result.getDate() - diff)
  result.setHours(0, 0, 0, 0)
  return result
}

/**
 * Configuration for event categories including colors and labels
 */
export const EVENT_CATEGORIES: Record<EventCategory, CategoryConfig> = {
  assistance_time: {
    id: 'assistance_time',
    label: 'scheduler.categories.assistance_time',
    color: '#3B82F6', // Blue
    bgColor: 'rgba(59, 130, 246, 0.2)',
    icon: 'User',
  },
  on_call: {
    id: 'on_call',
    label: 'scheduler.categories.on_call',
    color: '#F59E0B', // Amber
    bgColor: 'rgba(245, 158, 11, 0.2)',
    icon: 'Radio',
  },
  travel_time: {
    id: 'travel_time',
    label: 'scheduler.categories.travel_time',
    color: '#10B981', // Emerald
    bgColor: 'rgba(16, 185, 129, 0.2)',
    icon: 'Clock',
  },
  introduction: {
    id: 'introduction',
    label: 'scheduler.categories.introduction',
    color: '#8B5CF6', // Violet
    bgColor: 'rgba(139, 92, 246, 0.2)',
    icon: 'BookOpen',
  },
  meeting: {
    id: 'meeting',
    label: 'scheduler.categories.meeting',
    color: '#EC4899', // Pink
    bgColor: 'rgba(236, 72, 153, 0.2)',
    icon: 'Users',
  },
  administrative_hours: {
    id: 'administrative_hours',
    label: 'scheduler.categories.administrative_hours',
    color: '#6366F1', // Indigo
    bgColor: 'rgba(99, 102, 241, 0.2)',
    icon: 'FileText',
  },
  training: {
    id: 'training',
    label: 'scheduler.categories.training',
    color: '#F97316', // Orange
    bgColor: 'rgba(249, 115, 22, 0.2)',
    icon: 'GraduationCap',
  },
  escort_service: {
    id: 'escort_service',
    label: 'scheduler.categories.escort_service',
    color: '#06B6D4', // Cyan
    bgColor: 'rgba(6, 182, 212, 0.2)',
    icon: 'Accessibility',
  },
  respite_care: {
    id: 'respite_care',
    label: 'scheduler.categories.respite_care',
    color: '#14B8A6', // Teal
    bgColor: 'rgba(20, 184, 166, 0.2)',
    icon: 'Home',
  },
  unauthorized_absence: {
    id: 'unauthorized_absence',
    label: 'scheduler.categories.unauthorized_absence',
    color: '#EF4444', // Red
    bgColor: 'rgba(239, 68, 68, 0.2)',
    icon: 'XCircle',
  },
  involuntary_leave: {
    id: 'involuntary_leave',
    label: 'scheduler.categories.involuntary_leave',
    color: '#F43F5E', // Rose
    bgColor: 'rgba(244, 63, 94, 0.2)',
    icon: 'AlertCircle',
  },
  other_time: {
    id: 'other_time',
    label: 'scheduler.categories.other_time',
    color: '#84CC16', // Lime
    bgColor: 'rgba(132, 204, 22, 0.2)',
    icon: 'Plus',
  },
  customer_staff_note: {
    id: 'customer_staff_note',
    label: 'scheduler.categories.customer_staff_note',
    color: '#D946EF', // Fuchsia
    bgColor: 'rgba(217, 70, 239, 0.2)',
    icon: 'MessageSquare',
  },
  severance_pay: {
    id: 'severance_pay',
    label: 'scheduler.categories.severance_pay',
    color: '#6B7280', // Gray
    bgColor: 'rgba(107, 114, 128, 0.2)',
    icon: 'Banknote',
  },
  other: {
    id: 'other',
    label: 'scheduler.categories.other',
    color: '#9CA3AF', // Gray-400
    bgColor: 'rgba(156, 163, 175, 0.2)',
    icon: 'MoreHorizontal',
  },
}

/**
 * Get category configuration by category ID
 *
 * @param category - Event category
 * @returns Category configuration object
 */
export function getCategoryConfig(category: EventCategory): CategoryConfig {
  return EVENT_CATEGORIES[category] || EVENT_CATEGORIES['other']
}

/**
 * Calculate the top position for an event in the calendar grid
 * Based on the hour and minute of the start time
 *
 * @param date - Event start time
 * @param hourHeight - Height of one hour in pixels (default: 60)
 * @returns Top position in pixels
 */
export function calculateEventTop(
  date: Date,
  hourHeight: number = 60,
  startHour: number = 0,
): number {
  const hour = date.getHours()
  const minute = date.getMinutes()
  return (hour - startHour + minute / 60) * hourHeight
}

/**
 * Calculate the height of an event based on its duration
 *
 * @param startTime - Event start time
 * @param endTime - Event end time
 * @param hourHeight - Height of one hour in pixels (default: 60)
 * @returns Height in pixels
 */
export function calculateEventHeight(
  startTime: Date,
  endTime: Date,
  hourHeight: number = 60,
): number {
  const durationMs = endTime.getTime() - startTime.getTime()
  const durationHours = durationMs / (1000 * 60 * 60)
  return durationHours * hourHeight
}
