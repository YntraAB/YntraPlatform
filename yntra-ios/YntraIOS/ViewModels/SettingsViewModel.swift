import SwiftUI
import yntra_core

@MainActor
class SettingsViewModel: ObservableObject {
    @Published var workspace: Workspace? = nil
    @Published var errorMessage: String? = nil
    
    private var observer: SwiftDbObserver?

    init() {
        loadSettings()
        
        self.observer = SwiftDbObserver { [weak self] in
            self?.loadSettings()
        }
        registerObserver(observer: self.observer!)
    }

    func loadSettings() {
        Task {
            do {
                self.workspace = try await getWorkspace()
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func updateProfile(fullName: String, phone: String, languagePreference: String) {
        Task {
            do {
                try await updateUserProfile(
                    requesterUserId: "user-1",
                    userId: "user-1",
                    fullName: fullName,
                    phone: phone,
                    preferences: "{\"language\":\"\(languagePreference)\"}"
                )
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
}
