import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import i18n from '@/i18n/config'
import type { Session } from '@supabase/supabase-js'
import { supabase } from '@/lib/supabase'
import { isDemoMode, demoAuthUser } from '@/lib/demo'
import type { AuthState, SocialAuthProvider, UserRole } from '@/types'

interface AuthContextValue {
  isAuthenticated: boolean
  user: AuthState['user']
  isLoading: boolean
  error: string | null
  loginWithProvider: (provider: SocialAuthProvider) => Promise<boolean>
  logout: () => Promise<void>
  clearError: () => void
  simulateRole: (role: UserRole | null) => void
  isPlatformAdmin: boolean
}

const defaultAuthState: AuthState = {
  isAuthenticated: false,
  user: null,
  isLoading: true,
  error: null,
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined)

interface DbUser {
  id: string
  email: string
  full_name: string | null
  role: string | null
  workspace_id: string | null
  phone: string | null
  location: string | null
  client_id: string | null
  privacy_settings: import('@/types').UserPrivacySettings | null
}

async function getDbUser(userId: string): Promise<DbUser | null> {
  try {
    const { data, error } = await supabase
      .from('users')
      .select(
        'id, email, full_name, role, workspace_id, phone, location, client_id, privacy_settings',
      )
      .eq('id', userId)
      .maybeSingle()

    if (error) {
      return null
    }

    return data
  } catch {
    return null
  }
}

async function getAuthStateFromSession(session: Session | null): Promise<AuthState> {
  if (!session) {
    return {
      isAuthenticated: false,
      user: null,
      isLoading: false,
      error: null,
    }
  }

  return {
    isAuthenticated: true,
    user: {
      id: session.user.id,
      email: session.user.email || '',
      name: session.user.email || i18n.t('common.user'),
      role: (session.user.user_metadata?.role as UserRole) || 'user',
      last_sign_in_at: session.user.last_sign_in_at,
    },
    isLoading: false,
    error: null,
    user_metadata: session.user.user_metadata,
  }
}

function getErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback
}

