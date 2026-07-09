package com.yntra.app

import android.os.Handler
import android.os.Looper
import uniffi.yntra_core.DatabaseObserver

class KotlinDbObserver(private val onUpdate: () -> Unit) : DatabaseObserver {
    private val mainHandler = Handler(Looper.getMainLooper())
    
    override fun onDatabaseChanged() {
        mainHandler.post {
            onUpdate()
        }
    }
    
    override fun onTableChanged(table: String) {
        mainHandler.post {
            onUpdate()
        }
    }
}
