import { memo } from 'react'
import { Trash2, Reply, Forward, ChevronLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { useTranslation } from 'react-i18next'
import type { ProcessedMessage } from '../types'

interface MessageReadPaneProps {
  activeMessage: ProcessedMessage | null
  currentUserId: string | undefined
  setActiveMessageId: (id: string | null) => void
  handleReply: () => void
  handleForward: () => void
  handleDelete?: () => void
}

export const MessageReadPane = memo<MessageReadPaneProps>(
  ({ activeMessage, currentUserId, setActiveMessageId, handleReply, handleForward, handleDelete }) => {
    const { t } = useTranslation()

    if (!activeMessage) return null

    return (
      <div className="relative flex h-full flex-1 flex-col bg-background duration-300 animate-in fade-in selection:bg-primary/20">
        {/* Top Header - Compact h-12 with Back button and Quick Actions */}
        <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
          {/* Left: Back button & Breadcrumb */}
          <div className="flex min-w-0 items-center gap-3">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setActiveMessageId(null)}
              className="h-8 gap-1.5 px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-secondary/60 hover:text-foreground"
            >
              <ChevronLeft className="h-3.5 w-3.5" />
              <span>{t('common.back', 'Tillbaka')}</span>
            </Button>

            <div className="h-4 w-px bg-border/40" />

            <span className="truncate text-xs font-normal text-muted-foreground/60 max-w-[160px] sm:max-w-xs md:max-w-md">
              {activeMessage.subject}
            </span>
          </div>

          {/* Right: Actions */}
          <div className="flex shrink-0 items-center gap-1.5">
            <Button
              variant="outline"
              size="sm"
              onClick={handleReply}
              className="h-8 gap-1.5 px-2.5 text-xs font-medium text-foreground"
            >
              <Reply className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="hidden sm:inline">{t('messages.reply', 'Svara')}</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleForward}
              className="h-8 gap-1.5 px-2.5 text-xs font-medium text-muted-foreground hover:text-foreground"
            >
              <Forward className="h-3.5 w-3.5" />
              <span className="hidden sm:inline">{t('messages.forward', 'Vidarebefordra')}</span>
            </Button>
            {handleDelete && (
              <>
                <div className="mx-0.5 h-4 w-px bg-border/40" />
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleDelete}
                  aria-label={t('messages.trash', 'Radera')}
                  className="h-8 w-8 text-muted-foreground hover:bg-rose-500/10 hover:text-rose-500"
                  title={t('messages.trash', 'Radera')}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </>
            )}
          </div>
        </div>

        {/* Reading Pane Body */}
        <div className="scrollbar-dark flex-1 overflow-y-auto scroll-smooth px-6 py-6 md:px-12 md:py-8">
          <div className="mx-auto max-w-3xl space-y-6">
            {/* Subject Title */}
            <h1 className="text-base sm:text-lg font-medium tracking-tight text-foreground">
              {activeMessage.subject}
            </h1>

            {/* Sender & Recipient Information */}
            <div className="flex items-start justify-between gap-4 border-b border-border/40 pb-5">
              <div className="flex items-center gap-3">
                <Avatar className="size-8 border border-border/50 bg-muted/50">
                  {activeMessage.sender.avatar ? (
                    <AvatarImage src={activeMessage.sender.avatar} className="rounded-full" />
                  ) : null}
                  <AvatarFallback className="bg-primary/10 text-xs font-medium text-primary">
                    {activeMessage.sender.name.charAt(0).toUpperCase()}
                  </AvatarFallback>
                </Avatar>
                <div className="space-y-0.5">
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-medium text-foreground">
                      {activeMessage.sender.name}
                    </span>
                    <span className="rounded px-1.5 py-0.5 text-[10px] font-normal bg-muted/60 text-muted-foreground">
                      {activeMessage.sender_id === currentUserId
                        ? t('messages.you', 'Du')
                        : activeMessage.isTeamMessage
                          ? t('common.team', 'Team')
                          : t('directory.roles.assistant', 'Assistent')}
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground/60">
                    <span>{t('messages.to', 'Till')}:</span>
                    <span className="font-medium text-foreground/80">{activeMessage.to}</span>
                    <span className="opacity-40">·</span>
                    <span className="tabular-nums opacity-80">{activeMessage.date} kl. {activeMessage.timestamp}</span>
                  </div>
                </div>
              </div>

              <div className="hidden sm:block text-right">
                <span className="text-xs text-muted-foreground/60 tabular-nums">
                  {activeMessage.date} {activeMessage.timestamp}
                </span>
              </div>
            </div>

            {/* Message Body Content */}
            <div className="whitespace-pre-wrap text-xs md:text-sm font-normal leading-relaxed text-foreground/90 py-2">
              {activeMessage.content}
            </div>

            {/* Bottom Quick Actions */}
            <div className="mt-8 flex items-center gap-2.5 border-t border-border/40 pt-5">
              <Button
                onClick={handleReply}
                size="sm"
                className="h-8 gap-1.5 px-3 text-xs font-medium bg-primary text-primary-foreground shadow-xs hover:bg-primary/90"
              >
                <Reply className="h-3.5 w-3.5" />
                <span>{t('messages.reply', 'Svara')}</span>
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={handleForward}
                className="h-8 gap-1.5 px-3 text-xs font-medium border-border hover:bg-secondary"
              >
                <Forward className="h-3.5 w-3.5 text-muted-foreground" />
                <span>{t('messages.forward', 'Vidarebefordra')}</span>
              </Button>
            </div>
          </div>
        </div>
      </div>
    )
  },
)

MessageReadPane.displayName = 'MessageReadPane'
