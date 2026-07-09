import SwiftUI
import yntra_core

@MainActor
class CareViewModel: ObservableObject {
    @Published var clients: [ClientProfile] = []
    @Published var journals: [JournalEntry] = []
    @Published var medications: [MedicationItem] = []
    @Published var errorMessage: String? = nil
    
    private var observer: SwiftDbObserver?

    init() {
        loadClients()
        
        self.observer = SwiftDbObserver { [weak self] in
            self?.loadClients()
        }
        registerObserver(observer: self.observer!)
    }

    func loadClients() {
        Task {
            do {
                self.clients = try await getClients(requesterUserId: "user-1")
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func loadHealthRecords(clientId: String) {
        Task {
            do {
                self.journals = try await getJournals(clientId: clientId, actorId: "user-1")
                self.medications = try await getMedications(clientId: clientId, actorId: "user-1")
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func addJournal(clientId: String, content: String) {
        Task {
            do {
                _ = try await addJournalEntry(workspaceId: "workspace-1", clientId: clientId, authorId: "user-1", content: content)
                loadHealthRecords(clientId: clientId)
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func addMed(clientId: String, name: String, dosage: String, frequency: String, instructions: String) {
        Task {
            do {
                _ = try await addMedication(
                    workspaceId: "workspace-1",
                    clientId: clientId,
                    actorId: "user-1",
                    name: name,
                    dosage: dosage,
                    frequency: frequency,
                    instructions: instructions
                )
                loadHealthRecords(clientId: clientId)
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
}
