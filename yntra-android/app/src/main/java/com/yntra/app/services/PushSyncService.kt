package com.yntra.app.services

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import androidx.core.app.NotificationCompat
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.yntra.app.SessionManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import uniffi.yntra_core.performOsBackgroundSync
import uniffi.yntra_core.registerDevicePushToken

class PushSyncService : FirebaseMessagingService() {
    private val serviceScope = CoroutineScope(Dispatchers.IO)

    override fun onNewToken(token: String) {
        super.onNewToken(token)
        serviceScope.launch {
            try {
                registerDevicePushToken(
                    requesterUserId = SessionManager.activeUserId,
                    deviceToken = token,
                    platform = "android"
                )
            } catch (e: Exception) {
                // Ignore token sync error
            }
        }
    }

    override fun onMessageReceived(remoteMessage: RemoteMessage) {
        super.onMessageReceived(remoteMessage)

        val workspaceId = remoteMessage.data["workspace_id"] ?: "workspace-1"

        // Trigger background libSQL database replication via UniFFI
        serviceScope.launch {
            try {
                val syncResult = performOsBackgroundSync(workspaceId)
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
