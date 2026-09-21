import { QueryClient } from '@tanstack/react-query'
import { persistQueryClient, type Persister } from '@tanstack/react-query-persist-client'
import { get, set, del } from 'idb-keyval'

/**
 * Standard Query Client configuration
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // Stale time set to 5 minutes by default
      staleTime: 1000 * 60 * 5,
      // Keep unused data in cache for 24 hours
      gcTime: 1000 * 60 * 60 * 24,
      // Retry failed queries 3 times
      retry: 3,
      // Reference: https://tanstack.com/query/v5/docs/react/guides/network-mode
      networkMode: 'offlineFirst',
    },
    mutations: {
      networkMode: 'offlineFirst',
    },
  },
})

/**
 * IndexedDB Persister using idb-keyval
 * This allows storing larger amounts of data than localStorage
 */
const indexedDBPersister: Persister = {
  persistClient: async (client) => {
    await set('yntra-query-cache', client)
  },
  restoreClient: async () => {
    return await get('yntra-query-cache')
  },
  removeClient: async () => {
    await del('yntra-query-cache')
  },
}

/**
 * Initialize persistence
 */
persistQueryClient({
  queryClient,
  persister: indexedDBPersister,
  maxAge: 1000 * 60 * 60 * 24,
})
