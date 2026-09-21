import { describe, it, expect, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import {
  setDynamicBreadcrumbs,
  clearDynamicBreadcrumbs,
  useDynamicBreadcrumbs,
  useSetDynamicBreadcrumbs,
} from './BreadcrumbContext'

describe('BreadcrumbContext Store & Hooks', () => {
  beforeEach(() => {
    clearDynamicBreadcrumbs()
  })

  it('starts with empty breadcrumbs', () => {
    const { result } = renderHook(() => useDynamicBreadcrumbs())
    expect(result.current).toEqual([])
  })

  it('updates dynamic breadcrumbs when setDynamicBreadcrumbs is called', () => {
    const { result } = renderHook(() => useDynamicBreadcrumbs())

    act(() => {
      setDynamicBreadcrumbs([{ label: 'Allmänt', path: '/settings' }])
    })

    expect(result.current).toHaveLength(1)
    expect(result.current[0].label).toBe('Allmänt')
    expect(result.current[0].path).toBe('/settings')
  })

  it('does not trigger extra updates when identical visual breadcrumbs are passed', () => {
    let renderCount = 0
    renderHook(() => {
      renderCount++
      return useDynamicBreadcrumbs()
    })

    expect(renderCount).toBe(1)

    // Set initial
    act(() => {
      setDynamicBreadcrumbs([{ label: 'Team', path: '/directory' }])
    })
    expect(renderCount).toBe(2)

    // Set identical visual breadcrumbs with a new inline function reference
    act(() => {
      setDynamicBreadcrumbs([{ label: 'Team', path: '/directory', onClick: () => {} }])
    })
    // Render count must not increase because visuals did not change!
    expect(renderCount).toBe(2)

    // Set different label
    act(() => {
      setDynamicBreadcrumbs([{ label: 'Medlemmar', path: '/directory/members' }])
    })
    expect(renderCount).toBe(3)
  })

  it('provides a stable setter via useSetDynamicBreadcrumbs without re-rendering caller', () => {
    let renderCount = 0
    const { result } = renderHook(() => {
      renderCount++
      return useSetDynamicBreadcrumbs()
    })

    expect(renderCount).toBe(1)

    act(() => {
      result.current([{ label: 'Ny vy' }])
    })

    // Caller of useSetDynamicBreadcrumbs must NOT re-render
    expect(renderCount).toBe(1)
  })

  it('clears breadcrumbs properly', () => {
    const { result } = renderHook(() => useDynamicBreadcrumbs())

    act(() => {
      setDynamicBreadcrumbs([{ label: 'Test' }])
    })
    expect(result.current).toHaveLength(1)

    act(() => {
      clearDynamicBreadcrumbs()
    })
    expect(result.current).toEqual([])
  })
})
