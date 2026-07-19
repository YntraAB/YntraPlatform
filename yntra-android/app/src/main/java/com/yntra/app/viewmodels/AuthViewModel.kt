package com.yntra.app.viewmodels

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.yntra_core.*

class AuthViewModel : ViewModel() {
    private val _isLoggedIn = MutableStateFlow(false)
    val isLoggedIn: StateFlow<Boolean> = _isLoggedIn

    private val _authRegion = MutableStateFlow("sv") // "sv" | "da" | "no" | "fi" | "en"
    val authRegion: StateFlow<String> = _authRegion

    private val _bankIdFlowState = MutableStateFlow("idle") // "idle" | "qr_scan" | "verifying" | "success" | "error"
    val bankIdFlowState: StateFlow<String> = _bankIdFlowState

    private val _qrData = MutableStateFlow<String?>(null)
    val qrData: StateFlow<String?> = _qrData

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private var activeSessionId: String? = null
    private var isPolling = false

    fun setRegion(region: String) {
        _authRegion.value = region
    }

    fun initiateBankIdLogin(provider: String) {
        viewModelScope.launch {
            _errorMessage.value = null
            _bankIdFlowState.value = "connecting"
            try {
                // target_role: employee / assistant
                val session = initiateBankIdAuth("assistant", provider)
                activeSessionId = session.id
                _bankIdFlowState.value = session.status // e.g. "qr_scan"
                _qrData.value = session.qrData
                startPollingStatus(session.id)
            } catch (e: Exception) {
                _bankIdFlowState.value = "error"
                _errorMessage.value = e.message
            }
        }
    }

    private fun startPollingStatus(sessionId: String) {
        if (isPolling) return
        isPolling = true
        viewModelScope.launch {
            while (isPolling && _bankIdFlowState.value != "success" && _bankIdFlowState.value != "error") {
                delay(2000)
                try {
                    val sessionOpt = getBankidAuthSession(sessionId)
                    if (sessionOpt != null) {
                        _bankIdFlowState.value = sessionOpt.status
                        _qrData.value = sessionOpt.qrData
                         if (sessionOpt.status == "success" || sessionOpt.status == "authenticated") {
                            sessionOpt.authenticatedUserId?.let { uid ->
                                com.yntra.app.SessionManager.activeUserId = uid
                            }
                            _isLoggedIn.value = true
                            isPolling = false
                        } else if (sessionOpt.status == "failed" || sessionOpt.status == "error") {
                            _errorMessage.value = "Authentication failed"
                            _bankIdFlowState.value = "error"
                            isPolling = false
                        }
                    } else {
                        _bankIdFlowState.value = "error"
                        _errorMessage.value = "Session not found"
                        isPolling = false
                    }
                } catch (e: Exception) {
                    _bankIdFlowState.value = "error"
                    _errorMessage.value = e.message
                    isPolling = false
                }
            }
            isPolling = false
        }
    }

    fun passwordLogin(email: String, pin: String) {
        viewModelScope.launch {
            _errorMessage.value = null
            try {
                val user = verifyEmailPassword(email, pin)
                if (user != null) {
                    com.yntra.app.SessionManager.activeUserId = user.id
                    user.workspaceId?.let { ws ->
                        com.yntra.app.SessionManager.activeWorkspaceId = ws
                    }
                    _isLoggedIn.value = true
                } else {
                    _errorMessage.value = "Invalid email or credentials"
                }
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun logout() {
        _isLoggedIn.value = false
        _bankIdFlowState.value = "idle"
        _qrData.value = null
        activeSessionId = null
        isPolling = false
    }
}
