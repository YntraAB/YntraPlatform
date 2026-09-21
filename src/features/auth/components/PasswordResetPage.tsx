import React, { useState } from 'react'
import { Key } from 'lucide-react'
import { supabase } from '@/lib/supabase'
import { useTranslation } from 'react-i18next'

interface PasswordResetPageProps {
  onSuccess: () => void
  onCancel: () => void
}

export const PasswordResetPage: React.FC<PasswordResetPageProps> = ({ onSuccess, onCancel }) => {
  const { t } = useTranslation()
  const [newPassword, setNewPassword] = useState('')
  const [fullName, setFullName] = useState('')
  const [phoneNumber, setPhoneNumber] = useState('')
  const [resetLoading, setResetLoading] = useState(false)

  const handleSave = async () => {
    setResetLoading(true)
    const {
      data: { user },
      error,
    } = await supabase.auth.updateUser({
      password: newPassword,
      data: { full_name: fullName, phone: phoneNumber },
    })

    if (user) {
      await supabase
        .from('users')
        .update({
          full_name: fullName,
          phone: phoneNumber,
        })
        .eq('id', user.id)
    }

    setResetLoading(false)
    if (error) {
      alert(t('auth.setup.error_prefix') + error.message)
    } else {
      onSuccess()
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <div className="animate-fade-in w-[400px] rounded-xl border border-border bg-sidebar p-8 shadow-sm">
        <div className="mb-6 flex justify-center text-primary">
          <Key className="h-12 w-12" />
        </div>
        <h2 className="mb-2 text-center text-2xl font-bold text-foreground">
          {t('auth.setup.title')}
        </h2>
        <p className="mb-8 text-center text-sm text-muted-foreground">
          {t('auth.setup.description')}
        </p>

        <input
          type="text"
          value={fullName}
          onChange={(e) => setFullName(e.target.value)}
          placeholder={t('auth.setup.full_name')}
          className="mb-4 w-full rounded-md border border-border bg-accent px-3 py-3 text-sm text-foreground focus:border-primary focus:outline-none"
        />

        <input
          type="tel"
          value={phoneNumber}
          onChange={(e) => setPhoneNumber(e.target.value)}
          placeholder={t('auth.setup.phone_optional')}
          className="mb-4 w-full rounded-md border border-border bg-accent px-3 py-3 text-sm text-foreground focus:border-primary focus:outline-none"
        />

        <input
          type="password"
          value={newPassword}
          onChange={(e) => setNewPassword(e.target.value)}
          placeholder={t('auth.setup.new_password')}
          className="mb-6 w-full rounded-md border border-border bg-accent px-3 py-3 text-sm text-foreground focus:border-primary focus:outline-none"
        />

        <button
          disabled={resetLoading || newPassword.length < 6 || !fullName}
          onClick={handleSave}
          className="h-12 w-full rounded-lg bg-primary font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
        >
          {resetLoading ? t('common.saving') : t('auth.setup.save_continue')}
        </button>

        <button
          disabled={resetLoading}
          onClick={onCancel}
          className="mt-3 h-10 w-full rounded-lg bg-transparent text-sm font-medium text-muted-foreground transition-colors hover:bg-muted"
        >
          {t('auth.setup.cancel_logout')}
        </button>
      </div>
    </div>
  )
}
