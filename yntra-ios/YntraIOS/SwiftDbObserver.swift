import Foundation
import yntra_core

class SwiftDbObserver: DatabaseObserver {
    private let onChange: () -> Void
    private var pendingWorkItem: DispatchWorkItem?
    
    init(onChange: @escaping () -> Void) {
        self.onChange = onChange
    }
    
    private func notifyChangeDebounced() {
        pendingWorkItem?.cancel()
        let workItem = DispatchWorkItem { [weak self] in
            self?.onChange()
        }
        pendingWorkItem = workItem
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05, execute: workItem)
    }
    
    func onDatabaseChanged() {
        notifyChangeDebounced()
    }
    
    func onTableChanged(table: String) {
        notifyChangeDebounced()
    }
}
