import XCTest
import yntra_core
@testable import YntraIOS

class FfiBridgeTests: XCTestCase {

    override func setUpWithError() throws {
        try super.setUpWithError()
        
        // Setup mock database directory
        let tempDir = FileManager.default.temporaryDirectory.appendingPathComponent("yntra_ios_test_db")
        try? FileManager.default.removeItem(at: tempDir)
        try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        
        try setDatabaseDirectory(dirPath: tempDir.path)
    }

    func testFfiBridgeAndObserver() throws {
        let expectation = XCTestExpectation(description: "Database observer callback triggered")
        var callbackTriggered = false
        
        let observer = SwiftDbObserver {
            callbackTriggered = true
            expectation.fulfill()
        }
        
        registerObserver(observer: observer)
        
        defer {
            clearObservers()
        }
        
        // Add a todo item through the FFI boundary
        try addTodo(userId: "test-user-ios", workspaceId: "test-workspace-ios", text: "Swift FFI Test Todo")
        
        // Wait for observer callback notification
        wait(for: [expectation], timeout: 3.0)
        
        XCTAssertTrue(callbackTriggered, "Database change callback was not called")
        
        // Retrieve the list and assert the items are present
        let todos = try getTodos(userId: "test-user-ios", workspaceId: "test-workspace-ios")
        XCTAssertTrue(todos.contains(where: { $0.text == "Swift FFI Test Todo" }))
    }
}
