package com.yntra.app.viewmodels

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.yntra_core.*
import com.yntra.app.KotlinDbObserver

class SettingsViewModel : ViewModel() {
    private val _workspace = MutableStateFlow<Workspace?>(null)
    val workspace: StateFlow<Workspace?> = _workspace

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private val observer = KotlinDbObserver {
        loadSettings()
    }

    init {
        loadSettings()
        registerObserver(observer)
    }

    fun loadSettings() {
        viewModelScope.launch {
            try {
                _workspace.value = getWorkspace()
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun updateProfile(fullName: String, phone: String, languagePreference: String) {
        viewModelScope.launch {
            try {
                updateUserProfile(
                    requesterUserId = com.yntra.app.SessionManager.activeUserId,
                    userId = com.yntra.app.SessionManager.activeUserId,
                    fullName = fullName,
                    phone = phone,
                    preferences = "{\"language\":\"$languagePreference\"}"
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
