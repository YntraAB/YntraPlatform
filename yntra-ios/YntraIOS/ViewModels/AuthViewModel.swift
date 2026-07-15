import SwiftUI
import yntra_core

@MainActor
class AuthViewModel: ObservableObject {
    @Published var isLoggedIn: Bool = false
    @Published var authRegion: String = "sv" // "sv" | "da" | "no" | "fi" | "en"
    @Published var bankIdFlowState: String = "idle" // "idle" | "qr_scan" | "verifying" | "success" | "error"
    @Published var qrData: String? = nil
    @Published var errorMessage: String? = nil

    private var activeSessionId: String? = nil
    private var isPolling = false

    func setRegion(_ region: String) {
        self.authRegion = region
    }

    func initiateBankIdLogin(provider: String) {
        self.errorMessage = nil
        self.bankIdFlowState = "connecting"
        
        Task {
            do {
                let session = try await initiateBankIdAuth(targetRole: "assistant", provider: provider)
                self.activeSessionId = session.id
                self.bankIdFlowState = session.status
                self.qrData = session.qrData
                self.startPollingStatus(sessionId: session.id)
            } catch {
                self.bankIdFlowState = "error"
                self.errorMessage = error.localizedDescription
            }
        }
    }

    private func startPollingStatus(sessionId: String) {
        guard !isPolling else { return }
        isPolling = true
        
        Task {
            while isPolling && self.bankIdFlowState != "success" && self.bankIdFlowState != "error" {
                try? await Task.sleep(nanoseconds: 2_000_000_000) // Sleep 2 seconds
                do {
                    if let session = try await getBankidAuthSession(sessionId: sessionId) {
                        self.bankIdFlowState = session.status
                        self.qrData = session.qrData
                        if session.status == "success" || session.status == "authenticated" {
                            self.isLoggedIn = true
                            self.isPolling = false
                        } else if session.status == "failed" || session.status == "error" {
                            self.errorMessage = "Authentication failed"
                            self.bankIdFlowState = "error"
                            self.isPolling = false
                        }
                    } else {
                        self.bankIdFlowState = "error"
                        self.errorMessage = "Session not found"
                        self.isPolling = false
                    }
                } catch {
                    self.bankIdFlowState = "error"
                    self.errorMessage = error.localizedDescription
                    self.isPolling = false
                }
            }
            self.isPolling = false
        }
    }

    func passwordLogin(email: String, pin: String) {
        self.errorMessage = nil
        Task {
            do {
                if let _ = try await verifyEmailPassword(email: email, password: pin) {
                    self.isLoggedIn = true
                } else {
                    self.errorMessage = "Invalid credentials"
                }
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func logout() {
        self.isLoggedIn = false
        self.bankIdFlowState = "idle"
        self.qrData = nil
        self.activeSessionId = nil
        self.isPolling = false
    }
}
