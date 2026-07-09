package com.yntra.app

import android.app.Application
import android.util.Log

class YntraApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        Log.i("YntraApplication", "Initializing Yntra Application")
        // Load the JNI library compiled from Rust core (libyntra_core.so)
        System.loadLibrary("yntra_core")
    }
}
