/**
 * This custom hook manages calendar state including events, selected date,
 * view mode, and filtering. It provides methods for CRUD operations on events.
 */

import { useState, useCallback, useMemo } from 'react'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'
import { useEvents } from './queries/useEvents'
import type { CalendarEvent, CalendarView, EventCategory } from '@/types'

/**
 * Custom hook for managing calendar state and operations
 *
 * @returns Calendar state and methods for event management
 */
export function useCalendar() {
  const { workspaceId, settings } = useWorkspace()
  const { user } = useAuth()
  const {
    events: dbEvents,
    addEvent: dbAddEvent,
    updateEvent: dbUpdateEvent,
    deleteEvent: dbDeleteEvent,
  } = useEvents(workspaceId, user?.id || null)

  // Core UI state (excluding events which come from TanStack Query)
  const [state, setState] = useState<{
    selectedDate: Date
    selectedEndDate: Date | null
    view: CalendarView
    selectedEvent: CalendarEvent | null
    filterCategories: EventCategory[]
    selectedTeamId: string | 'all'
    selectedAssigneeId: string | 'all'
  }>({
    selectedDate: new Date(),
    selectedEndDate: null,
    view: (settings?.default_calendar_view as CalendarView) || 'week',
    selectedEvent: null,
    filterCategories: [],
    selectedTeamId: 'all',
    selectedAssigneeId: 'all',
  })

  /**
   * Set the currently selected date (or range if shiftClick is true)
   */
  const setSelectedDate = useCallback((date: Date, isShiftClick?: boolean) => {
    setState((prev) => {
      const normalizedDate = new Date(date)
      normalizedDate.setHours(0, 0, 0, 0)

      let newStart = normalizedDate
      let newEnd = null

      if (isShiftClick && prev.selectedDate) {
        const prevNormalized = new Date(prev.selectedDate)
        prevNormalized.setHours(0, 0, 0, 0)

        if (normalizedDate < prevNormalized) {
          newStart = normalizedDate
          newEnd = prevNormalized
        } else {
          newStart = prevNormalized
          newEnd = normalizedDate
        }

        const diffDiff = Math.abs(newEnd.getTime() - newStart.getTime())
        const daysDiff = Math.ceil(diffDiff / (1000 * 60 * 60 * 24))
        if (daysDiff > 14) {
          newEnd = new Date(newStart)
          newEnd.setDate(newStart.getDate() + 14)
        }
      }

      return {
        ...prev,
        selectedDate: newStart,
        selectedEndDate: newEnd,
        view: newEnd ? 'week' : prev.view,
      }
    })
  }, [])

  const setView = useCallback((view: CalendarView) => {
    setState((prev) => ({ ...prev, view, selectedEndDate: null }))
  }, [])

  const setSelectedTeamId = useCallback((teamId: string | 'all') => {
    setState((prev) => ({ ...prev, selectedTeamId: teamId }))
  }, [])

  const setSelectedAssigneeId = useCallback((assigneeId: string | 'all') => {
    setState((prev) => ({ ...prev, selectedAssigneeId: assigneeId }))
  }, [])

  const addEvent = useCallback(
    async (event: Omit<CalendarEvent, 'id'>) => {
      try {
        await dbAddEvent(event)
      } catch (error) {
        console.error('Failed to add event:', error)
      }
    },
    [dbAddEvent],
  )

  const updateEvent = useCallback(
    async (eventId: string, updates: Partial<CalendarEvent>) => {
      try {
        await dbUpdateEvent(eventId, updates)
      } catch (error) {
        console.error('Failed to update event:', error)
      }
    },
    [dbUpdateEvent],
  )

  const deleteEvent = useCallback(
    async (eventId: string) => {
      try {
        await dbDeleteEvent(eventId)
        setState((prev) => ({
          ...prev,
          selectedEvent: prev.selectedEvent?.id === eventId ? null : prev.selectedEvent,
        }))
      } catch (error) {
        console.error('Failed to delete event:', error)
      }
    },
    [dbDeleteEvent],
  )

  const selectEvent = useCallback((event: CalendarEvent | null) => {
    setState((prev) => ({ ...prev, selectedEvent: event }))
  }, [])

  const toggleCategoryFilter = useCallback((category: EventCategory) => {
    setState((prev) => ({
      ...prev,
      filterCategories: prev.filterCategories.includes(category)
        ? prev.filterCategories.filter((c) => c !== category)
        : [...prev.filterCategories, category],
    }))
  }, [])

  const clearFilters = useCallback(() => {
    setState((prev) => ({ ...prev, filterCategories: [] }))
  }, [])

  const navigateNext = useCallback(() => {
    setState((prev) => {
      const newDate = new Date(prev.selectedDate)
      switch (prev.view) {
        case 'day':
          newDate.setDate(newDate.getDate() + 1)
          break
        case 'week':
          newDate.setDate(newDate.getDate() + 7)
          break
        case 'month': {
          const originalDay = newDate.getDate()
          newDate.setMonth(newDate.getMonth() + 1)
          if (newDate.getDate() < originalDay) {
            newDate.setDate(0)
          }
          break
        }
        case 'agenda':
          newDate.setDate(newDate.getDate() + 7)
          break
        default:
          break
      }
      return { ...prev, selectedDate: newDate, selectedEndDate: null }
    })
  }, [])

  const navigatePrevious = useCallback(() => {
    setState((prev) => {
      const newDate = new Date(prev.selectedDate)
      switch (prev.view) {
        case 'day':
          newDate.setDate(newDate.getDate() - 1)
          break
        case 'week':
          newDate.setDate(newDate.getDate() - 7)
          break
        case 'month': {
          const originalDay = newDate.getDate()
          newDate.setMonth(newDate.getMonth() - 1)
          if (newDate.getDate() < originalDay) {
            newDate.setDate(0)
          }
          break
        }
        case 'agenda':
          newDate.setDate(newDate.getDate() - 7)
          break
        default:
          break
      }
      return { ...prev, selectedDate: newDate, selectedEndDate: null }
    })
  }, [])

  const navigateToToday = useCallback(() => {
    setState((prev) => ({ ...prev, selectedDate: new Date(), selectedEndDate: null }))
  }, [])

  /**
   * Filtered events based on data from DB and current UI filters
   */
  const filteredEvents = useMemo(() => {
    let result = dbEvents

    if (state.selectedTeamId !== 'all') {
      result = result.filter((event) => event.teamId === state.selectedTeamId)
    }

    if (state.filterCategories.length > 0) {
      result = result.filter((event) => state.filterCategories.includes(event.category))
    }

    if (state.selectedAssigneeId !== 'all') {
      result = result.filter((event) => event.assigneeId === state.selectedAssigneeId)
    }

    return result
  }, [dbEvents, state.filterCategories, state.selectedTeamId, state.selectedAssigneeId])

  const getEventsForDate = useCallback(
    (date: Date) => {
      const dayStart = new Date(date)
      dayStart.setHours(0, 0, 0, 0)
      const dayEnd = new Date(date)
      dayEnd.setHours(23, 59, 59, 999)

      return filteredEvents.filter((event) => {
        const eventStart = new Date(event.startTime)
        const eventEnd = new Date(event.endTime)

        return eventStart <= dayEnd && eventEnd >= dayStart
      })
    },
    [filteredEvents],
  )

  return {
    // State
    selectedDate: state.selectedDate,
    selectedEndDate: state.selectedEndDate,
    view: state.view,
    events: dbEvents,
    filteredEvents,
    selectedEvent: state.selectedEvent,
    filterCategories: state.filterCategories,
    selectedTeamId: state.selectedTeamId,
    selectedAssigneeId: state.selectedAssigneeId,
    // Actions
    setSelectedDate,
    setView,
    setSelectedTeamId,
    setSelectedAssigneeId,
    addEvent,
    updateEvent,
    deleteEvent,
    selectEvent,
    toggleCategoryFilter,
    clearFilters,
    navigateNext,
    navigatePrevious,
    navigateToToday,
    getEventsForDate,
  }
}
