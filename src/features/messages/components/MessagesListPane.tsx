import React, { memo, useCallback } from 'react'
import {
  Inbox,
  Trash2,
  Search,
  ChevronLeft,
  ChevronRight,
  MoreHorizontal,
  Clock,
  Archive,
  Plus,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useTranslation } from 'react-i18next'
import { cva } from 'class-variance-authority'
import { cn } from '@/lib/utils'
import { MessageSkeleton } from './MessageSkeleton'
import type { ProcessedMessage } from '../types'

const messageRowVariants = cva(
  'group flex items-center px-6 py-3.5 border-b border-border/40 hover:bg-secondary/40 cursor-pointer transition-all duration-300 animate-in fade-in slide-in-from-bottom-2 fill-mode-both',
  {
    variants: {
      selected: {
        true: 'bg-secondary/20',
        false: '',
      },
    },
    defaultVariants: {
      selected: false,
    },
  },
)

const senderVariants = cva('w-36 sm:w-48 md:w-56 shrink-0 truncate pr-3 transition-colors', {
  variants: {
    unread: {
      true: 'text-foreground font-medium',
      false: 'text-muted-foreground group-hover:text-foreground/80',
    },
  },
  defaultVariants: {
    unread: false,
  },
})

const subjectVariants = cva('truncate transition-colors', {
  variants: {
    unread: {
      true: 'text-foreground font-medium',
      false: 'text-foreground/90',
    },
  },
  defaultVariants: {
    unread: false,
  },
})

const timestampVariants = cva(
  'text-[11px] font-medium tracking-tight uppercase tabular-nums transition-colors',
  {
    variants: {
      unread: {
        true: 'text-primary',
        false: 'text-muted-foreground/50',
      },
    },
    defaultVariants: {
      unread: false,
    },
  },
)

const listHeaderVariants = cva(
  'h-12 px-6 flex items-center justify-between border-b border-border/50 bg-background/50 backdrop-blur-md sticky top-0 z-20',
)

const containerVariants = cva(
  'flex-1 flex flex-col h-full bg-background relative selection:bg-primary/20',
)

const actionIconVariants = cva('w-[18px] h-[18px] transition-colors cursor-pointer', {
  variants: {
    intent: {
      archive: 'hover:text-primary',
      trash: 'hover:text-rose-500',
      clock: 'hover:text-foreground',
    },
  },
})

const paginationButtonVariants = cva('w-7 h-7 rounded-md transition-all', {
  variants: {
    enabled: {
      true: 'hover:text-foreground hover:bg-secondary',
      false: 'opacity-30',
    },
  },
  defaultVariants: {
    enabled: true,
  },
})

interface MessagesListPaneProps {
  messages: ProcessedMessage[]
  isLoading: boolean
  filterType: string
  setFilterType: (val: string) => void
  searchQuery: string
  setSearchQuery: (val: string) => void
  currentPage: number
  setCurrentPage: (updater: number | ((prev: number) => number)) => void
  totalPages: number
  handleSelectMessage: (id: string) => void
  handleCompose: () => void
  selectedMsgs: string[]
  setSelectedMsgs: React.Dispatch<React.SetStateAction<string[]>>
  handleDeleteSelected: () => void
}

