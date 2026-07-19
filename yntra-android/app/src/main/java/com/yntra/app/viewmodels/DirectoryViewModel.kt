package com.yntra.app.viewmodels

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.yntra_core.*
import com.yntra.app.KotlinDbObserver

class DirectoryViewModel : ViewModel() {
    private val _users = MutableStateFlow<List<WorkspaceUser>>(emptyList())
    val users: StateFlow<List<WorkspaceUser>> = _users

    private val _teams = MutableStateFlow<List<Team>>(emptyList())
    val teams: StateFlow<List<Team>> = _teams

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private val observer = KotlinDbObserver {
        refreshDirectory()
    }

    init {
        refreshDirectory()
        registerObserver(observer)
    }

    fun refreshDirectory() {
        viewModelScope.launch {
            try {
                _users.value = getUsers(com.yntra.app.SessionManager.activeUserId)
                _teams.value = getTeams(com.yntra.app.SessionManager.activeUserId)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun inviteUser(email: String, name: String, role: String) {
        viewModelScope.launch {
            try {
                inviteUserViaDirectory(
                    requesterUserId = com.yntra.app.SessionManager.activeUserId,
                    workspaceId = com.yntra.app.SessionManager.activeWorkspaceId,
                    email = email,
                    name = name,
                    role = role
                )
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun createTeam(name: String) {
        viewModelScope.launch {
            try {
                addTeamViaDirectory(
                    requesterUserId = com.yntra.app.SessionManager.activeUserId,
                    workspaceId = com.yntra.app.SessionManager.activeWorkspaceId,
                    name = name
                )
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun deleteWorkspaceUser(userId: String) {
        viewModelScope.launch {
            try {
                deleteUser(com.yntra.app.SessionManager.activeUserId, userId)
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
