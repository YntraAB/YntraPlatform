import React, { useState, Suspense, lazy } from 'react'
import { Navigate, useLocation, Outlet, useNavigate, useMatches } from 'react-router-dom'
import { Sidebar } from './features/scheduler/components/Sidebar'
import { LogOut, Settings, ChevronDown, LayoutGrid, Menu } from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Avatar, AvatarFallback } from '@/components/ui/avatar'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Breadcrumb,
  BreadcrumbList,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb'
import { BreadcrumbProvider, useDynamicBreadcrumbs } from '@/contexts/BreadcrumbContext'
import { ErrorBoundary, RouteErrorBoundary } from './components/layout/ErrorBoundary'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { BLOCK_REGISTRY } from '@/lib/blocks/registry'
import type { WorkspaceModules, UserRole } from '@/types'
import type { TFunction } from 'i18next'

interface BreadcrumbHandle {
  breadcrumb: string | ((t: TFunction, data?: unknown) => string)
}

interface BreadcrumbMatch {
  handle: BreadcrumbHandle
  pathname: string
  data: unknown
}

const SettingsPage = lazy(() =>
  import('@/features/settings/components/SettingsPage').then((module) => ({
    default: module.SettingsPage,
  })),
)

import { LoginPage } from './features/auth/components/LoginPage'
import { PageSkeleton } from './components/layout/PageSkeleton'
import type { SocialAuthProvider } from '@/types'

import { ClientLayout } from './features/client-portal/components/ClientLayout'
const ClientHomePage = lazy(() =>
  import('./features/client-portal/components/ClientHomePage').then((module) => ({
    default: module.ClientHomePage,
  })),
)

const LoginPageWrapper: React.FC = () => {
  const { user, logout, loginWithProvider, error } = useAuth()
  const [pendingProvider, setPendingProvider] = useState<SocialAuthProvider | null>(null)
  const navigate = useNavigate()

  const handleLogin = async (provider: SocialAuthProvider) => {
    setPendingProvider(provider)
    const success = await loginWithProvider(provider)
    if (success) {
      navigate('/dashboard')
    }
    setPendingProvider(null)
  }

  const handleLogout = async () => {
    await logout()
  }

  return (
    <LoginPage
      onLogin={handleLogin}
      pendingProvider={pendingProvider}
      error={error}
      currentUser={user}
      onLogout={handleLogout}
      onNavigateToApp={() => navigate('/dashboard')}
    />
  )
}

const RoleGuard: React.FC<{ children: React.ReactNode; allowedRoles: string[] }> = ({
  children,
  allowedRoles,
}) => {
  const { t } = useTranslation()
  const { user } = useAuth()
  const location = useLocation()
  if (!user) return <Navigate to="/" replace />
  if (!allowedRoles.includes(user.role)) {
    const fallback = user.role === 'client' ? '/home' : '/dashboard'
    if (location.pathname === fallback || location.pathname === fallback + '/') {
      return (
        <div className="flex flex-1 flex-col items-center justify-center bg-background p-8">
          <h2 className="mb-2 text-2xl font-bold text-destructive">
            {t('auth.access_denied.title')}
          </h2>
          <p className="text-muted-foreground">
            {t('auth.access_denied.message', { role: user.role })}
          </p>
        </div>
      )
    }
    return <Navigate to={fallback} replace />
  }
  return <>{children}</>
}

const BlockGuard: React.FC<{
  children: React.ReactNode
  blockId: keyof WorkspaceModules | 'dashboard'
}> = ({ children, blockId }) => {
  const { modules } = useWorkspace()
  const { t } = useTranslation()

  if (blockId !== 'dashboard' && !modules[blockId]) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center bg-background p-8 text-center">
        <div className="mb-6 rounded-lg bg-muted/50 p-6">
          <LayoutGrid className="mx-auto h-12 w-12 text-muted-foreground/30" />
        </div>
        <h2 className="mb-2 text-2xl font-bold tracking-tight text-foreground">
          {t('settings.blocks.disabled_title', 'Feature Block Disabled')}
        </h2>
        <p className="max-w-md text-muted-foreground">
          {t(
            'settings.blocks.disabled_message',
            'This functional block is currently deactivated for your workspace. Please contact your administrator to enable it.',
          )}
        </p>
      </div>
    )
  }
  return <>{children}</>
}

import { OfflineIndicator } from './components/layout/OfflineIndicator'
import { GlobalSearch } from './components/GlobalSearch'

