import React from 'react'
import { useTranslation } from 'react-i18next'
import { useAuth } from '@/hooks/useAuth'

export const ClientHomePage: React.FC = () => {
  const { t } = useTranslation()
  const { user } = useAuth()

  return (
    <div className="mx-auto w-full max-w-4xl p-6">
      <h1 className="mb-5 text-base font-medium tracking-tight text-foreground">
        {t('client_portal.welcome', { name: user?.name })}
      </h1>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <div className="rounded-lg border border-border bg-card p-5 shadow-xs">
          <h2 className="mb-1.5 text-sm font-medium text-foreground">{t('client_portal.todays_assistance')}</h2>
          <p className="mb-3 text-xs text-muted-foreground">
            {t('client_portal.assistance_description')}
          </p>
          <div className="flex items-center justify-center rounded-md border border-dashed border-border bg-secondary/50 p-4 text-xs text-muted-foreground">
            {t('client_portal.no_schedule')}
          </div>
        </div>
        <div className="rounded-lg border border-border bg-card p-5 shadow-xs">
          <h2 className="mb-1.5 text-sm font-medium text-foreground">{t('client_portal.messages')}</h2>
          <p className="mb-3 text-xs text-muted-foreground">
            {t('client_portal.messages_description')}
          </p>
          <a
            href="/inbox"
            className="mt-2 inline-flex h-8 items-center justify-center rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs transition-colors hover:bg-primary/90"
          >
            {t('client_portal.go_to_inbox')}
          </a>
        </div>
      </div>
    </div>
  )
}
