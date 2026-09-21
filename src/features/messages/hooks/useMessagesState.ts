import { useState, useMemo, useCallback } from 'react'
import { useSearchParams } from 'react-router-dom'
import type { ComposeData, ProcessedMessage } from '../types'

export const useMessagesState = (processedMessages: ProcessedMessage[]) => {
  const [searchParams, setSearchParams] = useSearchParams()

  const filterType = searchParams.get('filter') || 'inbox'
  const searchQuery = searchParams.get('q') || ''
  const currentPageParams = searchParams.get('page')
  const currentPage = currentPageParams ? parseInt(currentPageParams, 10) : 1

  const setFilterType = useCallback(
    (newFilter: string) => {
      setSearchParams(
        (prev) => {
          if (newFilter === 'inbox') prev.delete('filter')
          else prev.set('filter', newFilter)
          prev.delete('page')
          return prev
        },
        { replace: true },
      )
    },
    [setSearchParams],
  )

  const setSearchQuery = useCallback(
    (newQuery: string) => {
      setSearchParams(
        (prev) => {
          if (!newQuery) prev.delete('q')
          else prev.set('q', newQuery)
          prev.delete('page')
          return prev
        },
        { replace: true },
      )
    },
    [setSearchParams],
  )

  const setCurrentPage = useCallback(
    (updater: number | ((prev: number) => number)) => {
      setSearchParams(
        (prev) => {
          const current = prev.get('page') ? parseInt(prev.get('page')!, 10) : 1
          const next = typeof updater === 'function' ? updater(current) : updater
          if (next === 1) prev.delete('page')
          else prev.set('page', next.toString())
          return prev
        },
        { replace: true },
      )
    },
    [setSearchParams],
  )

  const [activeMessageId, setActiveMessageId] = useState<string | null>(null)
  const [selectedMsgs, setSelectedMsgs] = useState<string[]>([])

  const [isComposing, setIsComposing] = useState(false)
  const [composeData, setComposeData] = useState<ComposeData>({
    targetType: 'user',
    targetId: '',
    subject: '',
    content: '',
    quote: null,
  })

  const activeMessage = useMemo(() => {
    return processedMessages.find((m) => m.id === activeMessageId) || null
  }, [processedMessages, activeMessageId])

  return {
    filterType,
    setFilterType,
    activeMessageId,
    setActiveMessageId,
    searchQuery,
    setSearchQuery,
    currentPage,
    setCurrentPage,
    selectedMsgs,
    setSelectedMsgs,
    isComposing,
    setIsComposing,
    composeData,
    setComposeData,
    activeMessage,
  }
}
