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
    
    func onTableChanged(table: String) {
        DispatchQueue.main.async {
            self.onChange()
        }
    }
}
