import { memo, useEffect, useRef } from 'react'
import { Send, ChevronLeft, X, Reply, Forward } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useTranslation } from 'react-i18next'
import type { ComposeData } from '../types'
import type { User } from '@/types'

interface SimpleTeam {
  id: string
  name: string
}

interface MessageComposePaneProps {
  composeData: ComposeData
  setComposeData: (data: ComposeData) => void
  isSending: boolean
  handleSendMessage: () => void
  setIsComposing: (val: boolean) => void
  users: User[]
  teams: SimpleTeam[]
  currentUserId: string | undefined
}

export const MessageComposePane = memo<MessageComposePaneProps>(
  ({
    composeData,
    setComposeData,
    isSending,
    handleSendMessage,
    setIsComposing,
    users,
    teams,
    currentUserId,
  }) => {
    const { t } = useTranslation()
    const subjectInputRef = useRef<HTMLInputElement>(null)
    const contentTextareaRef = useRef<HTMLTextAreaElement>(null)

    const isReply = composeData.quote?.type === 'reply'
    const isForward = composeData.quote?.type === 'forward'

    useEffect(() => {
      // Focus the subject input if empty and target selected, otherwise focus content textarea
      if (composeData.targetId && !composeData.subject) {
        subjectInputRef.current?.focus()
      } else if (composeData.targetId && composeData.subject) {
        contentTextareaRef.current?.focus()
      }
    }, [composeData.targetId, composeData.subject])

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        if (!isSending && composeData.targetId && composeData.content.trim()) {
          e.preventDefault()
          handleSendMessage()
        }
      }
    }

    const clearQuote = () => {
      setComposeData({ ...composeData, quote: null })
    }

    const titleText = isReply
      ? t('messages.reply', 'Svara')
      : isForward
        ? t('messages.forward', 'Vidarebefordra')
        : t('messages.compose_title', 'Nytt meddelande')

    return (
      <div className="relative flex h-full flex-1 flex-col bg-background duration-300 animate-in fade-in selection:bg-primary/20">
        {/* Sticky Topbar - Matching MessageReadPane h-12 */}
        <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
          <div className="flex items-center gap-3">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setIsComposing(false)}
              className="h-8 gap-1.5 px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-secondary/60 hover:text-foreground"
            >
              <ChevronLeft className="h-3.5 w-3.5" />
              <span>{t('common.back', 'Tillbaka')}</span>
            </Button>
            <div className="h-4 w-px bg-border/40" />
            <h2 className="text-xs font-medium text-foreground">
              {titleText}
            </h2>
          </div>

          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              className="h-8 px-2.5 text-xs font-medium text-muted-foreground hover:text-foreground"
              onClick={() => setIsComposing(false)}
            >
              {t('messages.cancel', 'Avbryt')}
            </Button>
            <Button
              onClick={handleSendMessage}
              disabled={isSending || !composeData.targetId || !composeData.content.trim()}
              className="h-8 gap-1.5 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs transition-all hover:bg-primary/90 disabled:opacity-50"
            >
              {isSending ? (
                <>
                  <div className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/30 border-t-white" />
                  <span>{t('messages.sending', 'Skickar...')}</span>
                </>
              ) : (
                <>
                  <Send className="h-3.5 w-3.5" />
                  <span>{t('messages.send', 'Skicka')}</span>
                </>
              )}
            </Button>
          </div>
        </div>

        {/* Scrollable Compose Form Body - Matching MessageReadPane max-w-3xl */}
        <div className="scrollbar-dark flex-1 overflow-y-auto scroll-smooth px-6 py-6 md:px-12 md:py-8">
          <div className="mx-auto max-w-3xl space-y-5" onKeyDown={handleKeyDown}>
            {/* Recipient ("Till") row */}
            <div className="flex items-center gap-3 border-b border-border/40 pb-3">
              <label className="w-14 shrink-0 text-xs font-medium text-muted-foreground">
                {t('messages.to', 'Till')}
              </label>
              <div className="flex flex-1 items-center gap-2">
                <Select
                  value={composeData.targetType}
                  onValueChange={(val) =>
                    setComposeData({
                      ...composeData,
                      targetType: val as 'user' | 'team',
                      targetId: '',
                    })
                  }
                >
                  <SelectTrigger
                    size="sm"
                    className="h-8 w-24 border-border/50 bg-muted/30 text-xs font-medium"
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="user" className="text-xs">
                      {t('messages.person', 'Person')}
                    </SelectItem>
                    <SelectItem value="team" className="text-xs">
                      {t('messages.team', 'Team')}
                    </SelectItem>
                  </SelectContent>
                </Select>

                <div className="h-3.5 w-px bg-border/40" />

                <Select
                  value={composeData.targetId}
                  onValueChange={(val) => setComposeData({ ...composeData, targetId: val })}
                >
                  <SelectTrigger className="h-8 flex-1 border-none bg-transparent text-xs font-medium shadow-none focus-visible:ring-0">
                    <SelectValue placeholder={t('messages.recipient_placeholder', 'Välj mottagare...')} />
                  </SelectTrigger>
                  <SelectContent>
                    {composeData.targetType === 'user'
                      ? users
                          .filter((u) => u.id !== currentUserId)
                          .map((u) => (
                            <SelectItem key={u.id} value={u.id} className="text-xs">
                              {u.name || u.email}
                            </SelectItem>
                          ))
                      : teams.map((t) => (
                          <SelectItem key={t.id} value={t.id} className="text-xs">
                            {t.name}
                          </SelectItem>
                        ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            {/* Subject ("Ämne") row */}
            <div className="flex items-center gap-3 border-b border-border/40 pb-3">
              <label className="w-14 shrink-0 text-xs font-normal text-muted-foreground">
                {t('messages.subject', 'Ämne')}
              </label>
              <Input
                ref={subjectInputRef}
                placeholder={t('messages.subject_placeholder', 'Ämne...')}
                value={composeData.subject}
                onChange={(e) => setComposeData({ ...composeData, subject: e.target.value })}
                className="h-8 flex-1 border-none bg-transparent p-0 text-xs font-normal text-foreground placeholder:text-xs placeholder:text-muted-foreground/40 shadow-none focus-visible:ring-0"
              />
            </div>

            {/* Content Textarea */}
            <div className="space-y-4 pt-1">
              <textarea
                ref={contentTextareaRef}
                value={composeData.content}
                onChange={(e) => setComposeData({ ...composeData, content: e.target.value })}
                className="w-full min-h-[160px] sm:min-h-[200px] resize-none border-none bg-transparent text-xs font-normal leading-relaxed text-foreground/90 placeholder:text-xs placeholder:text-muted-foreground/40 focus:outline-none"
                placeholder={t('messages.content_placeholder', 'Skriv ett meddelande...')}
                autoFocus={!!composeData.targetId && !!composeData.subject}
              />

              {/* Replied / Forwarded Quote Box */}
              {composeData.quote && (
                <div className="duration-300 animate-in fade-in slide-in-from-top-1.5">
                  <div className="rounded-md border border-border/50 border-l-2 border-l-primary/60 bg-muted/20 p-3.5 space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                        {isForward ? (
                          <>
                            <Forward className="h-3 w-3 text-muted-foreground/70" />
                            <span className="font-medium text-foreground/75">
                              {t('messages.forwarded_message', 'Vidarebefordrat meddelande')}
                            </span>
                          </>
                        ) : (
                          <>
                            <Reply className="h-3 w-3 text-muted-foreground/70" />
                            <span>
                              <span className="font-medium text-foreground/80">{composeData.quote.sender}</span>
                              {' '}{t('messages.wrote', 'skrev')}
                              <span className="text-muted-foreground/60 tabular-nums"> ({composeData.quote.date} kl. {composeData.quote.timestamp})</span>
                            </span>
                          </>
                        )}
                      </div>

                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        onClick={clearQuote}
                        className="h-6 w-6 text-muted-foreground/60 hover:text-foreground hover:bg-secondary/60 rounded"
                        title={t('common.remove', 'Ta bort citat')}
                      >
                        <X className="h-3 w-3" />
                      </Button>
                    </div>

                    {isForward && (
                      <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-1 text-[11px] text-muted-foreground/70 border-b border-border/30 pb-2">
                        <div>
                          <span className="opacity-70">{t('messages.from', 'Från')}: </span>
                          <span className="font-medium text-foreground/80">{composeData.quote.sender}</span>
                        </div>
                        <div>
                          <span className="opacity-70">{t('messages.date', 'Datum')}: </span>
                          <span className="tabular-nums font-normal">{composeData.quote.date} {composeData.quote.timestamp}</span>
                        </div>
                        <div className="sm:col-span-2">
                          <span className="opacity-70">{t('messages.subject', 'Ämne')}: </span>
                          <span className="font-medium text-foreground/80">{composeData.quote.subject}</span>
                        </div>
                      </div>
                    )}

                    <div className="whitespace-pre-wrap text-xs font-normal leading-relaxed text-muted-foreground/80 pt-1">
                      {composeData.quote.content}
                    </div>
                  </div>
                </div>
              )}

              {/* Bottom Quick Bar */}
              <div className="flex items-center justify-between border-t border-border/40 pt-4 pb-8">
                <div className="hidden sm:flex items-center gap-1.5 text-[11px] text-muted-foreground/50">
                  <span>Tryck</span>
                  <kbd className="rounded border border-border/60 bg-muted/40 px-1 py-0.5 font-mono text-[10px] text-muted-foreground">
                    Ctrl
                  </kbd>
                  <span>+</span>
                  <kbd className="rounded border border-border/60 bg-muted/40 px-1 py-0.5 font-mono text-[10px] text-muted-foreground">
                    Enter
                  </kbd>
                  <span>för att skicka</span>
                </div>

                <div className="flex items-center gap-2 ml-auto">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setIsComposing(false)}
                    className="h-8 px-3 text-xs font-medium text-muted-foreground hover:text-foreground"
                  >
                    {t('messages.cancel', 'Avbryt')}
                  </Button>
                  <Button
                    onClick={handleSendMessage}
                    disabled={isSending || !composeData.targetId || !composeData.content.trim()}
                    className="h-8 gap-1.5 rounded-md bg-primary px-3.5 text-xs font-medium text-primary-foreground shadow-xs transition-all hover:bg-primary/90 disabled:opacity-50"
                  >
                    {isSending ? (
                      <>
                        <div className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/30 border-t-white" />
                        <span>{t('messages.sending', 'Skickar...')}</span>
                      </>
                    ) : (
                      <>
                        <Send className="h-3.5 w-3.5" />
                        <span>{t('messages.send', 'Skicka')}</span>
                      </>
                    )}
                  </Button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    )
  },
)

MessageComposePane.displayName = 'MessageComposePane'
