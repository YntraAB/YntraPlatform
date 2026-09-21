import { supabase } from '@/lib/supabase'

export interface Client {
  id: string
  first_name: string
  last_name: string
  personal_number?: string
  care_level?: string
  message_settings?: Record<string, unknown>
}

export interface Medication {
  id: string
  client_id: string
  name: string
  dosage?: string
  frequency?: string
  instructions?: string
}

export interface JournalNote {
  id: string
  client_id: string
  author_id?: string
  content: string
  created_at: string
  author?: {
    full_name: string | null
  }
}

export const assistanceService = {
  /**
   * Fetch all clients for the current workspace
   */
  async getClients() {
    const { data, error } = await supabase
      .from('clients')
      .select('*')
      .order('last_name', { ascending: true })

    if (error) throw new Error(error.message)
    return (data || []) as Client[]
  },

  /**
   * Fetch a specific client by ID
   */
  async getClient(clientId: string) {
    const { data, error } = await supabase
      .from('clients')
      .select('*')
      .eq('id', clientId)
      .single()

    if (error) throw new Error(error.message)
    return data as Client
  },

  /**
   * Fetch medications for a client
   */
  async getMedications(clientId: string) {
    const { data, error } = await supabase
      .from('client_medications')
      .select('*')
      .eq('client_id', clientId)

    if (error) throw new Error(error.message)
    return (data || []) as Medication[]
  },

  /**
   * Fetch journal notes for a client
   */
  async getJournalNotes(clientId: string) {
    const { data, error } = await supabase
      .from('client_journals')
      .select('*, author:users(full_name)')
      .eq('client_id', clientId)
      .order('created_at', { ascending: false })

    if (error) throw new Error(error.message)
    return (data || []) as JournalNote[]
  },

  /**
   * Create a new journal note
   */
  async createJournalNote(note: Partial<JournalNote>) {
    const { data, error } = await supabase
      .from('client_journals')
      .insert(note)
      .select('*, author:users(full_name)')
      .single()

    if (error) throw new Error(error.message)
    return data as JournalNote
  },
}
