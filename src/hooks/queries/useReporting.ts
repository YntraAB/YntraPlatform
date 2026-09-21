import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { reportingService } from '@/services/reportingService'
import { queryKeys } from '@/lib/query-keys'

export function useReports(workspaceId: string | null, options: { isAdmin: boolean; userId: string; status?: string; type?: string }) {
  return useQuery({
    queryKey: [...queryKeys.reporting(workspaceId), options],
    queryFn: () => {
      if (!workspaceId) return []
      return reportingService.getReports(workspaceId, options)
    },
    enabled: !!workspaceId,
  })
}

export function useUpdateReportStatus() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ reportId, status }: { reportId: string; status: string }) =>
      reportingService.updateReportStatus(reportId, status),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['reports'] })
      queryClient.invalidateQueries({ queryKey: ['reporting-stats'] })
    },
  })
}

export function useCreateReport() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (report: Record<string, unknown>) => reportingService.createReport(report),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['reports'] })
      queryClient.invalidateQueries({ queryKey: ['reporting-stats'] })
    },
  })
}

export function useReportingStats(workspaceId: string | null) {
  return useQuery({
    queryKey: queryKeys.reportingStats(workspaceId),
    queryFn: () => {
      if (!workspaceId) return []
      return reportingService.getReportingStats(workspaceId)
    },
    enabled: !!workspaceId,
  })
}
