package com.yntra.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import uniffi.yntra_core.*

class TodoViewModel : ViewModel() {
    private val _todos = MutableStateFlow<List<TodoItem>>(emptyList())
    val todos: StateFlow<List<TodoItem>> = _todos
    
    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage
    
    private val observer = KotlinDbObserver {
        loadTodos()
    }
    
    init {
        loadTodos()
        // Register observer to Rust FFI boundary
        registerObserver(observer)
    }
    
    private fun loadTodos() {
        viewModelScope.launch {
            try {
                // Pass dynamic user and workspace IDs to FFI function
                _todos.value = getTodos(SessionManager.activeUserId, SessionManager.activeWorkspaceId)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }
    
    fun addNewTodo(text: String) {
        viewModelScope.launch {
            try {
                addTodo(SessionManager.activeUserId, SessionManager.activeWorkspaceId, text)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }

    fun toggle(todo: TodoItem) {
        viewModelScope.launch {
            try {
                toggleTodo(SessionManager.activeUserId, todo.id)
            } catch (e: Exception) {
                _errorMessage.value = e.message
            }
        }
    }
    
    override fun onCleared() {
        super.onCleared()
        // Prevent FFI callback memory leaks
        clearObservers()
    }
}
