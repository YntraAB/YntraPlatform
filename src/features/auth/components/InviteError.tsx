import React from 'react'
import { AlertCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'

interface InviteErrorProps {
  error: string
  onClose: () => void
}

export const InviteError: React.FC<InviteErrorProps> = ({ error, onClose }) => {
  const { t } = useTranslation()

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <div className="animate-fade-in w-[400px] rounded-xl border border-border bg-sidebar p-8 text-center shadow-sm">
        <div className="mb-6 flex justify-center text-red-500">
          <AlertCircle className="h-12 w-12" />
        </div>
        <h2 className="mb-4 text-xl font-bold text-foreground">{t('auth.invite_error.title')}</h2>
        <p className="mb-8 text-sm text-muted-foreground">{error}</p>
        <button
          onClick={onClose}
          className="h-12 w-full rounded-lg bg-primary font-medium text-primary-foreground transition-colors hover:bg-primary/90"
        >
          {t('auth.invite_error.back_to_login')}
        </button>
      </div>
    </div>
  )
}
