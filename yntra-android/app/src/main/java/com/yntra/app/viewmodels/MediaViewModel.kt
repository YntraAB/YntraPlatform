package com.yntra.app.viewmodels

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.yntra.app.SessionManager
import com.yntra.app.utils.MediaSandboxManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

data class MediaUploadState(
    val isUploading: Boolean = false,
    val progressPercent: Float = 0f,
    val lastHashPointer: String? = null,
    val errorMessage: String? = null
)

class MediaViewModel : ViewModel() {
    private val _state = MutableStateFlow(MediaUploadState())
    val state: StateFlow<MediaUploadState> = _state

    fun uploadCapturedMedia(context: Context, jobId: String, imageBytes: ByteArray) {
        viewModelScope.launch {
            _state.value = MediaUploadState(isUploading = true, progressPercent = 0.1f)
            try {
                val hash = MediaSandboxManager.uploadMediaInChunks(
                    context = context,
                    requesterUserId = SessionManager.activeUserId,
                    jobId = jobId,
                    bytes = imageBytes
                )
                _state.value = MediaUploadState(
                    isUploading = false,
                    progressPercent = 1.0f,
                    lastHashPointer = hash
                )
            } catch (e: Exception) {
                _state.value = MediaUploadState(
                    isUploading = false,
                    errorMessage = e.message ?: "Failed to upload media"
                )
            }
        }
    }
}