export const MessagesListPane = memo<MessagesListPaneProps>(
  ({
    messages,
    isLoading,
    filterType,
    setFilterType,
    searchQuery,
    setSearchQuery,
    currentPage,
    setCurrentPage,
    totalPages,
    handleSelectMessage,
    handleCompose,
    selectedMsgs,
    setSelectedMsgs,
    handleDeleteSelected,
  }) => {
    const { t } = useTranslation()

    const getTitle = useCallback(() => {
      if (filterType === 'inbox') return t('messages.inbox')
      if (filterType === 'unread') return t('messages.unread')
      if (filterType === 'sent') return t('messages.sent')
      if (filterType === 'archive') return t('messages.archive')
      if (filterType === 'trash') return t('messages.trash')
      return t('messages.inbox')
    }, [filterType, t])

    const isAllSelected = messages.length > 0 && selectedMsgs.length === messages.length

    const toggleSelectAll = useCallback(() => {
      if (isAllSelected) {
        setSelectedMsgs([])
      } else {
        setSelectedMsgs(messages.map((m) => m.id))
      }
    }, [isAllSelected, messages, setSelectedMsgs])

    const toggleSelect = useCallback(
      (id: string, e: React.MouseEvent<HTMLDivElement> | React.KeyboardEvent<HTMLDivElement>) => {
        e.stopPropagation()
        if (selectedMsgs.includes(id)) {
          setSelectedMsgs((prev) => prev.filter((m) => m !== id))
        } else {
          setSelectedMsgs((prev) => [...prev, id])
        }
      },
      [selectedMsgs, setSelectedMsgs],
    )

    return (
      <div className={cn(containerVariants())}>
        {/* Top Header Controls */}
        <div className={cn(listHeaderVariants())}>
          <div className="flex items-center gap-3">
            <div className="flex items-center justify-center">
              <Checkbox
                checked={isAllSelected}
                onCheckedChange={toggleSelectAll}
                aria-label={t('common.select_all')}
                className="h-4 w-4"
              />
            </div>

            <div className="h-3.5 w-px bg-border/40" />

            <h2 className="flex items-center text-xs font-medium text-foreground">
              {getTitle()}
            </h2>

            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 rounded-md"
              aria-label={t('common.more_options')}
            >
              <MoreHorizontal className="h-3.5 w-3.5 cursor-pointer text-muted-foreground transition-colors hover:text-foreground" />
            </Button>

            {selectedMsgs.length > 0 && (
              <div className="ml-1 flex items-center gap-1 border-l border-border/40 pl-2 duration-200 animate-in fade-in slide-in-from-left-2">
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={t('messages.delete_selected')}
                  className="h-7 w-7 text-muted-foreground transition-all hover:bg-destructive/10 hover:text-destructive"
                  onClick={handleDeleteSelected}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
                <div className="rounded border border-border/60 bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
                  {selectedMsgs.length} {t('messages.selected')}
                </div>
              </div>
            )}
          </div>

          <div className="flex items-center gap-2.5">
            <div className="group relative w-48 sm:w-56">
              <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground transition-colors group-focus-within:text-primary" />
              <Input
                placeholder={t('messages.search_placeholder')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                aria-label={t('messages.search_placeholder')}
                className="h-8 rounded-md border border-border/50 bg-muted/50 pl-8 text-xs text-foreground transition-all focus-visible:bg-muted focus-visible:ring-1 focus-visible:ring-primary"
              />
            </div>

            <Select value={filterType} onValueChange={setFilterType}>
              <SelectTrigger
                size="sm"
                className="h-8 w-[105px] rounded-md border-border/50 bg-muted/80 text-xs font-medium text-foreground"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="inbox" className="text-xs">{t('messages.inbox')}</SelectItem>
                <SelectItem value="unread" className="text-xs">{t('messages.unread')}</SelectItem>
                <SelectItem value="sent" className="text-xs">{t('messages.sent')}</SelectItem>
                <SelectItem value="archive" className="text-xs">{t('messages.archive')}</SelectItem>
                <SelectItem value="trash" className="text-xs text-rose-400 focus:text-rose-400">
                  {t('messages.trash')}
                </SelectItem>
              </SelectContent>
            </Select>

            <div className="flex items-center gap-1 text-muted-foreground">
              <Button
                variant="ghost"
                size="icon"
                aria-label={t('pagination.previous')}
                className={cn(paginationButtonVariants({ enabled: currentPage > 1 }))}
                disabled={currentPage <= 1 || isLoading}
                onClick={() => setCurrentPage((prev) => prev - 1)}
              >
                <ChevronLeft className="h-3.5 w-3.5" />
              </Button>
              <span
                className="text-xs font-medium tabular-nums text-foreground/80 px-1"
                aria-label={t('pagination.page_info', { current: currentPage, total: totalPages })}
              >
                {currentPage} <span className="mx-0.5 font-normal text-muted-foreground/60">/</span>{' '}
                {totalPages}
              </span>
              <Button
                variant="ghost"
                size="icon"
                aria-label={t('pagination.next')}
                className={cn(paginationButtonVariants({ enabled: currentPage < totalPages }))}
                disabled={currentPage >= totalPages || isLoading}
                onClick={() => setCurrentPage((prev) => prev + 1)}
              >
                <ChevronRight className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        </div>

        {isLoading ? (
          <MessageSkeleton />
        ) : (
          <div className="scrollbar-dark w-full flex-1 overflow-y-auto scroll-smooth">
            {messages.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center text-muted-foreground duration-500 animate-in fade-in zoom-in-95">
                <div className="mb-6 flex h-20 w-20 items-center justify-center rounded-full border border-border/50 bg-muted/30">
                  <Inbox className="h-10 w-10 opacity-20" />
                </div>
                <p className="text-sm font-medium">{t('messages.empty_state')}</p>
              </div>
            ) : (
              <div className="flex w-full flex-col text-sm">
                {messages.map((msg, idx) => {
                  const isSelected = selectedMsgs.includes(msg.id)
                  return (
                    <div
                      key={msg.id}
                      onClick={() => handleSelectMessage(msg.id)}
                      className={cn(messageRowVariants({ selected: isSelected }))}
                      style={{ animationDelay: `${idx * 30}ms` }}
                      tabIndex={0}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') handleSelectMessage(msg.id)
                      }}
                      aria-label={t('messages.read_message', { subject: msg.subject })}
                    >
                      <div
                        className="flex w-6 shrink-0 items-center justify-center"
                        onClick={(e) => {
                          e.stopPropagation()
                          toggleSelect(msg.id, e)
                        }}
                      >
                        <Checkbox
                          checked={isSelected}
                          onCheckedChange={() => toggleSelect(msg.id, { stopPropagation: () => {} } as any)}
                          className="h-4 w-4"
                        />
                      </div>

                      <div className={cn(senderVariants({ unread: !!msg.unread }))}>
                        <div className="flex items-center gap-2">
                          {msg.sender.name}
                          {msg.unread && <span className="h-1.5 w-1.5 rounded-full bg-primary" />}
                        </div>
                        {msg.isTeamMessage && msg.folderId !== 'sent' && (
                          <div className="mt-1 truncate text-[10px] font-medium uppercase tracking-widest text-primary opacity-80">
                            {t('messages.team_collective', { name: msg.to })}
                          </div>
                        )}
                      </div>

                      <div className="flex min-w-0 flex-1 items-center gap-x-2 truncate pr-6">
                        {msg.folderId === 'sent' && (
                          <span className="shrink-0 rounded bg-primary/5 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-tighter text-primary/60">
                            {t('messages.to')} {msg.to}
                          </span>
                        )}
                        <span className={cn(subjectVariants({ unread: !!msg.unread }))}>
                          {msg.subject}
                        </span>
                        <span className="truncate font-light italic text-muted-foreground/60">
                          {msg.snippet}
                        </span>
                      </div>

                      <div className="flex w-48 shrink-0 items-center justify-end">
                        <div className="mr-6 flex translate-x-2 items-center gap-3.5 text-muted-foreground/60 opacity-0 transition-all duration-300 group-hover:translate-x-0 group-hover:opacity-100">
                          <Archive className={cn(actionIconVariants({ intent: 'archive' }))} />
                          <Trash2 className={cn(actionIconVariants({ intent: 'trash' }))} />
                          <Clock className={cn(actionIconVariants({ intent: 'clock' }))} />
                        </div>
                        <span className={cn(timestampVariants({ unread: !!msg.unread }))}>
                          {msg.date === t('common.today') ? msg.timestamp : msg.date}
                        </span>
                      </div>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        )}

        {/* Floating Action Button */}
        <div className="absolute bottom-6 right-6 z-20">
          <Button
            onClick={handleCompose}
            size="default"
            aria-label={t('messages.new_message')}
            className="h-10 gap-2 rounded-lg px-4 font-medium shadow-md"
          >
            <Plus className="h-4 w-4" />
            <span>{t('messages.new_message')}</span>
          </Button>
        </div>
      </div>
    )
  },
)

MessagesListPane.displayName = 'MessagesListPane'