const AppLayoutWrapper: React.FC = () => {
  const { user } = useAuth()
  if (user?.role === 'client') {
    return (
      <ErrorBoundary>
        <BreadcrumbProvider>
          <GlobalSearch />
          <ClientLayout />
          <OfflineIndicator />
        </BreadcrumbProvider>
      </ErrorBoundary>
    )
  }
  return (
    <ErrorBoundary>
      <BreadcrumbProvider>
        <GlobalSearch />
        <Layout />
        <OfflineIndicator />
      </BreadcrumbProvider>
    </ErrorBoundary>
  )
}

const RootRedirect: React.FC = () => {
  const { user } = useAuth()
  if (user?.role === 'client') return <Navigate to="/home" replace />
  return <Navigate to="/dashboard" replace />
}

const HeaderBreadcrumbs: React.FC = () => {
  const { t } = useTranslation()
  const location = useLocation()
  const navigate = useNavigate()
  const matches = useMatches()
  const dynamicBreadcrumbs = useDynamicBreadcrumbs()

  const activeSection = location.pathname.substring(1) || 'dashboard'

  const baseBreadcrumbs: import('@/contexts/BreadcrumbContext').DynamicBreadcrumbItem[] = (
    matches as unknown as BreadcrumbMatch[]
  )
    .filter((match) => match.handle && match.handle.breadcrumb)
    .map((match) => ({
      label:
        typeof match.handle.breadcrumb === 'function'
          ? match.handle.breadcrumb(t, match.data)
          : match.handle.breadcrumb,
      path: match.pathname,
    }))

  const breadcrumbs = [...baseBreadcrumbs, ...dynamicBreadcrumbs]

  return (
    <div className="flex items-center gap-1.5 duration-200 animate-in fade-in slide-in-from-left-2">
      <Breadcrumb>
        <BreadcrumbList>
          {breadcrumbs.length > 0 ? (
            breadcrumbs.map((bc, idx) => (
              <React.Fragment key={`${bc.label}-${idx}`}>
                <BreadcrumbItem>
                  {idx === breadcrumbs.length - 1 ? (
                    <BreadcrumbPage>{bc.label}</BreadcrumbPage>
                  ) : (
                    <BreadcrumbLink asChild>
                      <span
                        className="cursor-pointer"
                        onClick={() => {
                          if (bc.onClick) {
                            bc.onClick()
                          } else if (bc.path) {
                            window.dispatchEvent(
                              new CustomEvent('yntra:breadcrumb-navigate', {
                                detail: { path: bc.path },
                              }),
                            )
                            navigate(bc.path)
                          }
                        }}
                      >
                        {bc.label}
                      </span>
                    </BreadcrumbLink>
                  )}
                </BreadcrumbItem>
                {idx < breadcrumbs.length - 1 && <BreadcrumbSeparator />}
              </React.Fragment>
            ))
          ) : (
            <BreadcrumbItem>
              <BreadcrumbPage className="capitalize">
                {t(
                  `sidebar.sections.${activeSection === 'notes' ? 'notes' : activeSection.replace('-', '')}`,
                  activeSection.replace('-', ' '),
                )}
              </BreadcrumbPage>
            </BreadcrumbItem>
          )}
        </BreadcrumbList>
      </Breadcrumb>
    </div>
  )
}

