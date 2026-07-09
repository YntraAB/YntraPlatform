import SwiftUI
import yntra_core

@MainActor
class DirectoryViewModel: ObservableObject {
    @Published var users: [WorkspaceUser] = []
    @Published var teams: [Team] = []
    @Published var errorMessage: String? = nil
    
    private var observer: SwiftDbObserver?

    init() {
        refreshDirectory()
        
        self.observer = SwiftDbObserver { [weak self] in
            self?.refreshDirectory()
        }
        registerObserver(observer: self.observer!)
    }

    func refreshDirectory() {
        Task {
            do {
                self.users = try await getUsers(requesterUserId: "user-1")
                self.teams = try await getTeams(requesterUserId: "user-1")
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func inviteUser(email: String, name: String, role: String) {
        Task {
            do {
                _ = try await inviteUserViaDirectory(
                    requesterUserId: "user-1",
                    workspaceId: "workspace-1",
                    email: email,
                    name: name,
                    role: role
                )
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func createTeam(name: String) {
        Task {
            do {
                _ = try await addTeamViaDirectory(
                    requesterUserId: "user-1",
                    workspaceId: "workspace-1",
                    name: name
                )
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func deleteWorkspaceUser(userId: String) {
        Task {
            do {
                try await deleteUser(requesterUserId: "user-1", userId: userId)
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
}
