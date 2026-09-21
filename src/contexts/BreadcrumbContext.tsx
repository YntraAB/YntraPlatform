import React, { useSyncExternalStore, type ReactNode } from 'react'

export interface DynamicBreadcrumbItem {
  label: string
  path?: string
  onClick?: () => void
}

let breadcrumbsStore: DynamicBreadcrumbItem[] = []
const listeners = new Set<() => void>()

function areBreadcrumbsVisuallyEqual(
  a: DynamicBreadcrumbItem[],
  b: DynamicBreadcrumbItem[],
): boolean {
  if (a === b) return true
  if (a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) {
    if (a[i].label !== b[i].label || a[i].path !== b[i].path) {
      return false
    }
  }
  return true
}

const subscribe = (listener: () => void) => {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

const getSnapshot = () => breadcrumbsStore

export const setDynamicBreadcrumbs = (next: DynamicBreadcrumbItem[]) => {
  if (areBreadcrumbsVisuallyEqual(breadcrumbsStore, next)) {
    // Keep updated callbacks without triggering unnecessary subscriber re-renders
    breadcrumbsStore = next
    return
  }
  breadcrumbsStore = next
  listeners.forEach((listener) => listener())
}

export const clearDynamicBreadcrumbs = () => {
  if (breadcrumbsStore.length === 0) return
  breadcrumbsStore = []
  listeners.forEach((listener) => listener())
}

export const useDynamicBreadcrumbs = (): DynamicBreadcrumbItem[] => {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot)
}

export const useSetDynamicBreadcrumbs = () => {
  return setDynamicBreadcrumbs
}

export const BreadcrumbProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  return <>{children}</>
}

// For backwards compatibility
export const useBreadcrumbContext = () => {
  const dynamicBreadcrumbs = useDynamicBreadcrumbs()
  return {
    dynamicBreadcrumbs,
    setDynamicBreadcrumbs,
  }
}
