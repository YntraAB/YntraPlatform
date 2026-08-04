import Foundation
import Security
import LocalAuthentication

class KeychainVault {
    static let shared = KeychainVault()
    private let defaultAccountTag = "com.yntra.app.biometric.masterkey"
    
    private init() {}
    
    func saveEncryptionKey(keyData: Data, account: String = "com.yntra.app.biometric.masterkey") -> Bool {
        // First delete existing key if any
        deleteEncryptionKey(account: account)
        
        var accessError: Unmanaged<CFError>?
        guard let accessControl = SecAccessControlCreateWithFlags(
            kCFAllocatorDefault,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            [.userPresence, .biometryAny],
            &accessError
        ) else {
            print("Failed to create access control: \(String(describing: accessError))")
            return false
        }
        
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: account,
            kSecValueData as String: keyData,
            kSecAttrAccessControl as String: accessControl
        ]
        
        let status = SecItemAdd(query as CFDictionary, nil)
        return status == errSecSuccess
    }
    
    func retrieveEncryptionKey(account: String = "com.yntra.app.biometric.masterkey", context: LAContext? = nil) -> Data? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        if let laContext = context {
            query[kSecUseAuthenticationContext as String] = laContext
        }
        
        var dataTypeRef: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &dataTypeRef)
        
        if status == errSecSuccess, let data = dataTypeRef as? Data {
            return data
        }
        return nil
    }
    
    @discardableResult
    func deleteEncryptionKey(account: String = "com.yntra.app.biometric.masterkey") -> Bool {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: account
        ]
        let status = SecItemDelete(query as CFDictionary)
        return status == errSecSuccess || status == errSecItemNotFound
    }
    
    func ensureWorkspaceKeyExists(workspaceId: String) -> Data? {
        let account = "com.yntra.workspace.\(workspaceId).key"
        if let existing = retrieveEncryptionKey(account: account) {
            return existing
        }
        
        // Generate new secure 256-bit (32-byte) key
        var keyBytes = [UInt8](repeating: 0, count: 32)
        let result = SecRandomCopyBytes(kSecRandomDefault, 32, &keyBytes)
        guard result == errSecSuccess else { return nil }
        
        let keyData = Data(keyBytes)
        if saveEncryptionKey(keyData: keyData, account: account) {
            return keyData
        }
        return nil
    }
}
