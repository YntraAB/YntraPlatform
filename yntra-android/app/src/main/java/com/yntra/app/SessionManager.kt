package com.yntra.app

import uniffi.yntra_core.isSessionKeySet
import uniffi.yntra_core.clearSessionKey

object SessionManager {
    var activeUserId: String = "user-1"
    var activeWorkspaceId: String = "workspace-1"
    
    fun isUnlocked(): Boolean {
        return try {
            isSessionKeySet()
        } catch (e: Exception) {
            false
        }
    }
    
    fun lockSession() {
        try {
            clearSessionKey()
        } catch (e: Exception) {
            // Ignore if FFI not bound
        }
    }
}

