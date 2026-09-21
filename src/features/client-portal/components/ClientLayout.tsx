import React, { useState, Suspense } from 'react'
import { useLocation, Outlet, useNavigate } from 'react-router-dom'
import { LogOut, ChevronDown, Menu } from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Avatar, AvatarFallback } from '@/components/ui/avatar'
import { ClientPageSkeleton } from '@/components/layout/ClientPageSkeleton'

export const ClientLayout: React.FC = () => {
  const [mobileOpen, setMobileOpen] = useState(false)
  const { user, logout } = useAuth()
  const { t } = useTranslation()
  const location = useLocation()
  const navigate = useNavigate()

  const userName = user?.name || ''
  const activeSection = location.pathname.substring(1) || 'home'

  const handleNav = (path: string) => {
    navigate(path)
    setMobileOpen(false)
  }

  const renderNavButtons = () => (
    <div className="flex flex-1 flex-col gap-1 overflow-y-auto px-3 py-4">
      <button
        onClick={() => handleNav('/home')}
        className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm transition-all duration-150 ${activeSection === 'home' ? 'bg-secondary font-medium text-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}`}
      >
        {t('common.home')}
      </button>
      <button
        onClick={() => handleNav('/inbox')}
        className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm transition-all duration-150 ${activeSection === 'inbox' ? 'bg-secondary font-medium text-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}`}
      >
        {t('sidebar.sections.inbox')}
      </button>
      <button
        onClick={() => handleNav('/directory')}
        className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm transition-all duration-150 ${activeSection === 'directory' ? 'bg-secondary font-medium text-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}`}
      >
        {t('sidebar.teams')}
      </button>
    </div>
  )

  return (
    <div className="relative flex h-screen overflow-hidden bg-background">
      {/* Desktop Sidebar */}
      <aside className="z-20 hidden w-64 flex-col border-r border-border bg-sidebar md:flex">
        <div className="flex h-14 items-center border-b border-border bg-sidebar px-4">
          <span className="text-sm font-bold tracking-tight text-foreground">
            {t('client_portal.title')}
          </span>
        </div>
        {renderNavButtons()}
        <div className="mt-auto border-t border-border p-3">
          <Button
            variant="ghost"
            size="sm"
            onClick={logout}
            className="w-full justify-start gap-2 text-destructive hover:bg-destructive/10 hover:text-destructive"
          >
            <LogOut className="h-4 w-4" /> {t('common.logout')}
          </Button>
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="sticky top-0 z-10 flex h-12 items-center justify-between border-b border-border/40 bg-background/80 px-3 backdrop-blur-md md:h-14 md:px-4">
          <div className="flex items-center gap-2 text-sm font-medium capitalize text-foreground">
            {/* Mobile Sheet Trigger */}
            <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="h-8 w-8 md:hidden">
                  <Menu className="h-4 w-4" />
                </Button>
              </SheetTrigger>
              <SheetContent side="left" className="w-64 p-0 bg-sidebar">
                <div className="flex h-14 items-center border-b border-border px-4">
                  <span className="text-sm font-bold tracking-tight text-foreground">
                    {t('client_portal.title')}
                  </span>
                </div>
                {renderNavButtons()}
                <div className="mt-auto border-t border-border p-3">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={logout}
                    className="w-full justify-start gap-2 text-destructive hover:bg-destructive/10 hover:text-destructive"
                  >
                    <LogOut className="h-4 w-4" /> {t('common.logout')}
                  </Button>
                </div>
              </SheetContent>
            </Sheet>

            <span>
              {activeSection === 'home'
                ? t('client_portal.sections.start')
                : t(`sidebar.sections.${activeSection}`)}
            </span>
          </div>

          <div className="flex items-center gap-3">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button className="flex cursor-pointer items-center gap-2.5 rounded-md px-2 py-1 transition-colors hover:bg-secondary focus:outline-none">
                  <div className="hidden text-right md:block">
                    <div className="text-xs font-semibold leading-tight text-foreground">
                      {userName}
                    </div>
                    <div className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                      {t('client_portal.account')}
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

        <Suspense fallback={<ClientPageSkeleton />}>
          <div className="h-full w-full flex-1 overflow-y-auto p-4 md:p-6">
            <Outlet />
          </div>
        </Suspense>
      </div>
    </div>
  )
}
