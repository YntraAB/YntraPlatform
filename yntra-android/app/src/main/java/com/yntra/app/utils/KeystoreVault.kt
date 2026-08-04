package com.yntra.app.utils

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import java.security.SecureRandom

object KeystoreVault {
    private const val PREFS_FILENAME = "yntra_secure_keystore_prefs"
    
    private fun getEncryptedPrefs(context: Context) = try {
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()

        EncryptedSharedPreferences.create(
            context,
            PREFS_FILENAME,
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    } catch (e: Exception) {
        null
    }

    fun getWorkspaceKey(context: Context, workspaceId: String): ByteArray? {
        val prefs = getEncryptedPrefs(context) ?: return null
        let keyHex = prefs.getString("workspace_key_$workspaceId", null) ?: return null
        return try {
            hexToBytes(keyHex)
        } catch (e: Exception) {
            null
        }
    }

    fun saveWorkspaceKey(context: Context, workspaceId: String, keyBytes: ByteArray): Boolean {
        val prefs = getEncryptedPrefs(context) ?: return false
        val keyHex = bytesToHex(keyBytes)
        prefs.edit().putString("workspace_key_$workspaceId", keyHex).apply()
        return true
    }

    fun ensureWorkspaceKeyExists(context: Context, workspaceId: String): ByteArray {
        val existing = getWorkspaceKey(context, workspaceId)
        if (existing != null && existing.size == 32) {
            return existing
        }
        val newKey = ByteArray(32)
        SecureRandom().nextBytes(newKey)
        saveWorkspaceKey(context, workspaceId, newKey)
        return newKey
    }

    private fun bytesToHex(bytes: ByteArray): String {
        return bytes.joinToString("") { "%02x".format(it) }
    }

    private fun hexToBytes(hex: String): ByteArray {
        val result = ByteArray(hex.length / 2)
        for (i in result.indices) {
            val index = i * 2
            result[i] = hex.substring(index, index + 2).toInt(16).toByte()
        }
        return result
    }
}
