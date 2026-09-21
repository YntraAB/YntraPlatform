export const queryKeys = {
  auth: ['auth'] as const,

  workspace: (id: string | null) => ['workspace', id] as const,
  workspaceUsers: (workspaceId: string | null) => ['users', workspaceId] as const,
  workspaceTeams: (workspaceId: string | null) => ['teams', workspaceId] as const,

  userPreferences: (userId: string | null) => ['user-preferences', userId] as const,
  messages: (workspaceId: string | undefined) => ['messages', workspaceId] as const,
  noteTeams: (scope: string | null) => ['note-teams', scope] as const,
  teamNotes: (teamId: string | null) => ['team-notes', teamId] as const,

  events: (workspaceId: string | null, teamId: string | null) =>
    ['events', workspaceId, teamId] as const,
  workspaceNotes: (workspaceId: string | null) => ['notes', workspaceId] as const,
  reporting: (workspaceId: string | null) => ['reports', workspaceId] as const,
  reportingStats: (workspaceId: string | null) => ['reporting-stats', workspaceId] as const,
} as const
