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

@RunWith(AndroidJUnit4::class)
class FfiBridgeTest {

    @Before
    fun setUp() {
        // Load the Rust library
        System.loadLibrary("yntra_core")

        // Set mock storage directory to context's cache path
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dbDir = File(context.cacheDir, "yntra_test_db")
        dbDir.deleteRecursively()
        dbDir.mkdirs()
        setDatabaseDirectory(dbDir.absolutePath)
    }

    @Test
    fun testFfiBridgeAndObserver() {
        val latch = CountDownLatch(1)
        var callbackTriggered = false

        val testObserver = object : DatabaseObserver {
            override fun onDatabaseChanged() {
                callbackTriggered = true
                latch.countDown()
            }
            override fun onTableChanged(table: String) {
                callbackTriggered = true
                latch.countDown()
            }
        }

        // Register the observer
        registerObserver(testObserver)

        try {
            // Perform basic operation to trigger database update notification
            addTodo(userId = "test-user", workspaceId = "test-workspace", text = "FFI Test Todo")
            
            // Wait for observer callback
            latch.await(3, TimeUnit.SECONDS)
            
            assertTrue("Expected database observer callback to be triggered", callbackTriggered)

            // Verify getTodos returns the created item
            val todos = getTodos(userId = "test-user", workspaceId = "test-workspace")
            assertTrue(todos.any { it.text == "FFI Test Todo" })
        } finally {
            clearObservers()
        }
    }
}
