# Swift FFI Bindings & SwiftUI Integration

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

    deinit {
        if let observer = self.observer {
            unregisterObserver(observer: observer)
        }
    }
}

---

## 3. Observer Lifecycle & Memory Cleanup

> [!WARNING]
> Failing to unregister UniFFI observers in `deinit` will cause native Swift-Rust ARC reference leaks and stale notification callbacks.

```swift
struct WorkspaceView: View {
    @StateObject private var viewModel = WorkspaceViewModel()

    var body: some View {
        List(viewModel.todos) { todo in
            Text(todo.title)
        }
        .onDisappear {
            // Explicit cleanup when view leaves hierarchy
            viewModel.cleanup()
        }
    }
}
```

```
