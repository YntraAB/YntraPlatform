/**
 * This file contains all TypeScript type definitions used throughout the
 * application. Centralizing types ensures consistency and
 * makes the codebase more maintainable.
 */

export interface UserPrivacySettings {
  phone: 'everyone' | 'organization' | 'none'
  location: 'everyone' | 'organization' | 'none'
}

export type UserRole = 'platform_admin' | 'admin' | 'user' | 'assistant' | 'client'

export interface User {
  id: string
  email: string
  name: string
  full_name?: string
  avatar?: string
  phone?: string
  location?: string
  privacy_settings?: UserPrivacySettings
  role: UserRole
  permissions?: Record<string, unknown>
  workspaceId?: string
  client_id?: string | null
  last_sign_in_at?: string
}

export type SocialAuthProvider = 'google' | 'facebook' | 'apple'

export interface AuthState {
  isAuthenticated: boolean
  user: User | null
  isLoading: boolean
  error: string | null
  user_metadata?: Record<string, unknown>
}

export interface CalendarEvent {
  id: string
  title: string
  description?: string
  startTime: Date
  endTime: Date
  category: EventCategory
  location?: string
  isAllDay?: boolean
  color?: string
  attendees?: string[]
  teamId?: string
  assigneeId?: string
  clientId?: string
  waitingTime?: { from: string; to: string }
  activeTimes?: { from: string; to: string }[]
  break?: { from: string; to: string; isPaid: boolean }
  isUnscheduled?: boolean
}

export type EventCategory =
  | 'assistance_time'
  | 'on_call'
  | 'travel_time'
  | 'introduction'
  | 'meeting'
  | 'administrative_hours'
  | 'training'
  | 'escort_service'
  | 'respite_care'
  | 'unauthorized_absence'
  | 'involuntary_leave'
  | 'other_time'
  | 'customer_staff_note'
  | 'severance_pay'
  | 'other'

export interface CategoryConfig {
  id: EventCategory
  label: string
  color: string
  bgColor: string
  icon: string
}

export type CalendarView = 'day' | 'week' | 'month' | 'agenda'

export interface CalendarState {
  selectedDate: Date
  view: CalendarView
  events: CalendarEvent[]
  selectedEvent: CalendarEvent | null
  filterCategories: EventCategory[]
}

export interface NavItem {
  id: string
  label: string
  icon: string
  path?: string
  children?: NavItem[]
  isExpanded?: boolean
  badge?: number
}

export type AppSection = 'dashboard' | 'calendar' | 'tasks' | 'settings' | 'profile'

export interface TimeSlot {
  hour: number
  label: string
}

export interface DayInfo {
  date: Date
  name: string
  dayOfMonth: number
  isToday: boolean
  isWeekend: boolean
}

export interface MiniCalendarDay {
  date: Date
  dayOfMonth: number
  isCurrentMonth: boolean
  isToday: boolean
  hasEvents: boolean
}

export interface WorkspaceModules {
  school?: boolean
  assistance: boolean
  messaging: boolean
  scheduling: boolean
  notes: boolean
  time: boolean
  directory: boolean
  reporting: boolean
}

export interface Workspace {
  id: string
  name: string
  logo_url: string | null
  brand_color: string
  modules_active: WorkspaceModules
  block_settings: Record<string, unknown>
  settings: WorkspaceSettings
  created_at: string
}

export type ReportingStatus = 'pending' | 'reviewed' | 'resolved'

export interface Report {
  id: string
  workspace_id: string
  user_id: string
  type: string
  is_anonymous: boolean
  content: {
    subject: string
    description: string
    date_of_incident: string
  }
  status: ReportingStatus
  created_at: string
}

export interface WorkspaceSettings {
  timezone: string
  week_start: number
  language: string
  business_hours: {
    start: number
    end: number
  }
  default_calendar_view?: 'day' | 'week' | 'month'
}

export interface UserPreferences {
  theme: 'light' | 'dark' | 'system'
  calendar_density: 'compact' | 'relaxed'
  font_scale: number
  accent_color?: string
}

export interface Message {
  id: string
  workspace_id: string
  sender_id: string
  receiver_id?: string | null
  target_team_id?: string | null
  subject: string | null
  body: string | null
  is_read: boolean
  created_at: string
  sender?: User
  receiver?: User
}

export interface Client {
  id: string
  workspace_id: string
  team_id?: string | null
  first_name: string
  last_name: string
  personal_number?: string
  care_level?: string
  message_settings?: {
    allowed_contacts: 'admin_only' | 'contact_person' | 'open'
    contact_person_email?: string
  }
  created_at?: string
}

export interface ClientMedication {
  id: string
  client_id: string
  name: string
  dosage: string
  time_to_take: string
  is_active: boolean
  created_at?: string
}

export interface ClientJournal {
  id: string
  client_id: string
  author_id?: string | null
  type: 'daily' | 'incident' | 'medical'
  content: string
  created_at?: string
}
