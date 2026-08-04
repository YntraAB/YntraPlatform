import Foundation
import UserNotifications
import UIKit
import yntra_core

class PushSyncManager: NSObject, UNUserNotificationCenterDelegate {
    static let shared = PushSyncManager()

    @Published var deviceToken: String? = nil
    @Published var lastSyncResult: MobileSyncResult? = nil

    private override init() {
        super.init()
    }

    func registerForPushNotifications(userId: String = "user-1") {
        UNUserNotificationCenter.current().delegate = self
        let options: UNAuthorizationOptions = [.alert, .sound, .badge]

        UNUserNotificationCenter.current().requestAuthorization(options: options) { granted, error in
            if granted {
                DispatchQueue.main.async {
                    UIApplication.shared.registerForRemoteNotifications()
                }
            } else if let err = error {
                print("APNs push authorization failed: \(err.localizedDescription)")
            }
        }
    }

    func handleDeviceToken(_ tokenData: Data, userId: String = "user-1") {
        let tokenParts = tokenData.map { data in String(format: "%02.2hhx", data) }
        let token = tokenParts.joined()
        DispatchQueue.main.async {
            self.deviceToken = token
        }

        Task {
            _ = try? await registerDevicePushToken(
                requesterUserId: userId,
                deviceToken: token,
                platform: "ios"
            )
        }
    }

    func handleSilentPush(userInfo: [AnyHashable: Any], completionHandler: @escaping (UIBackgroundFetchResult) -> Void) {
        let workspaceId = userInfo["workspace_id"] as? String ?? "workspace-1"

        Task {
            do {
                let syncResult = try await performOsBackgroundSync(workspaceId: workspaceId)
                DispatchQueue.main.async {
                    self.lastSyncResult = syncResult
                    if syncResult.success {
                        completionHandler(.newData)
                    } else {
                        completionHandler(.failed)
                    }
                }
            } catch {
                DispatchQueue.main.async {
                    completionHandler(.failed)
                }
            }
        }
    }

    // UNUserNotificationCenterDelegate - Receive when app is in foreground
    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification, withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        let userInfo = notification.request.content.userInfo
        let workspaceId = userInfo["workspace_id"] as? String ?? "workspace-1"

        Task {
            _ = try? await performOsBackgroundSync(workspaceId: workspaceId)
        }

        completionHandler([.banner, .sound, .badge])
    }

    // Handle user interaction with notification banner / actions
    func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse, withCompletionHandler completionHandler: @escaping () -> Void) {
        let userInfo = response.notification.request.content.userInfo
        let workspaceId = userInfo["workspace_id"] as? String ?? "workspace-1"

        Task {
            _ = try? await performOsBackgroundSync(workspaceId: workspaceId)
            DispatchQueue.main.async {
                completionHandler()
            }
        }
    }
}
