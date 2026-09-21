import { useEffect } from 'react'
import { useWorkspace } from '@/contexts/WorkspaceContext'

export function VisualEffectHandler() {
  const { preferences } = useWorkspace()

  useEffect(() => {
    if (!preferences?.accent_color) return

    const root = document.documentElement

    if (preferences.accent_color === 'primary') {
      root.style.removeProperty('--primary')
      root.style.removeProperty('--primary-foreground')
      return
    }

    const hsl = hexToHsl(preferences.accent_color)
    const hslString = `${hsl.h} ${hsl.s}% ${hsl.l}%`
    const foregroundString = hsl.l > 60 ? '0 0% 0%' : '0 0% 100%'

    root.style.setProperty('--primary', hslString)
    root.style.setProperty('--primary-foreground', foregroundString)
    root.style.setProperty('--ring', hslString)

    if (preferences.font_scale) {
      let scale = preferences.font_scale
      if (scale > 2) scale = scale / 100

      const clampedScale = Math.min(Math.max(scale, 0.5), 2.0)
      root.style.fontSize = `${clampedScale * 100}%`
    } else {
      root.style.fontSize = '100%'
    }
  }, [preferences?.accent_color, preferences?.font_scale])

  return null
}

function hexToHsl(hex: string) {
  let r = 0,
    g = 0,
    b = 0
  if (hex.length === 4) {
    r = parseInt(hex[1] + hex[1], 16)
    g = parseInt(hex[2] + hex[2], 16)
    b = parseInt(hex[3] + hex[3], 16)
  } else if (hex.length === 7) {
    r = parseInt(hex[1] + hex[2], 16)
    g = parseInt(hex[3] + hex[4], 16)
    b = parseInt(hex[5] + hex[6], 16)
  }
  r /= 255
  g /= 255
  b /= 255
  const max = Math.max(r, g, b),
    min = Math.min(r, g, b)
  let h = 0,
    s = 0
  const l = (max + min) / 2
  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0)
        break
      case g:
        h = (b - r) / d + 2
        break
      case b:
        h = (r - g) / d + 4
        break
    }
    h /= 6
  }
  return {
    h: Math.round(h * 360),
    s: Math.round(s * 100),
    l: Math.round(l * 100),
  }
}
