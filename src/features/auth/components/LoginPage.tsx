import React from 'react'
import { ArrowRight, Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { SocialAuthProvider } from '@/types'

import { isDemoMode } from '@/lib/demo'

interface LoginPageProps {
  onLogin: (provider: SocialAuthProvider) => void
  pendingProvider?: SocialAuthProvider | null
  error?: string | null
  currentUser?: { name?: string | null; email?: string; role?: string } | null
  onLogout?: () => void
  onNavigateToApp?: () => void
}

const YntraMark: React.FC<{ className?: string }> = ({ className = 'h-7 w-7' }) => (
  <svg
    viewBox="0 0 48 48"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
    aria-hidden="true"
  >
    <path
      d="M28 4L12 24H22L18 44L36 20H24L28 4Z"
      fill="currentColor"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinejoin="miter"
    />
  </svg>
)

const providerIcon: Record<SocialAuthProvider, React.ReactNode> = {
  google: (
    <svg viewBox="0 0 24 24" aria-hidden="true" className="h-4 w-4">
      <path
        fill="#EA4335"
        d="M12 10.2v3.9h5.5c-.2 1.2-.9 2.2-1.9 2.9l3.1 2.4c1.8-1.7 2.8-4.1 2.8-6.9 0-.7-.1-1.5-.2-2.2H12Z"
      />
      <path
        fill="#34A853"
        d="M12 21c2.5 0 4.6-.8 6.1-2.2L15 16.4c-.8.5-1.8.8-3 .8-2.3 0-4.3-1.6-5-3.8H3.8v2.5A9.2 9.2 0 0 0 12 21Z"
      />
      <path fill="#FBBC05" d="M7 13.4a5.5 5.5 0 0 1 0-3.4V7.5H3.8a9.2 9.2 0 0 0 0 8.4L7 13.4Z" />
      <path
        fill="#4285F4"
        d="M12 6.8c1.3 0 2.4.4 3.3 1.3l2.5-2.5A9 9 0 0 0 12 3 9.2 9.2 0 0 0 3.8 7.5L7 10c.7-2.2 2.7-3.2 5-3.2Z"
      />
    </svg>
  ),
  facebook: (
    <svg viewBox="0 0 24 24" aria-hidden="true" className="h-4 w-4">
      <path
        fill="#1877F2"
        d="M24 12a12 12 0 1 0-13.9 11.9v-8.4H7.1V12h3V9.4c0-3 1.8-4.7 4.5-4.7 1.3 0 2.7.2 2.7.2v3h-1.5c-1.5 0-1.9.9-1.9 1.8V12h3.3l-.5 3.5H14v8.4A12 12 0 0 0 24 12Z"
      />
    </svg>
  ),
  apple: (
    <svg viewBox="0 0 24 24" aria-hidden="true" className="h-4 w-4 fill-current">
      <path d="M16.7 12.8c0-2.4 2-3.5 2.1-3.6-1.1-1.7-2.9-1.9-3.5-1.9-1.5-.2-2.9.9-3.6.9-.8 0-1.9-.9-3.1-.8-1.6 0-3.1.9-3.9 2.3-1.7 2.9-.4 7.2 1.2 9.5.8 1.1 1.7 2.3 2.9 2.2 1.2 0 1.6-.7 3-.7s1.8.7 3 .7c1.2 0 2.1-1.1 2.8-2.2.9-1.3 1.3-2.6 1.3-2.7-.1 0-2.2-.9-2.2-3.7Zm-2.4-7c.6-.8 1-1.9.9-3-1 .1-2.2.7-2.9 1.5-.6.7-1.1 1.8-1 2.9 1.1.1 2.3-.6 3-1.4Z" />
    </svg>
  ),
}

function SocialButton({
  provider,
  label,
  description,
  isPending,
  onClick,
}: {
  provider: SocialAuthProvider
  label: string
  description: string
  isPending: boolean
  onClick: (provider: SocialAuthProvider) => void
}) {
  return (
    <button
      type="button"
      disabled={isPending}
      onClick={() => onClick(provider)}
      className="group flex w-full items-center gap-3 border border-border bg-card px-4 py-3 text-left transition-colors duration-150 hover:border-foreground/30 disabled:cursor-not-allowed disabled:opacity-50"
    >
      <span className="flex h-8 w-8 shrink-0 items-center justify-center border border-border bg-background">
        {providerIcon[provider]}
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-sm font-medium text-foreground">{label}</span>
        <span className="block truncate text-xs text-muted-foreground">{description}</span>
      </span>
      {isPending ? (
        <Loader2 className="h-4 w-4 shrink-0 animate-spin text-muted-foreground" />
      ) : (
        <ArrowRight className="h-4 w-4 shrink-0 text-muted-foreground transition-transform duration-150 group-hover:translate-x-0.5" />
      )}
    </button>
  )
}

export const LoginPage: React.FC<LoginPageProps> = ({
  onLogin,
  pendingProvider = null,
  error = null,
  currentUser = null,
  onLogout,
  onNavigateToApp,
}) => {
  const { t } = useTranslation()

  const loginProviders: Array<{
    provider: SocialAuthProvider
    label: string
    description: string
  }> = [
    {
      provider: 'google',
      label: t('auth.login.google_label'),
      description: t('auth.login.google_desc'),
    },
    {
      provider: 'facebook',
      label: t('auth.login.facebook_label'),
      description: t('auth.login.facebook_desc'),
    },
    {
      provider: 'apple',
      label: t('auth.login.apple_label'),
      description: t('auth.login.apple_desc'),
    },
  ]

  return (
    <div className="flex min-h-screen bg-background">
      {/* Brand panel */}
      <aside className="hidden w-[42%] flex-col justify-between bg-[#101623] p-10 text-white lg:flex">
        <div className="flex items-center gap-2.5">
          <YntraMark className="h-6 w-6" />
          <span className="text-sm font-semibold uppercase tracking-[0.2em]">Yntra</span>
        </div>

        <div>
          <h1 className="max-w-sm text-4xl font-semibold leading-tight tracking-tight">
            {t('auth.login.title')}
          </h1>
          <p className="mt-4 max-w-sm text-sm leading-relaxed text-white/60">
            {t('auth.login.subtitle')}
          </p>
        </div>

        <p className="text-xs text-white/40">Yntra Platform</p>
      </aside>

      {/* Sign-in */}
      <main className="flex flex-1 items-center justify-center px-6 py-12">
        <div className="animate-fade-in w-full max-w-sm">
          <div className="mb-8 flex items-center gap-2 lg:hidden">
            <YntraMark className="h-6 w-6 text-foreground" />
            <span className="text-sm font-semibold uppercase tracking-[0.2em] text-foreground">
              Yntra
            </span>
          </div>

          {currentUser && (
            <div className="mb-6 rounded-md border border-border bg-card p-4 text-sm">
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate font-medium text-foreground">
                    {currentUser.name || currentUser.email}
                  </p>
                  <p className="text-xs text-muted-foreground capitalize">
                    {currentUser.role || 'user'} • Inloggad
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {onNavigateToApp && (
                    <button
                      type="button"
                      onClick={onNavigateToApp}
                      className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                    >
                      {t('common.open_app', 'Öppna app')}
                    </button>
                  )}
                  {onLogout && (
                    <button
                      type="button"
                      onClick={onLogout}
                      className="h-8 rounded-md border border-border px-3 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                    >
                      {t('common.logout', 'Logga ut')}
                    </button>
                  )}
                </div>
              </div>
            </div>
          )}

          <h2 className="text-xl font-semibold tracking-tight text-foreground">
            {t('auth.login.sign_in')}
          </h2>
          <p className="mt-1 text-sm text-muted-foreground">{t('auth.login.subtitle')}</p>

          <div className="mt-8 space-y-2">
            {loginProviders.map(({ provider, label, description }) => (
              <SocialButton
                key={provider}
                provider={provider}
                label={label}
                description={description}
                isPending={pendingProvider === provider}
                onClick={onLogin}
              />
            ))}
          </div>

          {isDemoMode() && (
            <div className="mt-4 border-t border-border/60 pt-4">
              <button
                type="button"
                onClick={() => onLogin('google')}
                className="flex h-9 w-full items-center justify-center gap-2 rounded-md bg-secondary/80 px-3 text-xs font-medium text-foreground transition-colors hover:bg-secondary"
              >
                <span>Demoläge: Logga in direkt som Admin</span>
              </button>
            </div>
          )}

          {error && (
            <div className="mt-6 border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {error}
            </div>
          )}

          <div className="mt-8 text-center text-xs text-muted-foreground">
            {t('auth.login.footer_info')}
          </div>
        </div>
      </main>
    </div>
  )
}

export default LoginPage
