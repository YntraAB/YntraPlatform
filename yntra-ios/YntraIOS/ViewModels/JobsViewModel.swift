import SwiftUI
import yntra_core

@MainActor
class JobsViewModel: ObservableObject {
    @Published var jobs: [JobTicket] = []
    @Published var errorMessage: String? = nil
    
    private var observer: SwiftDbObserver?

    init() {
        loadJobs()
        
        self.observer = SwiftDbObserver { [weak self] in
            self?.loadJobs()
        }
        registerObserver(observer: self.observer!)
    }

    func loadJobs() {
        Task {
            do {
                self.jobs = try await getJobTickets(requesterUserId: "user-1")
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func updateStatus(jobId: String, status: String) {
        Task {
            do {
                try await updateJobStatus(requesterUserId: "user-1", jobId: jobId, status: status)
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func completeJob(jobId: String, checklistJson: String, completionReport: String) {
        Task {
            do {
                try await submitJobCompletion(
                    requesterUserId: "user-1",
                    jobId: jobId,
                    checklistJson: checklistJson,
                    completionReport: completionReport
                )
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
}
