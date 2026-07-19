package com.yntra.app.viewmodels

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.yntra_core.*
import com.yntra.app.KotlinDbObserver

class MessagingViewModel : ViewModel() {
    private val _messages = MutableStateFlow<List<MessageItem>>(emptyList())
    val messages: StateFlow<List<MessageItem>> = _messages

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private val observer = KotlinDbObserver {
        loadMessages()
    }

    init {
        loadMessages()
        registerObserver(observer)
    }

    fun loadMessages() {
        viewModelScope.launch {
            try {
                _messages.value = getMessages(com.yntra.app.SessionManager.activeUserId, com.yntra.app.SessionManager.activeUserId)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun markAsRead(messageId: String) {
        viewModelScope.launch {
            try {
                markMessageRead(com.yntra.app.SessionManager.activeUserId, messageId)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun sendMessageToUser(receiverId: String, subject: String, body: String) {
        viewModelScope.launch {
            try {
                sendMessage(
                    requesterUserId = com.yntra.app.SessionManager.activeUserId,
                    workspaceId = com.yntra.app.SessionManager.activeWorkspaceId,
                    senderId = com.yntra.app.SessionManager.activeUserId,
                    receiverId = receiverId,
                    teamId = null,
                    subject = subject,
                    body = body
                )
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun sendMessageToTeam(teamId: String, subject: String, body: String) {
        viewModelScope.launch {
            try {
                sendMessage(
                    requesterUserId = com.yntra.app.SessionManager.activeUserId,
                    workspaceId = com.yntra.app.SessionManager.activeWorkspaceId,
                    senderId = com.yntra.app.SessionManager.activeUserId,
                    receiverId = null,
                    teamId = teamId,
                    subject = subject,
                    body = body
                )
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    override fun onCleared() {
        super.onCleared()
        clearObservers()
    }
}
