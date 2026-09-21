import React, { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router-dom'
import { MiniCalendar } from './MiniCalendar'
import { CalendarView } from './CalendarView'
import { useCalendar } from '@/hooks/useCalendar'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useAuth } from '@/hooks/useAuth'
import { EventModal } from './EventModal'
import { AddEventModal } from './AddEventModal'
import { useWorkspaceTeams, useWorkspaceUsers } from '@/hooks/queries/useWorkspaceData'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Button } from '@/components/ui/button'
import { CalendarClock, PanelLeftClose } from 'lucide-react'
import { Skeleton } from '@/components/ui/skeleton'
import { TimeOffRequestModal } from './TimeOffRequestModal'
import type { CalendarEvent } from '@/types'

export const CalendarPage: React.FC = () => {
  const { workspaceId, isLoading, settings } = useWorkspace()
  const { user } = useAuth()
  const { t } = useTranslation()
  const [isAddModalOpen, setIsAddModalOpen] = React.useState(false)
  const [isTimeOffModalOpen, setIsTimeOffModalOpen] = React.useState(false)
  const [editingEvent, setEditingEvent] = React.useState<CalendarEvent | null>(null)
  const [isLeftPanelOpen, setIsLeftPanelOpen] = React.useState(true)
  const isSv = settings.language === 'sv'
  const activeRole = user?.role || 'assistant'
  const isAdmin = activeRole === 'admin' || activeRole === 'platform_admin'

  const { data: dbTeams = [] } = useWorkspaceTeams(workspaceId || null)
  const { data: dbUsers = [] } = useWorkspaceUsers(workspaceId || null)
  const [searchParams, setSearchParams] = useSearchParams()

  useEffect(() => {
    if (searchParams.get('action') === 'leave_request') {
      setIsTimeOffModalOpen(true)
      // Clear the param so it doesn't reopen on refresh
      const newParams = new URLSearchParams(searchParams)
      newParams.delete('action')
      setSearchParams(newParams, { replace: true })
    }
  }, [searchParams, setSearchParams])

  const {
    selectedDate,
    selectedEndDate,
    view,
    filteredEvents,
    selectedEvent,
    setSelectedDate,
    setView,
    selectEvent,
    navigateNext,
    navigatePrevious,
    navigateToToday,
    deleteEvent,
    addEvent,
    updateEvent,
    selectedTeamId,
    setSelectedTeamId,
    selectedAssigneeId,
    setSelectedAssigneeId,
  } = useCalendar()

  const upcomingEvents = React.useMemo(() => {
    const now = new Date()
    return [...filteredEvents]
      .filter((event) => event.startTime >= now)
      .sort((a, b) => a.startTime.getTime() - b.startTime.getTime())
      .slice(0, 3)
  }, [filteredEvents])

  useEffect(() => {
    if (selectedTeamId === 'all' && dbTeams.length > 0) {
      setSelectedTeamId(dbTeams[0].id)
    }

    if (!isAdmin) {
      if (selectedAssigneeId !== 'all' && selectedAssigneeId !== user?.id) {
        setSelectedAssigneeId(user?.id || 'all')
      }
    }
  }, [
    isAdmin,
    selectedTeamId,
    setSelectedTeamId,
    selectedAssigneeId,
    setSelectedAssigneeId,
    dbTeams,
    user,
  ])

  const handleEventClick = (event: CalendarEvent) => {
    selectEvent(event)
  }

  const handleDeleteEvent = (eventId: string) => {
    deleteEvent(eventId)
    selectEvent(null)
  }

  return (
    <div className="flex h-full flex-1 overflow-hidden">
      {/* Left Panel - Mini Calendar and Filters */}
      {isLeftPanelOpen && (
        <div className="scrollbar-dark w-72 shrink-0 overflow-y-auto border-r border-border bg-sidebar p-4 duration-200 animate-in fade-in">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
              {t('scheduler.overview', 'Översikt')}
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 text-muted-foreground hover:text-foreground"
              onClick={() => setIsLeftPanelOpen(false)}
              title={t('scheduler.hide_panel', 'Dölj panel')}
            >
              <PanelLeftClose className="h-3.5 w-3.5" />
            </Button>
          </div>
          <MiniCalendar
            selectedDate={selectedDate}
            selectedEndDate={selectedEndDate}
            onSelectDate={(date, isShiftClick) => {
              setSelectedDate(date, isShiftClick)
              if (!isShiftClick && view === 'month') {
                setView('day')
              }
            }}
            onNewEvent={() => setIsAddModalOpen(true)}
            datesWithEvents={filteredEvents.map((e) => e.startTime)}
          />

        {isLoading ? (
          <div className="mt-6 border-b border-border pb-6 space-y-2">
            <Skeleton className="h-3.5 w-24" />
            <Skeleton className="h-8 w-full rounded-md" />
          </div>
        ) : dbTeams.length > 1 ? (
          <div className="mt-6 border-b border-border pb-6">
            <h3 className="mb-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
              {t('scheduler.active_schedule')}
            </h3>
            <div className="space-y-2">
              <Select value={selectedTeamId} onValueChange={setSelectedTeamId}>
                <SelectTrigger className="h-8 w-full rounded-md border-border bg-muted text-xs text-foreground">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {dbTeams.map((team) => (
                    <SelectItem key={team.id} value={team.id}>
                      {team.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        ) : null}

        <div className="mt-6 border-b border-border pb-6">
          <h3 className="mb-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
            {t('scheduler.staff_assistants')}
          </h3>
          <div className="space-y-2">
            {isLoading ? (
              <Skeleton className="h-8 w-full rounded-md" />
            ) : (
              <Select value={selectedAssigneeId} onValueChange={setSelectedAssigneeId}>
                <SelectTrigger className="h-8 w-full rounded-md border-border bg-muted text-xs text-foreground">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {isAdmin ? (
                    <>
                      <SelectItem value="all">{t('scheduler.all_assistants')}</SelectItem>
                      {dbUsers.map((u) => (
                        <SelectItem key={u.id} value={u.id}>
                          {u.full_name || u.email || t('common.unknown_agent')}
                        </SelectItem>
                      ))}
                    </>
                  ) : (
                    <>
                      <SelectItem value="all">{t('scheduler.all_assistants')}</SelectItem>
                      <SelectItem value={user?.id || 'all'}>
                        {t('scheduler.only_my_shifts')}
                      </SelectItem>
                    </>
                  )}
                </SelectContent>
              </Select>
            )}
          </div>
        </div>

        <div className="mt-6 border-b border-border pb-6">
          <Button
            variant="outline"
            className="w-full justify-start gap-2 border-primary/20 bg-primary/5 hover:bg-primary/10 hover:text-primary transition-all"
            onClick={() => setIsTimeOffModalOpen(true)}
          >
            <CalendarClock className="h-4 w-4 text-primary" />
            {t('reporting.types.leave_request')}
          </Button>
        </div>

        <div className="mt-6">
          <h3 className="mb-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
            {t('common.upcoming')}
          </h3>
          <div className="space-y-2">
            {isLoading ? (
              [1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-[68px] w-full rounded-lg" />
              ))
            ) : upcomingEvents.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border p-2 text-center text-sm italic text-muted-foreground">
                {t('scheduler.no_upcoming_events')}
              </div>
            ) : (
              upcomingEvents.map((event) => (
                <div
                  key={event.id}
                  onClick={() => handleEventClick(event)}
                  className="cursor-pointer rounded-lg bg-secondary p-3 transition-colors hover:bg-muted"
                >
                  <div className="truncate text-sm font-medium text-foreground">{event.title}</div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {event.startTime.toLocaleDateString(isSv ? 'sv-SE' : 'en-US', {
                      month: 'short',
                      day: 'numeric',
                    })}
                    {' · '}
                    {event.startTime.toLocaleTimeString(isSv ? 'sv-SE' : 'en-US', {
                      hour: '2-digit',
                      minute: '2-digit',
                    })}
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
      )}

      <div className="min-w-0 flex-1">
        <CalendarView
          selectedDate={selectedDate}
          selectedEndDate={selectedEndDate}
          view={view}
          events={filteredEvents}
          users={dbUsers}
          onDateChange={setSelectedDate}
          onViewChange={setView}
          onEventClick={handleEventClick}
          onEventUpdate={updateEvent}
          onNext={navigateNext}
          onPrevious={navigatePrevious}
          onToday={navigateToToday}
          isLeftPanelOpen={isLeftPanelOpen}
          onToggleLeftPanel={() => setIsLeftPanelOpen(!isLeftPanelOpen)}
        />
      </div>

      <EventModal
        event={selectedEvent}
        onClose={() => selectEvent(null)}
        onEdit={(event) => {
          setEditingEvent(event)
          selectEvent(null)
        }}
        onDelete={handleDeleteEvent}
      />

      {(isAddModalOpen || editingEvent) && (
        <AddEventModal
          selectedDate={editingEvent ? editingEvent.startTime : selectedDate}
          event={editingEvent}
          onClose={() => {
            setIsAddModalOpen(false)
            setEditingEvent(null)
          }}
          onSave={async (eventData, eventId) => {
            if (eventId) {
              await updateEvent(eventId, eventData)
            } else {
              await addEvent(eventData)
            }
          }}
        />
      )}

      <TimeOffRequestModal
        isOpen={isTimeOffModalOpen}
        onClose={() => setIsTimeOffModalOpen(false)}
      />
    </div>
  )
}
