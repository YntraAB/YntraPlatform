import { createClient, type SupabaseClient } from '@supabase/supabase-js'
import { isDemoMode } from './demo'
import { demoSupabase } from './demo-client'

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY

if (!isDemoMode() && (!supabaseUrl || !supabaseAnonKey)) {
  console.warn(
    '[Yntra] Saknar Supabase miljövariabler. Applikationen kommer inte kunna hämta data från databasen.',
  )
}

/**
 * In demo mode the app runs entirely on an in-memory data layer —
 * no network calls are made.
 */
export const supabase: SupabaseClient = isDemoMode()
  ? (demoSupabase as SupabaseClient)
  : createClient(
      supabaseUrl || 'https://placeholder-url.supabase.co',
      supabaseAnonKey || 'placeholder-key',
    )
