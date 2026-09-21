import React from 'react'
import { User, Phone, Mail, MapPin, AlertTriangle, FileText, Lock } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { useAuth } from '@/hooks/useAuth'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import type { MemberItem } from '../hooks/useDirectoryData'

interface MemberDetailSheetProps {
  member: MemberItem | null
  onClose: () => void
  userRole: string
  onEdit: (member: MemberItem) => void
}

export const MemberDetailSheet: React.FC<MemberDetailSheetProps> = ({
  member,
  onClose,
  userRole,
  onEdit,
}) => {
  const { t } = useTranslation()
  const { user: viewer } = useAuth()

  if (!member) return null
  const isPatient = member.role === 'Patient'
  const isSelf = viewer?.id === member.id
  const canSeeAll = userRole === 'admin' || userRole === 'platform_admin' || isSelf

  // Privacy checks
  const showPhone =
    canSeeAll ||
    member.privacy_settings?.phone === 'everyone' ||
    member.privacy_settings?.phone === 'organization' // Since we only show members of the same org in the directory anyway

  const showLocation =
    canSeeAll ||
    member.privacy_settings?.location === 'everyone' ||
    member.privacy_settings?.location === 'organization'

  let headerGradient = 'from-primary/20'
  if (isPatient) headerGradient = 'from-violet-500/20'

  return (
    <Sheet open={!!member} onOpenChange={(open) => !open && onClose()}>
      <SheetContent className="scrollbar-dark flex h-full w-full flex-col overflow-y-auto border-l-border bg-sidebar p-0 text-foreground sm:max-w-md">
        <div className="relative border-b border-border bg-card p-6">
          <div
            className={`absolute left-0 top-0 h-32 w-full bg-gradient-to-b ${headerGradient} pointer-events-none to-transparent`}
          />

          <SheetHeader className="relative z-10 pt-4">
            <div className="flex flex-col items-center text-center">
              <Avatar className="mb-3 h-20 w-20 border-4 border-border shadow-sm">
                <AvatarImage src={member.avatar} />
                <AvatarFallback className="text-xl">{member.name.charAt(0)}</AvatarFallback>
              </Avatar>
              <SheetTitle className="text-xl text-foreground">{member.name}</SheetTitle>

              {isPatient ? (
                <SheetDescription className="mt-1 flex items-center justify-center gap-1.5 text-xs text-muted-foreground">
                  <User className="h-3 w-3" /> {member.ssn}
                </SheetDescription>
              ) : (
                <Badge
                  variant="outline"
                  className="mt-2 border-primary/30 bg-primary/10 text-[10px] capitalize text-primary"
                >
                  {member.role === 'platform_admin'
                    ? t('directory.roles.platform_admin')
                    : member.role}
                </Badge>
              )}
            </div>
          </SheetHeader>

          {isPatient && (
            <div className="mt-4 flex justify-center gap-2">
              <Badge variant="secondary" className="bg-muted text-[10px] text-muted-foreground">
                {t('settings.font_scale')}: {member.careLevel}
              </Badge>
            </div>
          )}
        </div>

        <div className="flex-1 space-y-6 p-6">
          {/* Contact Info */}
          <div className="space-y-3">
            <h4 className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
              {t('directory.detail.info_title')}
            </h4>
            <div className="space-y-2 rounded-lg border border-border bg-background p-3">
              {/* Phone */}
              <div className="flex items-center justify-between text-xs">
                <div className="flex items-center gap-3">
                  <Phone className="h-3.5 w-3.5 text-muted-foreground" />
                  {showPhone ? (
                    <span className="text-foreground">
                      {member.phone || t('directory.detail.phone_unspecified')}
                    </span>
                  ) : (
                    <span className="flex items-center gap-1.5 italic text-muted-foreground">
                      <Lock className="h-3 w-3" /> {t('directory.detail.private')}
                    </span>
                  )}
                </div>
              </div>

              {/* Email */}
              {!isPatient && member.email && (
                <div className="mt-2 flex items-center gap-3 border-t border-border pt-2 text-xs">
                  <Mail className="h-3.5 w-3.5 text-muted-foreground" />
                  <span className="text-foreground">{member.email}</span>
                </div>
              )}

              {/* Location/Address */}
              {(isPatient || member.location) && (
                <div className="mt-2 flex items-center gap-3 border-t border-border pt-2 text-xs">
                  <MapPin className="h-3.5 w-3.5 text-muted-foreground" />
                  {isPatient ? (
                    <span className="text-foreground">{member.address}</span>
                  ) : showLocation ? (
                    <span className="text-foreground">
                      {member.location || t('directory.detail.location_unspecified')}
                    </span>
                  ) : (
                    <span className="flex items-center gap-1.5 italic text-muted-foreground">
                      <Lock className="h-3 w-3" /> {t('directory.detail.private')}
                    </span>
                  )}
                </div>
              )}
            </div>
          </div>

          {/* Alerts */}
          {member.alerts && member.alerts.length > 0 && (
            <div className="space-y-3">
              <h4 className="text-[10px] font-semibold uppercase tracking-wider text-rose-500/80">
                {t('directory.detail.warnings_title')}
              </h4>
              <div className="rounded-lg border border-rose-500/20 bg-rose-500/10 p-3">
                {member.alerts.map((alert: string, i: number) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 text-xs font-medium text-rose-400"
                  >
                    <AlertTriangle className="h-3.5 w-3.5" />
                    {alert}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Notes */}
          {isPatient && member.notes && (
            <div className="space-y-3">
              <h4 className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                {t('directory.detail.care_plan_title')}
              </h4>
              <div className="rounded-lg border border-border bg-background p-4">
                <p className="text-xs leading-relaxed text-muted-foreground">{member.notes}</p>
              </div>
            </div>
          )}
        </div>

        {/* Action Footer */}
        {(isPatient || userRole === 'platform_admin' || userRole === 'admin') && (
          <div className="flex gap-2 border-t border-border bg-muted/30 p-4">
            {isPatient && (
              <Button
                size="sm"
                className="h-9 flex-1 bg-violet-600 text-xs text-foreground hover:bg-violet-700"
              >
                <FileText className="mr-2 h-3.5 w-3.5" />
                {t('directory.detail.journal_button')}
              </Button>
            )}
            {(userRole === 'platform_admin' || userRole === 'admin') && (
              <Button
                size="sm"
                variant="outline"
                onClick={() => member && onEdit(member)}
                className={`h-9 flex-1 border border-border bg-transparent text-xs text-foreground hover:bg-muted ${!isPatient && 'w-full'}`}
              >
                {t('directory.detail.edit_button', {
                  type: isPatient ? t('directory.detail.patient') : t('directory.detail.staff'),
                })}
              </Button>
            )}
          </div>
        )}
      </SheetContent>
    </Sheet>
  )
}
