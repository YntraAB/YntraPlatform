import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useWorkspace } from '@/contexts/WorkspaceContext'
import { Switch } from '@/components/ui/switch'
import {
  LayoutGrid,
  Search,
  Settings2,
} from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { supabase } from '@/lib/supabase'
import { ICON_MAP, type IconName } from '@/lib/blocks/icons'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { SchedulerSettings } from './SchedulerSettings'
import { NotificationsSettings } from './NotificationsSettings'

interface Block {
  id: string
  name: string
  description: string
  icon: string
  category: string
  dependencies: string[]
}

const CONFIGURABLE_BLOCKS = ['scheduling', 'messaging']

export const BlockSettings: React.FC<{ setIsSaving: (val: boolean) => void }> = ({ setIsSaving }) => {
  const { t } = useTranslation()
  const { modules, updateModules } = useWorkspace()
  const [availableBlocks, setAvailableBlocks] = useState<Block[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [searchTerm, setSearchTerm] = useState('')
  const [selectedCategory, setSelectedCategory] = useState<string>('all')
  const [configBlock, setConfigBlock] = useState<Block | null>(null)

  useEffect(() => {
    async function fetchBlocks() {
      const { data, error } = await supabase
        .from('blocks')
        .select('*')
        .order('name')

      if (!error && data) {
        setAvailableBlocks(data)
      }
      setIsLoading(false)
    }
    fetchBlocks()
  }, [])

  const categories = ['all', ...Array.from(new Set(availableBlocks.map((b) => b.category.toLowerCase())))]

  const filteredBlocks = availableBlocks.filter((block) => {
    const matchesSearch =
      block.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      block.description.toLowerCase().includes(searchTerm.toLowerCase())

    const matchesCategory = selectedCategory === 'all' || block.category.toLowerCase() === selectedCategory

    return matchesSearch && matchesCategory
  })

  const handleToggle = async (blockId: string, enabled: boolean) => {
    if (enabled) {
      const block = availableBlocks.find((b) => b.id === blockId)
      if (block?.dependencies?.length) {
        const missing = block.dependencies.filter((depId) => !(modules as any)[depId])
        if (missing.length > 0) {
          const names = missing
            .map((id) => availableBlocks.find((b) => b.id === id)?.name || id)
            .join(', ')
          alert(t('settings.blocks.dependency_error', { names }))
          return
        }
      }
    }

    setIsSaving(true)
    await updateModules({ [blockId]: enabled })
    setTimeout(() => setIsSaving(false), 500)
  }

  if (isLoading) {
    return (
      <div className="space-y-2">
        {[1, 2, 3, 4, 5].map((i) => (
          <div key={i} className="h-12 w-full animate-pulse rounded-lg border border-border/40 bg-card/30" />
        ))}
      </div>
    )
  }

  return (
    <div className="space-y-6 animate-in fade-in duration-300">
      {/* Header & Description */}
      <div className="space-y-1">
        <div className="flex items-center gap-2">
          <LayoutGrid className="h-4 w-4 text-muted-foreground/70" />
          <h2 className="text-sm font-medium tracking-tight text-foreground">
            {t('settings.blocks.title')}
          </h2>
        </div>
        <p className="text-xs font-light text-muted-foreground/70">
          {t('settings.blocks.desc')}
        </p>
      </div>

      {/* Search & Filter Toolbar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="relative flex-1 sm:max-w-xs">
          <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground/60" />
          <Input
            placeholder={t('settings.blocks.search_placeholder')}
            className="h-[30px] pl-8 text-xs rounded-md border-border/60 bg-background focus-visible:ring-1 focus-visible:ring-ring"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          {categories.map((cat) => {
            const isSelected = selectedCategory === cat
            return (
              <button
                key={cat}
                type="button"
                onClick={() => setSelectedCategory(cat)}
                className={cn(
                  'h-[28px] rounded-md px-2.5 text-xs transition-colors',
                  isSelected
                    ? 'bg-secondary font-medium text-foreground shadow-xs border border-border/60'
                    : 'text-muted-foreground hover:bg-secondary/40 hover:text-foreground',
                )}
              >
                {cat === 'all'
                  ? t('settings.blocks.filter_all')
                  : t(`blocks.categories.${cat}`, cat.charAt(0).toUpperCase() + cat.slice(1))}
              </button>
            )
          })}
        </div>
      </div>

      {/* Canonical List Standard (h-12 rows) */}
      {filteredBlocks.length === 0 ? (
        <div className="flex h-36 flex-col items-center justify-center rounded-lg border border-dashed border-border/40 text-muted-foreground">
          <LayoutGrid className="mb-2 h-6 w-6 opacity-20" />
          <p className="text-xs">{t('settings.blocks.no_results')}</p>
        </div>
      ) : (
        <div className="overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs divide-y divide-border/40">
          {filteredBlocks.map((block) => {
            const Icon = ICON_MAP[block.icon as IconName] || LayoutGrid
            const isEnabled = !!(modules as any)[block.id]

            return (
              <div
                key={block.id}
                className="group flex h-12 items-center px-4 transition-colors hover:bg-secondary/30"
              >
                {/* 1. Leading Icon + Name */}
                <div className="flex w-44 sm:w-52 shrink-0 items-center gap-2.5 pr-2">
                  <div className="flex size-6 shrink-0 items-center justify-center text-muted-foreground/70 transition-colors group-hover:text-foreground">
                    <Icon className="h-3.5 w-3.5" />
                  </div>
                  <span className="truncate text-xs font-medium text-foreground">
                    {t(`blocks.${block.id}.name`, block.name ?? '')}
                  </span>
                </div>

                {/* 2. Snippet / Description */}
                <div className="min-w-0 flex-1 pr-4">
                  <span className="truncate block text-xs font-light text-muted-foreground/70">
                    {t(`blocks.${block.id}.desc`, block.description ?? '')}
                  </span>
                </div>

                {/* 3. Actions: Tag, Status Micro-dot, Config, Switch */}
                <div className="flex shrink-0 items-center gap-3">
                  <span className="hidden sm:inline-block rounded px-1.5 py-0.5 text-[10px] font-normal text-muted-foreground/60 bg-secondary/40">
                    {t(`blocks.categories.${block.category.toLowerCase()}`, block.category)}
                  </span>

                  {isEnabled && (
                    <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shrink-0" title="Aktiv" />
                  )}

                  {CONFIGURABLE_BLOCKS.includes(block.id) && (
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 rounded-md text-muted-foreground hover:text-foreground"
                      onClick={(e) => {
                        e.stopPropagation()
                        setConfigBlock(block)
                      }}
                      title="Konfigurera modul"
                    >
                      <Settings2 className="h-3.5 w-3.5" />
                    </Button>
                  )}

                  <Switch
                    checked={isEnabled}
                    className="scale-85"
                    onCheckedChange={(checked) => handleToggle(block.id, checked)}
                  />
                </div>
              </div>
            )
          })}
        </div>
      )}

      {/* Configuration Dialog */}
      {(() => {
        const ConfigIcon = configBlock ? (ICON_MAP[configBlock.icon as IconName] || LayoutGrid) : null
        return (
          <Dialog open={!!configBlock} onOpenChange={() => setConfigBlock(null)}>
            <DialogContent className="max-h-[85vh] sm:max-w-2xl overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl">
              <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
                <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
                  {ConfigIcon && (
                    <ConfigIcon className="size-3.5 text-muted-foreground/70 shrink-0" />
                  )}
                  <span>{t(`blocks.${configBlock?.id}.name`, configBlock?.name ?? '')}</span>
                </DialogTitle>
              </DialogHeader>

          <div className="max-h-[calc(85vh-48px)] overflow-y-auto p-5">
            {configBlock?.id === 'scheduling' && (
              <SchedulerSettings setIsSaving={setIsSaving} />
            )}
            {configBlock?.id === 'messaging' && (
              <NotificationsSettings setIsSaving={setIsSaving} />
            )}
            {!['scheduling', 'messaging'].includes(configBlock?.id || '') && (
              <div className="flex flex-col items-center justify-center py-10 text-center text-muted-foreground">
                <Settings2 className="mb-2 size-6 text-muted-foreground/30" />
                <p className="text-xs">{t('settings.blocks.config_placeholder_desc')}</p>
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>
    )
  })()}
</div>
  )
}

export default BlockSettings
