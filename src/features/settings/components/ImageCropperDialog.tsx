import React, { useState, useCallback } from 'react'
import Cropper from 'react-easy-crop'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Slider } from '@/components/ui/slider'
import { getCroppedImg } from '@/lib/image-utils'
import type { PixelCrop } from '@/lib/image-utils'
import { useTranslation } from 'react-i18next'
import { Crop, Check } from 'lucide-react'

interface ImageCropperDialogProps {
  image: string
  open: boolean
  onClose: () => void
  onCropComplete: (croppedImage: Blob) => void
  aspect?: number
}

export const ImageCropperDialog: React.FC<ImageCropperDialogProps> = ({
  image,
  open,
  onClose,
  onCropComplete,
  aspect = 1,
}) => {
  const { t } = useTranslation()
  const [crop, setCrop] = useState({ x: 0, y: 0 })
  const [zoom, setZoom] = useState(1)
  const [croppedAreaPixels, setCroppedAreaPixels] = useState<PixelCrop | null>(null)

  const onCropChange = useCallback((crop: { x: number; y: number }) => {
    setCrop(crop)
  }, [])

  const onCropCompleteInternal = useCallback((_croppedArea: unknown, croppedAreaPixels: PixelCrop) => {
    setCroppedAreaPixels(croppedAreaPixels)
  }, [])

  const handleSave = async () => {
    if (!croppedAreaPixels) return
    try {
      const croppedImage = await getCroppedImg(image, croppedAreaPixels)
      if (croppedImage) {
        onCropComplete(croppedImage)
        onClose()
      }
    } catch (e) {
      console.error(e)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl sm:max-w-[480px]">
        <DialogHeader className="flex h-12 flex-row items-center justify-between space-y-0 border-b border-border bg-secondary/40 px-5 py-0">
          <DialogTitle className="flex items-center gap-2 text-xs font-medium text-foreground">
            <Crop className="size-3.5 text-muted-foreground/70 shrink-0" />
            <span>{t('settings.workspace.crop_logo', 'Crop Organization Logo')}</span>
          </DialogTitle>
        </DialogHeader>

        <div className="p-5">
          <div className="relative h-[260px] w-full overflow-hidden rounded-md border border-border bg-black/20">
            <Cropper
              image={image}
              crop={crop}
              zoom={zoom}
              aspect={aspect}
              onCropChange={onCropChange}
              onCropComplete={onCropCompleteInternal}
              onZoomChange={setZoom}
            />
          </div>

          <div className="mt-4 space-y-2">
            <div className="flex items-center justify-between text-[11px] font-medium text-muted-foreground">
              <span>Zoom</span>
              <span>{Math.round(zoom * 100)}%</span>
            </div>
            <Slider
              value={[zoom]}
              min={1}
              max={3}
              step={0.1}
              onValueChange={([val]) => setZoom(val)}
            />
          </div>
        </div>

        <DialogFooter className="flex items-center justify-end gap-2 border-t border-border bg-secondary/40 px-5 py-2.5">
          <Button
            variant="outline"
            size="sm"
            onClick={onClose}
            className="h-8 rounded-md border border-border bg-background px-3 text-xs font-medium text-foreground hover:bg-secondary"
          >
            {t('common.cancel')}
          </Button>
          <Button
            size="sm"
            onClick={handleSave}
            className="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground shadow-xs hover:bg-primary/90"
          >
            <Check className="mr-1.5 size-3" />
            <span>{t('common.save')}</span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