function getAuthRedirectUrl() {
  return `${window.location.origin}/`
}

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { t } = useTranslation()
  const [authState, setAuthState] = useState<AuthState>(defaultAuthState)
  const [simulatedRole, setSimulatedRole] = useState<UserRole | null>(null)

  const isPlatformAdmin = useMemo(() => {
    return (
      authState.user_metadata?.role === 'platform_admin' ||
      authState.user?.role === 'platform_admin' ||
      simulatedRole !== null
    )
  }, [authState.user_metadata, authState.user, simulatedRole])

  useEffect(() => {
    if (isDemoMode()) {
      const isExplicitlyLoggedOut =
        sessionStorage.getItem('yntra_demo_logged_out') === 'true' ||
        localStorage.getItem('yntra_demo_logged_out') === 'true'

      if (isExplicitlyLoggedOut) {
        setAuthState({
          isAuthenticated: false,
          user: null,
          isLoading: false,
          error: null,
        })
        return
      }

      // Demo mode: sign in as the demo admin instantly, no backend needed.
      setAuthState({
        isAuthenticated: true,
        user: {
          id: demoAuthUser.id,
          email: demoAuthUser.email,
          name: 'Anna Andersson',
          role: 'admin',
          last_sign_in_at: demoAuthUser.last_sign_in_at,
        },
        isLoading: false,
        error: null,
        user_metadata: demoAuthUser.user_metadata,
      })
      return
    }

    let isMounted = true

    const syncSession = async (session: Session | null) => {
      try {
        const nextState = await getAuthStateFromSession(session)

        if (isMounted) {
          setAuthState(nextState)
        }

        if (session) {
          getDbUser(session.user.id)
            .then((dbUser) => {
              if (isMounted && dbUser) {
                setAuthState((prev: AuthState) => {
                  if (!prev.user) return prev
                  return {
                    ...prev,
                    user: {
                      ...prev.user!,
                      email: dbUser.email || prev.user!.email,
                      name: dbUser.full_name || prev.user!.email,
                      role: (dbUser.role as UserRole) || prev.user!.role,
                      workspaceId: dbUser.workspace_id || undefined,
                      phone: dbUser.phone || undefined,
                      location: dbUser.location || undefined,
                      client_id: dbUser.client_id || undefined,
                      privacy_settings: dbUser.privacy_settings as unknown as import('@/types').UserPrivacySettings,
                    },
                  }
                })
              }
            })
            .catch(() => {
              // silently
            })
        }
      } catch (error: unknown) {
        if (isMounted) {
          setAuthState({
            isAuthenticated: false,
            user: null,
            isLoading: false,
            error: getErrorMessage(
              error,
              t('auth.sync_error', 'Kunde inte uppdatera autentiseringssessionen.'),
            ),
          })
        }
      }
    }

    const initAuth = async () => {
      // If the auth endpoint is unreachable (e.g. blocked network), never stay
      // stuck on the loading screen — fall back to the login page promptly.
      const timeout = setTimeout(() => {
        if (isMounted) {
          setAuthState((prev: AuthState) =>
            prev.isLoading ? { ...prev, isLoading: false } : prev,
          )
        }
      }, 2500)

      try {
        const sessionPromise = supabase.auth.getSession()
        const timeoutPromise = new Promise<{ data: { session: null }; error: null }>((resolve) =>
          setTimeout(() => resolve({ data: { session: null }, error: null }), 2000),
        )
        const {
          data: { session },
          error,
        } = await Promise.race([sessionPromise, timeoutPromise])

        if (error) throw error
        await syncSession(session)
      } catch (error) {
        console.error('[AuthContext] Init error:', error)
        if (isMounted) {
          setAuthState((prev: AuthState) => ({ ...prev, isLoading: false }))
        }
      } finally {
        clearTimeout(timeout)
      }
    }

    initAuth()

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange(async (_event, session) => {
      if (isMounted) {
        await syncSession(session)
      }
    })

    return () => {
      isMounted = false
      subscription.unsubscribe()
    }
  }, [t])

  const loginWithProvider = useCallback(async (provider: SocialAuthProvider): Promise<boolean> => {
    setAuthState((prev: AuthState) => ({ ...prev, isLoading: true, error: null }))

    try {
      if (isDemoMode()) {
        sessionStorage.removeItem('yntra_demo_logged_out')
        localStorage.removeItem('yntra_demo_logged_out')
        // Brief simulated async pause for tactile feedback
        await new Promise((resolve) => setTimeout(resolve, 350))
        setAuthState({
          isAuthenticated: true,
          user: {
            id: demoAuthUser.id,
            email: demoAuthUser.email,
            name: 'Anna Andersson',
            role: 'admin',
            last_sign_in_at: new Date().toISOString(),
          },
          isLoading: false,
          error: null,
          user_metadata: demoAuthUser.user_metadata,
        })
        return true
      }

      const { error } = await supabase.auth.signInWithOAuth({
        provider,
        options: {
          redirectTo: getAuthRedirectUrl(),
        },
      })

      if (error) {
        throw error
      }

      return true
    } catch (error: unknown) {
      const message = getErrorMessage(
        error,
        t('auth.social_login_start_error', 'Social inloggning kunde inte startas.'),
      )

      setAuthState({
        isAuthenticated: false,
        user: null,
        isLoading: false,
        error: message,
      })

      return false
    }
  }, [t])

  const logout = useCallback(async () => {
    setAuthState((prev: AuthState) => ({ ...prev, isLoading: true, error: null }))

    try {
      if (isDemoMode()) {
        sessionStorage.setItem('yntra_demo_logged_out', 'true')
        localStorage.setItem('yntra_demo_logged_out', 'true')
      }

      const { error } = await supabase.auth.signOut()

      if (error) {
        throw error
      }

      setAuthState({
        isAuthenticated: false,
        user: null,
        isLoading: false,
        error: null,
      })
    } catch (error: unknown) {
      setAuthState((prev: AuthState) => ({
        ...prev,
        isLoading: false,
        error: getErrorMessage(error, t('auth.logout_failed', 'Utloggningen misslyckades.')),
      }))

      throw error
    }
  }, [t])

  const clearError = useCallback(() => {
    setAuthState((prev: AuthState) => ({ ...prev, error: null }))
  }, [])

  const simulateRole = useCallback((role: UserRole | null) => {
    setSimulatedRole(role)
  }, [])

  const userWithSimulation = useMemo(() => {
    if (!authState.user) return null
    if (!simulatedRole) return authState.user
    return {
      ...authState.user,
      role: simulatedRole,
    }
  }, [authState.user, simulatedRole])

  const value = useMemo<AuthContextValue>(
    () => ({
      isAuthenticated: authState.isAuthenticated,
      user: userWithSimulation,
      isLoading: authState.isLoading,
      error: authState.error,
      loginWithProvider,
      logout,
      clearError,
      simulateRole,
      isPlatformAdmin,
    }),
    [
      authState.error,
      authState.isAuthenticated,
      authState.isLoading,
      userWithSimulation,
      clearError,
      loginWithProvider,
      logout,
      simulateRole,
      isPlatformAdmin,
    ],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuthContext() {
  const context = useContext(AuthContext)

  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }

  return context
}
