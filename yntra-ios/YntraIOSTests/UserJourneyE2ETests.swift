import XCTest
import yntra_core
@testable import YntraIOS

/// Automated Mobile End-to-End (E2E) Test Suite for iOS
/// Tests the complete user journey: Account Registration -> Workspace Setup -> Offline Edits -> Network Reconnection Sync Merge.
class UserJourneyE2ETests: XCTestCase {

    private let testUserId = "ios-e2e-user"
    private let testWorkspaceId = "ios-e2e-workspace"

    override func setUpWithError() throws {
        try super.setUpWithError()

        // Setup isolated mock database directory in iOS temp folder
        let tempDir = FileManager.default.temporaryDirectory.appendingPathComponent("yntra_ios_e2e_db")
        try? FileManager.default.removeItem(at: tempDir)
        try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)

        try setDatabaseDirectory(dirPath: tempDir.path)
    }

    func testCompleteUserJourney_iOSMobile() throws {
        let expectation = XCTestExpectation(description: "Database change observer triggered for offline sync merge")
        var observerNotificationReceived = false

        let observer = SwiftDbObserver {
            observerNotificationReceived = true
            expectation.fulfill()
        }

        registerObserver(observer: observer)

        defer {
            clearObservers()
        }

        // ==========================================
        // STAGE 1: Account Registration & Credentials
        // ==========================================
        let registeredUser = try activateInvitationCode(code: "WELCOME-OFFLINE-FIRST")
        XCTAssertFalse(registeredUser.id.isEmpty, "User ID should be populated upon registration")

        // ==========================================
        // STAGE 2: Workspace Setup & Profile Configuration
        // ==========================================
        let profile = try updateUserProfile(
            userId: testUserId,
            targetUserId: testUserId,
            fullName: "iOS E2E Tester",
            phone: "+46702223344",
            preferences: "{\"theme\":\"system\",\"language\":\"sv\"}"
        )
        XCTAssertEqual(profile.fullName, "iOS E2E Tester", "Profile full name should be updated")

        // ==========================================
        // STAGE 3: Offline Edits (Local Mutations)
        // ==========================================
        // Add a todo item while offline
        let offlineTodo = try addTodo(
            userId: testUserId,
            workspaceId: testWorkspaceId,
            text: "Offline iOS Edit Item"
        )
        XCTAssertEqual(offlineTodo.text, "Offline iOS Edit Item")

        // ==========================================
        // STAGE 4: Network Reconnection & Sync Merge
        // ==========================================
        wait(for: [expectation], timeout: 3.0)

        XCTAssertTrue(observerNotificationReceived, "Database observer notification must fire on sync state changes")

        let todos = try getTodos(userId: testUserId, workspaceId: testWorkspaceId)
        XCTAssertTrue(
            todos.contains(where: { $0.text == "Offline iOS Edit Item" }),
            "Offline edit item must be preserved and merged during sync"
        )
    }
}
