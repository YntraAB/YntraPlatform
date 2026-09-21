import React, { useEffect, useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { supabase } from '@/lib/supabase'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { InputOTP, InputOTPGroup, InputOTPSlot } from '@/components/ui/input-otp'
import { toast } from 'sonner'
import { Shield, ShieldCheck, Loader2, KeyRound } from 'lucide-react'
import { cn } from '@/lib/utils'

interface MFAFactor {
  id: string
  status: 'verified' | 'unverified'
  factor_type: 'totp' | 'phone' | 'webauthn'
  friendly_name?: string
}

interface MFAEnrollment {
  id: string
  type: 'totp'
  totp: {
    qr_code: string
    secret: string
    uri: string
  }
}

export const TwoFactorSettings: React.FC = () => {
  const { t } = useTranslation()
  const [loading, setLoading] = useState(true)
  const [isStartingEnroll, setIsStartingEnroll] = useState(false)
  const [factors, setFactors] = useState<MFAFactor[]>([])
  const [isDialogOpen, setIsDialogOpen] = useState(false)
  const [isConfirmOpen, setIsConfirmOpen] = useState(false)
  const [enrollmentData, setEnrollmentData] = useState<MFAEnrollment | null>(null)
  const [verificationCode, setVerificationCode] = useState('')
  const [isVerifying, setIsVerifying] = useState(false)
  const [showSecret, setShowSecret] = useState(false)

  const fetchFactors = useCallback(async () => {
    try {
      const { data, error } = await supabase.auth.mfa.listFactors()
      if (error) throw error
      setFactors(data.all || [])
    } catch (error) {
      console.error('Error fetching MFA factors:', error)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchFactors()
  }, [fetchFactors])

  const isEnabled = factors.some((f) => f.status === 'verified')

  const cleanupUnverifiedFactors = async (allFactors: MFAFactor[]) => {
    const unverified = allFactors.filter((f) => f.status === 'unverified')
    for (const factor of unverified) {
      await supabase.auth.mfa.unenroll({ factorId: factor.id })
    }
  }

  const handleStartEnroll = async () => {
    setIsStartingEnroll(true)
    try {
      const { data: latestData } = await supabase.auth.mfa.listFactors()
      if (latestData?.all) {
        await cleanupUnverifiedFactors(latestData.all)
      }

      const { data, error } = await supabase.auth.mfa.enroll({
        factorType: 'totp',
      })
      if (error) throw error

      setEnrollmentData(data)
      setShowSecret(false)
      setIsDialogOpen(true)
      setVerificationCode('')
    } catch {
      toast.error(t('settings.account.mfa.error_enroll'))
    } finally {
      setIsStartingEnroll(false)
    }
  }

  const handleVerify = async () => {
    if (!enrollmentData || verificationCode.length !== 6) return
    setIsVerifying(true)
    try {
      const { data: challengeData, error: challengeError } = await supabase.auth.mfa.challenge({
        factorId: enrollmentData.id,
      })
      if (challengeError) throw challengeError

      const { error: verifyError } = await supabase.auth.mfa.verify({
        factorId: enrollmentData.id,
        challengeId: challengeData.id,
        code: verificationCode,
      })
      if (verifyError) throw verifyError

      toast.success(t('settings.account.mfa.success_enabled'))
      setIsDialogOpen(false)
      setVerificationCode('')
      fetchFactors()
    } catch {
      toast.error(t('settings.account.mfa.error_verify'))
    } finally {
      setIsVerifying(false)
    }
  }

  const handleUnenroll = async () => {
    const factor = factors.find((f) => f.status === 'verified')
    if (!factor) return

    try {
      const { error } = await supabase.auth.mfa.unenroll({
        factorId: factor.id,
      })
      if (error) throw error
      toast.success(t('settings.account.mfa.success_disabled'))
      setIsConfirmOpen(false)
      fetchFactors()
    } catch {
      toast.error(t('settings.account.mfa.error_unenroll'))
    }
  }

  if (loading) {
    return (
      <div className="flex items-center gap-2 p-2 text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span className="text-xs font-medium italic">{t('common.loading')}</span>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              'rounded-xl p-2.5 transition-all duration-500',
              isEnabled
                ? 'bg-emerald-500/10 text-emerald-500 shadow-inner'
                : 'bg-amber-500/10 text-amber-500',
            )}
          >
            {isEnabled ? (
              <ShieldCheck className="h-5 w-5 animate-in fade-in zoom-in" />
            ) : (
              <Shield className="h-5 w-5" />
            )}
          </div>
          <div className="space-y-0.5">
            <p
              className={cn(
                'text-sm font-bold transition-colors',
                isEnabled
                  ? 'text-emerald-700 dark:text-emerald-400'
                  : 'text-amber-700 dark:text-amber-400',
              )}
            >
              {t('settings.account.mfa.title')}
            </p>
            <p
              className={cn(
                'text-[11px] leading-relaxed transition-colors',
                isEnabled
                  ? 'text-emerald-600 dark:text-emerald-500/70'
                  : 'text-amber-600 dark:text-amber-500/80',
              )}
            >
              {isEnabled ? t('settings.account.mfa.enabled') : t('settings.account.mfa.desc')}
            </p>
          </div>
        </div>
        <Button
          variant={isEnabled ? 'outline' : 'default'}
          size="sm"
          onClick={isEnabled ? () => setIsConfirmOpen(true) : handleStartEnroll}
          disabled={isStartingEnroll}
          className={cn(
            'min-w-[100px] transition-all duration-300',
            isEnabled
              ? 'border-red-500/20 text-red-600 hover:bg-red-500/10 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300'
              : 'shadow-sm shadow-primary/20 hover:shadow-primary/30',
          )}
        >
          {isStartingEnroll ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : isEnabled ? (
            t('settings.account.mfa.disable_button')
          ) : (
            t('settings.account.mfa.enable_button')
          )}
        </Button>
      </div>

      {/* Enrollment Dialog */}
      <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
        <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-md">
          <DialogHeader className="flex h-12 flex-row items-center justify-between border-b border-border bg-secondary/40 px-5">
            <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
              <KeyRound className="size-3.5 text-muted-foreground/70 shrink-0" />
              <span>{t('settings.account.mfa.setup_title')}</span>
            </DialogTitle>
          </DialogHeader>

          <div className="flex flex-col items-center justify-center space-y-5 p-5">
            <div className="group relative flex w-full flex-col items-center">
              {enrollmentData?.totp?.qr_code && (
                <div className="rounded-md border border-border bg-white p-3 shadow-xs">
                  <img src={enrollmentData.totp.qr_code} alt="QR Code" className="h-40 w-40" />
                </div>
              )}

              <div className="mt-3 w-full text-center">
                <button
                  type="button"
                  onClick={() => setShowSecret(!showSecret)}
                  className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground underline underline-offset-4 transition-colors hover:text-primary"
                >
                  {showSecret
                    ? t('settings.account.mfa.hide_secret')
                    : t('settings.account.mfa.show_secret')}
                </button>
                {showSecret && (
                  <div className="mt-2.5 select-all break-all rounded-md border border-border bg-secondary/40 p-2.5 text-center font-mono text-[11px]">
                    {enrollmentData?.totp?.secret}
                  </div>
                )}
              </div>
            </div>

            <div className="w-full max-w-[280px] space-y-3 text-center">
              <div className="space-y-0.5">
                <p className="text-xs font-medium text-foreground">
                  {t('settings.account.mfa.verify_title')}
                </p>
                <p className="text-[11px] text-muted-foreground">
                  {t('settings.account.mfa.verify_desc')}
                </p>
              </div>
              <div className="flex justify-center">
                <InputOTP
                  maxLength={6}
                  value={verificationCode}
                  onChange={setVerificationCode}
                  onComplete={handleVerify}
                >
                  <InputOTPGroup className="gap-1.5">
                    <InputOTPSlot
                      index={0}
                      className="h-10 w-9 rounded-md border border-border text-base font-medium"
                    />
                    <InputOTPSlot
                      index={1}
                      className="h-10 w-9 rounded-md border border-border text-base font-medium"
                    />
                    <InputOTPSlot
                      index={2}
                      className="h-10 w-9 rounded-md border border-border text-base font-medium"
                    />
                    <InputOTPSlot
                      index={3}
                      className="h-10 w-9 rounded-md border border-border text-base font-medium"
                    />
                    <InputOTPSlot
                      index={4}
                      className="h-10 w-9 rounded-md border border-border text-base font-medium"
                    />
                    <InputOTPSlot
                      index={5}
                      className="h-10 w-9 rounded-md border border-border text-base font-medium"
                    />
                  </InputOTPGroup>
                </InputOTP>
              </div>
            </div>
          </div>

          <DialogFooter className="flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setIsDialogOpen(false)}
              className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
            >
              {t('common.cancel')}
            </Button>
            <Button
              size="sm"
              onClick={handleVerify}
              disabled={verificationCode.length !== 6 || isVerifying}
              className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
            >
              {isVerifying ? (
                <Loader2 className="mr-1.5 size-3.5 animate-spin" />
              ) : (
                <ShieldCheck className="mr-1.5 size-3" />
              )}
              <span>{t('settings.account.mfa.verify_button')}</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Unenroll Confirmation */}
      <AlertDialog open={isConfirmOpen} onOpenChange={setIsConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('settings.account.mfa.unenroll_confirm_title')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('settings.account.mfa.unenroll_confirm_desc')}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleUnenroll}
              className="bg-destructive text-destructive-foreground shadow-sm shadow-destructive/20 hover:bg-destructive/90"
            >
              {t('settings.account.mfa.disable_button')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