export const Layout: React.FC = () => {
  const [mobileOpen, setMobileOpen] = useState(false)
  const { user, logout, isPlatformAdmin, simulateRole } = useAuth()
  const { t } = useTranslation()
  const location = useLocation()
  const navigate = useNavigate()

  const userName = user?.name || ''
  const activeSection = location.pathname.substring(1) || 'dashboard'

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      {/* Desktop Sidebar */}
      <Sidebar
        className="hidden md:flex"
        activeSection={activeSection}
        onSectionChange={(path) => navigate(`/${path}`)}
      />

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-12 items-center justify-between border-b border-border/40 bg-background/80 px-4 backdrop-blur-md">
          <div className="flex items-center gap-2 text-[13px] text-muted-foreground">
            {/* Mobile Sheet Trigger */}
            <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="h-8 w-8 md:hidden">
                  <Menu className="h-4 w-4" />
                </Button>
              </SheetTrigger>
              <SheetContent side="left" className="w-64 p-0">
                <Sidebar
                  activeSection={activeSection}
                  onSectionChange={(path) => {
                    navigate(`/${path}`)
                    setMobileOpen(false)
                  }}
                  onNavigate={() => setMobileOpen(false)}
                />
              </SheetContent>
            </Sheet>

            <HeaderBreadcrumbs />
          </div>

          <div className="flex items-center gap-3">
            {isPlatformAdmin && (
              <Select value={user?.role || ''} onValueChange={(val) => simulateRole(val as UserRole)}>
                <SelectTrigger className="h-8 w-[120px] rounded-lg border-border bg-secondary/50 text-[11px] font-bold shadow-none duration-300 animate-in fade-in zoom-in focus:ring-0 focus:ring-offset-0">
                  <SelectValue placeholder={t('auth.role_simulator.placeholder')} />
                </SelectTrigger>
                <SelectContent align="end" className="border-border bg-sidebar">
                  <SelectItem value="platform_admin">
                    {t('auth.role_simulator.platform_admin')}
                  </SelectItem>
                  <SelectItem value="admin">{t('auth.role_simulator.admin')}</SelectItem>
                  <SelectItem value="assistant">{t('auth.role_simulator.assistant')}</SelectItem>
                  <SelectItem value="user">{t('auth.role_simulator.user')}</SelectItem>
                  <SelectItem value="client">{t('auth.role_simulator.client')}</SelectItem>
                </SelectContent>
              </Select>
            )}

            {/* Profile Dropdown */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button className="flex cursor-pointer items-center gap-2.5 rounded-md px-2 py-1 transition-colors hover:bg-secondary focus:outline-none">
                  <div className="hidden text-right md:block">
                    <div className="text-xs font-semibold leading-tight text-foreground">
                      {userName}
                    </div>
                    <div className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                      {user?.role === 'platform_admin'
                        ? t('auth.role_simulator.platform_admin')
                        : user?.role || t('auth.role_simulator.admin')}
                    </div>
                  </div>
                  <Avatar className="h-7 w-7 border border-border">
                    <AvatarFallback className="bg-secondary text-xs font-semibold text-foreground">
                      {userName.charAt(0).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                  <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-48">
                {user?.role !== 'client' && (
                  <>
                    <DropdownMenuItem
                      onClick={() => navigate('/settings')}
                      className="cursor-pointer gap-2"
                    >
                      <Settings className="h-4 w-4" />
                      <span>{t('common.settings')}</span>
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                  </>
                )}
                <DropdownMenuItem
                  onClick={logout}
                  className="cursor-pointer gap-2 text-destructive focus:bg-destructive/10 focus:text-destructive"
                >
                  <LogOut className="h-4 w-4" />
                  <span>{t('common.logout')}</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </header>
        <main className="min-h-0 flex-1 overflow-y-auto">
          <Suspense fallback={<PageSkeleton />}>
            <Outlet />
          </Suspense>
        </main>
      </div>
    </div>
  )
}

export const routes = [
  {
    path: '/login',
    element: <LoginPageWrapper />,
    errorElement: <RouteErrorBoundary />,
  },
  {
    path: '/',
    element: <AppLayoutWrapper />,
    errorElement: <RouteErrorBoundary />,
    children: [
      { index: true, element: <RootRedirect /> },
      {
        path: 'home',
        element: (
          <RoleGuard allowedRoles={['client']}>
            <ClientHomePage />
          </RoleGuard>
        ),
        errorElement: <RouteErrorBoundary />,
        handle: { breadcrumb: (t: TFunction) => t('common.home') },
      },
      {
        path: 'settings',
        element: (
          <RoleGuard allowedRoles={['platform_admin', 'admin', 'user', 'assistant']}>
            <SettingsPage />
          </RoleGuard>
        ),
        errorElement: <RouteErrorBoundary />,
        handle: { breadcrumb: (t: TFunction) => t('common.settings') },
      },
      ...Object.values(BLOCK_REGISTRY).flatMap((block) =>
        block.routes.map((route) => ({
          path: route.path,
          element: (
            <RoleGuard allowedRoles={route.allowedRoles}>
              <BlockGuard blockId={block.id as keyof WorkspaceModules}>
                <route.component />
              </BlockGuard>
            </RoleGuard>
          ),
          errorElement: <RouteErrorBoundary />,
          handle: { breadcrumb: (t: TFunction) => t(route.breadcrumbKey) },
        })),
      ),
    ],
  },
]
