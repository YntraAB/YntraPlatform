import SwiftUI
import yntra_core

@MainActor
class TodoViewModel: ObservableObject {
    @Published var todos: [TodoItem] = []
    @Published var errorMessage: String? = nil
    private var observer: SwiftDbObserver?
    
    init() {
        // Initial Load
        loadTodos()
        
        // Setup observer callback
        self.observer = SwiftDbObserver { [weak self] in
            self?.loadTodos()
        }
        
        // Register observer to Rust FFI boundary
        registerObserver(observer: self.observer!)
    }
    
    func loadTodos() {
        Task {
            do {
                self.todos = try await getTodos(requesterUserId: "user-1", workspaceId: "workspace-1")
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
    
    func addNewTodo(text: String) {
        Task {
            do {
                _ = try await addTodo(requesterUserId: "user-1", workspaceId: "workspace-1", text: text)
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
    
    func toggle(todo: TodoItem) {
        Task {
            do {
                try await toggleTodo(requesterUserId: "user-1", id: todo.id)
            } catch {
                self.errorMessage = error.localizedDescription
            }
        }
    }
}
