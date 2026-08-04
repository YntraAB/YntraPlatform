---
title: "Swift FFI Bindings & SwiftUI Integration"
head:
  - tag: script
    attrs:
      src: /target-blank.js
sidebar:
  hidden: false
---

This guide describes consuming UniFFI FFI bindings in **iOS SwiftUI** applications via `SwiftDbObserver`.

---

## 1. Observer Protocol Implementation

```swift
import Foundation
import yntra_core

class SwiftDbObserver: DatabaseObserver {
    private let onChange: () -> Void

    init(onChange: @escaping () -> Void) {
        self.onChange = onChange
    }

    func onDatabaseChanged() {
        DispatchQueue.main.async {
            self.onChange()
        }
    }
}
```

---

## 2. ObservableObject ViewModel Pattern

```swift
import SwiftUI
import yntra_core

class WorkspaceViewModel: ObservableObject {
    @Published var todos: [TodoItem] = []
    private var observer: SwiftDbObserver?

    init() {
        self.observer = SwiftDbObserver { [weak self] in
            self?.loadData()
        }
        registerObserver(observer: self.observer!)
        loadData()
    }

    func loadData() {
        self.todos = (try? getTodos(userId: "user-1", workspaceId: "ws-1")) ?? []
    }
}
```