import SwiftUI
import yntra_core

@MainActor
class DashboardViewModel: ObservableObject {
    @Published var events: [TeamEvent] = []
    @Published var todosCount: Int = 0
    @Published var completedTodosCount: Int = 0
    @Published var errorMessage: String? = nil
    
    private var observer: SwiftDbObserver?

    init() {
        refreshDashboardData()
        
        self.observer = SwiftDbObserver { [weak self] in
            self?.refreshDashboardData()
        }
        registerObserver(observer: self.observer!)
    }

    func refreshDashboardData() {
        Task {
            do {
                self.events = try await getEvents(requesterUserId: "user-1", teamId: nil)
                let todos = try await getTodos(requesterUserId: "user-1", workspaceId: "workspace-1")
                self.todosCount = todos.count
                self.completedTodosCount = todos.filter { $0.completed }.count
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
}
