---
title: "Kotlin FFI Bindings & Jetpack Compose Integration"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

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
}
```