import SwiftUI
import yntra_core

@MainActor
class MessagingViewModel: ObservableObject {
    @Published var messages: [MessageItem] = []
    @Published var errorMessage: String? = nil
    
    private var observer: SwiftDbObserver?

    init() {
        loadMessages()
        
        self.observer = SwiftDbObserver { [weak self] in
            self?.loadMessages()
        }
        registerObserver(observer: self.observer!)
    }

    func loadMessages() {
        Task {
            do {
                self.messages = try await getMessages(requesterUserId: "user-1", userId: "user-1")
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func markAsRead(messageId: String) {
        Task {
            do {
                try await markMessageRead(requesterUserId: "user-1", id: messageId)
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func sendMessageToUser(receiverId: String, subject: String, body: String) {
        Task {
            do {
                _ = try await sendMessage(
                    requesterUserId: "user-1",
                    workspaceId: "workspace-1",
                    senderId: "user-1",
                    receiverId: receiverId,
                    teamId: nil,
                    subject: subject,
                    body: body
                )
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func sendMessageToTeam(teamId: String, subject: String, body: String) {
        Task {
            do {
                _ = try await sendMessage(
                    requesterUserId: "user-1",
                    workspaceId: "workspace-1",
                    senderId: "user-1",
                    receiverId: nil,
                    teamId: teamId,
                    subject: subject,
                    body: body
                )
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
}
