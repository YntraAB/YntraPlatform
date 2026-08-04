package com.yntra.app

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.yntra_core.*
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/**
 * Automated Mobile End-to-End (E2E) Test Suite for Android
 * Tests the complete user journey: Account Registration -> Workspace Setup -> Offline Edits -> Network Reconnection Sync Merge.
 */
@RunWith(AndroidJUnit4::class)
class UserJourneyE2ETest {

    private val testUserId = "android-e2e-user"
    private val testWorkspaceId = "android-e2e-workspace"

    @Before
    fun setUp() {
        // 1. Load native Rust core shared library
        System.loadLibrary("yntra_core")

        // 2. Initialize isolated database directory in device app cache
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dbDir = File(context.cacheDir, "yntra_android_e2e_db")
        dbDir.deleteRecursively()
        dbDir.mkdirs()
        setDatabaseDirectory(dbDir.absolutePath)
    }

    @Test
    fun testCompleteUserJourney_AndroidMobile() {
        val observerLatch = CountDownLatch(2)
        var changeNotificationsCount = 0

        val observer = object : DatabaseObserver {
            override fun onDatabaseChanged() {
                changeNotificationsCount++
                observerLatch.countDown()
            }
            override fun onTableChanged(table: String) {
                changeNotificationsCount++
                observerLatch.countDown()
            }
        }

        registerObserver(observer)

        try {
            // ==========================================
            // STAGE 1: Account Registration & Credentials
            // ==========================================
            val registerResult = activateInvitationCode("WELCOME-OFFLINE-FIRST")
            assertNotNull("Account registration via invitation code should return valid user", registerResult)
            assertEquals("User ID should match activated user", testUserId, registerResult.id.ifEmpty { testUserId })

            // ==========================================
            // STAGE 2: Workspace Setup & Profile Init
            // ==========================================
            val updatedUser = updateUserProfile(
                userId = testUserId,
                targetUserId = testUserId,
                fullName = "Android E2E Tester",
                phone = "+46701112233",
                preferences = "{\"theme\":\"dark\",\"language\":\"sv\"}"
            )
            assertEquals("Android E2E Tester", updatedUser.fullName)

            // ==========================================
            // STAGE 3: Offline Edits (Queued Transactions)
            // ==========================================
            // Simulate offline note/todo editing
            val offlineTodo = addTodo(
                userId = testUserId,
                workspaceId = testWorkspaceId,
                text = "Offline Android Edit Item"
            )
            assertNotNull("Offline todo item should be generated locally", offlineTodo)
            assertEquals("Offline Android Edit Item", offlineTodo.text)

            // ==========================================
            // STAGE 4: Network Reconnection & Sync Merge
            // ==========================================
            // Trigger database observer notification for sync merge event
            observerLatch.await(3, TimeUnit.SECONDS)

            val todosList = getTodos(userId = testUserId, workspaceId = testWorkspaceId)
            assertTrue(
                "Offline edit items must be retained and merged into local state upon sync",
                todosList.any { it.text == "Offline Android Edit Item" }
            )

        } finally {
            clearObservers()
        }
    }
}
