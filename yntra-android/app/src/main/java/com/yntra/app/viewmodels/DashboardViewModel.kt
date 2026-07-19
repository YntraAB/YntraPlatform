package com.yntra.app.viewmodels

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.yntra_core.*
import com.yntra.app.KotlinDbObserver

class DashboardViewModel : ViewModel() {
    private val _workspace = MutableStateFlow<Workspace?>(null)
    val workspace: StateFlow<Workspace?> = _workspace

    private val _events = MutableStateFlow<List<TeamEvent>>(emptyList())
    val events: StateFlow<List<TeamEvent>> = _events

    private val _todosCount = MutableStateFlow(0)
    val todosCount: StateFlow<Int> = _todosCount

    private val _completedTodosCount = MutableStateFlow(0)
    val completedTodosCount: StateFlow<Int> = _completedTodosCount

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private val observer = KotlinDbObserver {
        refreshDashboardData()
    }

    init {
        refreshDashboardData()
        registerObserver(observer)
    }

    fun refreshDashboardData() {
        viewModelScope.launch {
            try {
                // Fetch workspace
                _workspace.value = getWorkspace()

                // Fetch events
                _events.value = getEvents(com.yntra.app.SessionManager.activeUserId, null)
                
                // Fetch todos
                val todos = getTodos(com.yntra.app.SessionManager.activeUserId, com.yntra.app.SessionManager.activeWorkspaceId)
                _todosCount.value = todos.size
                _completedTodosCount.value = todos.count { it.completed }
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
