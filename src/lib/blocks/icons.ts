import {
  MessageSquare,
  Calendar,
  FileText,
  Clock,
  Heart,
  Users,
  LayoutGrid,
  Settings,
  HelpCircle,
  AlertTriangle
} from 'lucide-react'

export type IconName =
  | 'MessageSquare'
  | 'Calendar'
  | 'FileText'
  | 'Clock'
  | 'Heart'
  | 'Users'
  | 'LayoutGrid'
  | 'Settings'
  | 'HelpCircle'
  | 'AlertTriangle'

export const ICON_MAP: Record<IconName, any> = {
  MessageSquare,
  Calendar,
  FileText,
  Clock,
  Heart,
  Users,
  LayoutGrid,
  Settings,
  HelpCircle,
  AlertTriangle
}
