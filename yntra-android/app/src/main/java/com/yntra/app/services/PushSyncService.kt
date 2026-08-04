package com.yntra.app.services

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import androidx.core.app.NotificationCompat
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.yntra.app.R
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import uniffi.yntra_core.performOsBackgroundSync

class PushSyncService : FirebaseMessagingService() {
    private val serviceScope = CoroutineScope(Dispatchers.IO)

    override fun onNewToken(token: String) {
        super.onNewToken(token)
        // Store or register FCM token
    }

    override fun onMessageReceived(remoteMessage: RemoteMessage) {
        super.onMessageReceived(remoteMessage)

        val workspaceId = remoteMessage.data["workspace_id"] ?: "workspace-1"
        val pushType = remoteMessage.data["type"] ?: "silent_sync"

        // Trigger background libSQL database replication via UniFFI
        serviceScope.launch {
            try {
                let syncResult = performOsBackgroundSync(workspaceId)
                if (remoteMessage.notification != null) {
                    showLocalNotification(
                        title = remoteMessage.notification?.title ?: "Yntra Alert",
                        body = remoteMessage.notification?.body ?: "Workspace updated"
                    )
                }
            } catch (e: Exception) {
                // Log background sync failure
            }
        }
    }

    private fun showLocalNotification(title: String, body: String) {
        val channelId = "yntra_push_sync_channel"
        val notificationManager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                channelId,
                "Yntra Workspace Notifications",
                NotificationManager.IMPORTANCE_DEFAULT
            )
            notificationManager.createNotificationChannel(channel)
        }

        val notification = NotificationCompat.Builder(this, channelId)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(title)
            .setContentText(body)
            .setAutoCancel(true)
            .build()

        notificationManager.notify(System.currentTimeMillis().toInt(), notification)
    }
}
