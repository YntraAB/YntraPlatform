import React, { useState } from 'react'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { useTranslation } from 'react-i18next'
import {
  Globe,
  Mail,
  User as UserIcon,
  Calendar,
  LayoutGrid,
} from 'lucide-react'

import { GeneralSettings } from './GeneralSettings'
import { NotificationsSettings } from './NotificationsSettings'
import { AccountSettings } from './AccountSettings'
import { SchedulerSettings } from './SchedulerSettings'
import { BlockSettings } from './BlockSettings'

import { useSearchParams } from 'react-router-dom'
import { useAuth } from '@/hooks/useAuth'
import { cn } from '@/lib/utils'
import { useSetDynamicBreadcrumbs } from '@/contexts/BreadcrumbContext'

interface NavTabItem {
  id: string
  label: string
  icon: React.ElementType
  adminOnly?: boolean
}

export const SettingsPage: React.FC = () => {
  const { workspaceName, isLoading } = useWorkspace()
  const { t } = useTranslation()
  const { user } = useAuth()
  const [searchParams, setSearchParams] = useSearchParams()
  const [_isSaving, setIsSaving] = useState(false)

  const isAdmin = user?.role === 'admin' || user?.role === 'platform_admin'
  const defaultTab = isAdmin ? 'general' : 'account'
  const activeTab = searchParams.get('tab') || defaultTab

  const handleTabChange = (val: string) => {
    setSearchParams({ tab: val })
  }

  const NAV_ITEMS: NavTabItem[] = [
    {
      id: 'general',
      label: t('settings.tabs.general'),
      icon: Globe,
      adminOnly: true,
    },
    {
      id: 'account',
      label: t('settings.tabs.account'),
      icon: UserIcon,
    },
    {
      id: 'scheduler',
      label: t('settings.tabs.scheduler'),
      icon: Calendar,
      adminOnly: true,
    },
    {
      id: 'blocks',
      label: t('settings.tabs.blocks', 'Blocks'),
      icon: LayoutGrid,
      adminOnly: true,
    },
    {
      id: 'notifications',
      label: t('settings.tabs.notifications'),
      icon: Mail,
    },
  ].filter((item) => !item.adminOnly || isAdmin)

  const currentTabItem = NAV_ITEMS.find((item) => item.id === activeTab)
  const setDynamicBreadcrumbs = useSetDynamicBreadcrumbs()
  const currentTabLabel = currentTabItem?.label

  React.useEffect(() => {
    if (currentTabLabel) {
      setDynamicBreadcrumbs([
        {
          label: currentTabLabel,
        },
      ])
    }
  }, [currentTabLabel, setDynamicBreadcrumbs])

  React.useEffect(() => {
    return () => setDynamicBreadcrumbs([])
  }, [setDynamicBreadcrumbs])

  React.useEffect(() => {
    const handleReset = (e: Event) => {
      const customEvent = e as CustomEvent<{ path: string }>
      if (customEvent.detail?.path === '/settings') {
        setSearchParams({ tab: defaultTab })
      }
    }
    window.addEventListener('yntra:breadcrumb-navigate', handleReset)
    return () => window.removeEventListener('yntra:breadcrumb-navigate', handleReset)
  }, [defaultTab, setSearchParams])

  return (
    <div className="flex h-full w-full flex-col bg-background">
      {/* Sticky Clean Header - h-12 standard */}
      <div className="sticky top-0 z-10 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/80 px-6 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <h1 className="text-xs font-medium tracking-tight text-foreground">
            {t('common.settings')}
          </h1>
          <span className="text-muted-foreground/40">•</span>
          <span className="truncate text-xs font-normal text-muted-foreground">
            {isAdmin ? workspaceName || 'Yntra Platform' : user?.name || t('settings.account.profile')}
          </span>
        </div>
      </div>

      {/* Main Settings Body */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Sub-Navigation Sidebar (Desktop) */}
        <nav className="hidden w-56 shrink-0 border-r border-border/40 p-4 sm:block overflow-y-auto">
          <div className="space-y-0.5">
            {NAV_ITEMS.map((item) => {
              const Icon = item.icon
              const isActive = activeTab === item.id
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => handleTabChange(item.id)}
                  className={cn(
                    'group flex h-[34px] w-full cursor-pointer items-center gap-2.5 rounded-lg px-3 text-[13px] transition-colors duration-150 text-left',
                    isActive
                      ? 'bg-secondary/80 font-medium text-foreground'
                      : 'text-muted-foreground hover:bg-secondary/40 hover:text-foreground',
                  )}
                >
                  <Icon
                    className={cn(
                      'h-3.5 w-3.5 shrink-0 transition-colors',
                      isActive ? 'text-foreground' : 'text-muted-foreground/70 group-hover:text-foreground',
                    )}
                  />
                  <span className="flex-1 truncate">{item.label}</span>
                </button>
              )
            })}
          </div>
        </nav>

        {/* Content Pane */}
        <div className="flex-1 overflow-y-auto px-6 py-6 lg:px-10">
          {/* Mobile Horizontal Tabs */}
          <div className="scrollbar-none mb-6 flex gap-1.5 overflow-x-auto border-b border-border/40 pb-2.5 sm:hidden">
            {NAV_ITEMS.map((item) => {
              const Icon = item.icon
              const isActive = activeTab === item.id
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => handleTabChange(item.id)}
                  className={cn(
                    'flex h-8 shrink-0 items-center gap-2 rounded-md px-3 text-xs transition-colors',
                    isActive
                      ? 'bg-secondary font-medium text-foreground'
                      : 'text-muted-foreground hover:bg-secondary/40 hover:text-foreground',
                  )}
                >
                  <Icon className="h-3.5 w-3.5" />
                  <span>{item.label}</span>
                </button>
              )
            })}
          </div>

          <div className="mx-auto max-w-3xl pb-16">
            {isLoading ? (
              <div className="space-y-4">
                <div className="h-28 w-full animate-pulse rounded-lg border border-border/40 bg-muted/20" />
                <div className="h-44 w-full animate-pulse rounded-lg border border-border/40 bg-muted/20" />
              </div>
            ) : (
              <>
                {activeTab === 'general' && isAdmin && (
                  <GeneralSettings setIsSaving={setIsSaving} />
                )}
                {activeTab === 'account' && (
                  <AccountSettings setIsSaving={setIsSaving} />
                )}
                {activeTab === 'scheduler' && isAdmin && (
                  <SchedulerSettings setIsSaving={setIsSaving} />
                )}
                {activeTab === 'blocks' && isAdmin && (
                  <BlockSettings setIsSaving={setIsSaving} />
                )}
                {activeTab === 'notifications' && (
                  <NotificationsSettings setIsSaving={setIsSaving} />
                )}
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

export default SettingsPage
