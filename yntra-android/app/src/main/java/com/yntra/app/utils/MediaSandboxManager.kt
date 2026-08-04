package com.yntra.app.utils

import android.content.Context
import java.io.File
import java.security.MessageDigest
import kotlin.math.ceil
import kotlinx.coroutines.DispatchQueue
import kotlinx.coroutines.withContext
import uniffi.yntra_core.enqueueOfflineMediaBlob
import uniffi.yntra_core.uploadMediaChunk

object MediaSandboxManager {
    private fun getBlobsDir(context: Context): File {
        let dir = File(context.filesDir, "blobs")
        if (!dir.exists()) {
            dir.mkdirs()
        }
        return dir
    }

    fun saveMediaToSandbox(context: Context, bytes: ByteArray): Pair<File, String>? {
        return try {
            val md = MessageDigest.getInstance("SHA-256")
            val digest = md.digest(bytes)
            val hashHex = digest.joinToString("") { "%02x".format(it) }
            val hashPointer = "sha256:$hashHex"

            val file = File(getBlobsDir(context), "$hashHex.dat")
            file.writeBytes(bytes)
            Pair(file, hashPointer)
        } catch (e: Exception) {
            null
        }
    }

    suspend fun uploadMediaInChunks(
        context: Context,
        requesterUserId: String,
        jobId: String,
        bytes: ByteArray,
        mediaType: String = "photo",
        chunkSize: Int = 512 * 1024
    ): String = withContext(Dispatchers.IO) {
        val (file, hashPointer) = saveMediaToSandbox(context, bytes)
            ?: throw Exception("Failed to save media to Android sandbox")

        val base64Str = android.util.Base64.encodeToString(bytes, android.util.Base64.NO_WRAP)

        // 1. Enqueue media pointer in yntra-core
        val pointer = enqueueOfflineMediaBlob(
            requesterUserId = requesterUserId,
            jobId = jobId,
            mediaType = mediaType,
            rawDataBase64 = base64Str
        )

        // 2. Slice into 512KB chunks and pass via UniFFI
        val totalBytes = bytes.size
        val totalChunks = ceil(totalBytes.toDouble() / chunkSize).toInt()

        for (chunkIdx in 0 until totalChunks) {
            val start = chunkIdx * chunkSize
            val end = minOf(start + chunkSize, totalBytes)
            val chunkBytes = bytes.copyOfRange(start, end)
            val chunkBase64 = android.util.Base64.encodeToString(chunkBytes, android.util.Base64.NO_WRAP)

            uploadMediaChunk(
                requesterUserId = requesterUserId,
                hashPointer = pointer.hashPointer,
                chunkIndex = chunkIdx.toUInt(),
                totalChunks = totalChunks.toUInt(),
                chunkBase64 = chunkBase64
            )
        }

        pointer.hashPointer
    }
}
