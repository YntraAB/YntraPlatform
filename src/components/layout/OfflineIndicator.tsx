import React, { useEffect, useState } from 'react'
import { WifiOff, RefreshCw, AlertCircle } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useQueryClient } from '@tanstack/react-query'

import { useTranslation } from 'react-i18next'
import { usePlatform } from '@/platform'

export const OfflineIndicator: React.FC = () => {
  const { t } = useTranslation()
  const { isOnline } = usePlatform()
  const [isSyncing, setIsSyncing] = useState(false)
  const [hasSyncError, setHasSyncError] = useState(false)
  const queryClient = useQueryClient()

  useEffect(() => {
    const unsubscribe = queryClient.getMutationCache().subscribe((event) => {
      if (event.type === 'updated') {
        const mutations = queryClient.getMutationCache().getAll()
        const active = mutations.some((m) => m.state.status === 'pending')
        const error = mutations.some((m) => m.state.status === 'error')

        setIsSyncing(active)
        setHasSyncError(error)
      }
    })
    return () => unsubscribe()
  }, [queryClient])

  if (isOnline && !isSyncing && !hasSyncError) return null

  return (
    <div className="fixed bottom-4 left-4 z-[100] animate-in fade-in slide-in-from-bottom-2 duration-300">
      <div
        className={cn(
          'flex items-center gap-2 rounded-full px-4 py-2 text-xs font-bold shadow-sm border backdrop-blur-md',
          !isOnline
            ? 'bg-red-500/90 text-white border-red-400'
            : hasSyncError
              ? 'bg-amber-500/90 text-white border-amber-400'
              : 'bg-primary/90 text-primary-foreground border-primary/40',
        )}
      >
        {!isOnline ? (
          <>
            <WifiOff className="h-3.5 w-3.5 animate-pulse" />
            <span>{t('common.offline.mode')}</span>
          </>
        ) : hasSyncError ? (
          <>
            <AlertCircle className="h-3.5 w-3.5" />
            <span>{t('common.offline.sync_error')}</span>
          </>
        ) : (
          <>
            <RefreshCw className="h-3.5 w-3.5 animate-spin" />
            <span>{t('common.offline.syncing')}</span>
          </>
        )}
      </div>
    </div>
  )
}
