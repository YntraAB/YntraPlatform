import Foundation
import LocalAuthentication
import Security
import Combine

class BiometricAuthManager: ObservableObject {
    @Published var isBiometricsAvailable: Bool = false
    @Published var biometricType: LABiometryType = .none
    @Published var isAuthenticated: Bool = false
    @Published var errorMessage: String? = nil
    
    private let keyTag = "com.yntra.app.biometric.masterkey"
    
    init() {
        checkBiometricAvailability()
    }
    
    func checkBiometricAvailability() {
        let context = LAContext()
        var error: NSError?
        
        let available = context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error)
        DispatchQueue.main.async {
            self.isBiometricsAvailable = available
            self.biometricType = context.biometryType
            if let err = error {
                self.errorMessage = err.localizedDescription
            }
        }
    }
    
    func authenticate(workspaceId: String = "workspace-1", reason: String = "Authenticate to access Yntra Secure Mobile Enclave", completion: @escaping (Bool, String?) -> Void) {
        let context = LAContext()
        context.touchIDAuthenticationAllowableReuseDuration = 10
        var error: NSError?
        
        guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) else {
            let msg = error?.localizedDescription ?? "Biometric authentication hardware not available"
            DispatchQueue.main.async {
                self.errorMessage = msg
                completion(false, msg)
            }
            return
        }
        
        context.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, localizedReason: reason) { success, authenticationError in
            DispatchQueue.main.async {
                if success {
                    let account = "com.yntra.workspace.\(workspaceId).key"
                    var keyData = KeychainVault.shared.retrieveEncryptionKey(account: account, context: context)
                    if keyData == nil {
                        // Generate initial workspace session key
                        var rawBytes = [UInt8](repeating: 0, count: 32)
                        _ = SecRandomCopyBytes(kSecRandomDefault, 32, &rawBytes)
                        keyData = Data(rawBytes)
                        _ = KeychainVault.shared.saveEncryptionKey(keyData: keyData!, account: account)
                    }
                    
                    if let validKeyData = keyData {
                        let keyBytes = Array(validKeyData)
                        // Pass key bytes to Rust yntra-core via UniFFI
                        _ = setSessionKey(keyBytes: keyBytes, workspaceId: workspaceId)
                        self.isAuthenticated = true
                        self.errorMessage = nil
                        completion(true, nil)
                    } else {
                        self.isAuthenticated = false
                        self.errorMessage = "Failed to access Secure Enclave cryptographic key"
                        completion(false, self.errorMessage)
                    }
                } else {
                    let msg = authenticationError?.localizedDescription ?? "Biometric evaluation rejected"
                    self.isAuthenticated = false
                    self.errorMessage = msg
                    completion(false, msg)
                }
            }
        }
    }
}

