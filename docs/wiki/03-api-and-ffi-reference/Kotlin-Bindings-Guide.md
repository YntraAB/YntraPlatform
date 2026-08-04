# Kotlin FFI Bindings & Jetpack Compose Integration

This guide describes consuming UniFFI FFI bindings in **Android Jetpack Compose** applications via `KotlinDbObserver`.

---

## 1. Observer Implementation

```kotlin
package com.yntra.app

import android.os.Handler
import android.os.Looper
import uniffi.yntra_core.DatabaseObserver

class KotlinDbObserver(private val onUpdate: () -> Unit) : DatabaseObserver {
    private val mainHandler = Handler(Looper.getMainLooper())

    override fun onDatabaseChanged() {
        mainHandler.post {
            onUpdate()
        }
    }
}
```

---

## 2. ViewModel & StateFlow Pattern

```kotlin
class WorkspaceViewModel : ViewModel() {
    private val _todos = MutableStateFlow<List<TodoItem>>(emptyList())
    val todos: StateFlow<List<TodoItem>> = _todos

    private val observer = KotlinDbObserver {
        loadData()
    }

    init {
        registerObserver(observer)
        loadData()
    }

    private fun loadData() {
        _todos.value = getTodos(SessionManager.activeUserId, SessionManager.activeWorkspaceId)
    }

    override fun onCleared() {
        super.onCleared()
        unregisterObserver(observer)
    }
}

---

## 3. Observer Lifecycle & Memory Cleanup

> [!WARNING]
> Failing to unregister UniFFI observers in `onCleared()` will cause JVM-JNI reference leaks and memory retain cycles.

```kotlin
@Composable
fun WorkspaceScreen(viewModel: WorkspaceViewModel = viewModel()) {
    val todos by viewModel.todos.collectAsState()

    DisposableEffect(Unit) {
        onDispose {
            // Unregister observers when composable leaves composition
            viewModel.onCleared()
        }
    }

    TodoList(todos = todos)
}
```

```
