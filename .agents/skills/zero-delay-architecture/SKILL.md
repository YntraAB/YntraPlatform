---
name: zero-delay-architecture
description: 'Use when doing ANY task involving Yntra Platform tripartite architecture (Rust core, FFI, Dioxus Web/Desktop, SwiftUI/Compose Mobile, libSQL/Turso + OPFS, zero-copy serialization).'
---

# Yntra Architecture Skill

This skill guides the implementation of local-first components, schemas, and UI bindings within the Yntra Platform.

---

## 1. Modifying the Schema & DB Queries

All database changes must occur in the Rust core (`yntra-core`). 

### Rule 1: Schema Updates
* Database operations use embedded libSQL. Always design tables with support for optimistic writes.
* Include a synchronization tracking column `sync_status` (e.g., `'pending'`, `'synced'`) and a `version` or `updated_at` column.
* Include `workspace_id` in all data tables to support partitioning.

### Schema Template
```sql
CREATE TABLE IF NOT EXISTS daily_notes (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    updated_at INTEGER NOT NULL, -- Unix timestamp in milliseconds
    sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
);
```

---

## 2. Zero-Copy Serialization

Ensure any struct being transferred across language boundaries implements `rkyv::Archive`.

### Rust Serialization Model
```rust
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct DailyNote {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub content: String,
    pub updated_at: i64,
}
```

---

## 3. Creating UniFFI Interfaces

Define the export interfaces for mobile clients in `yntra-core/src/lib.rs` (or your UDL file):

```rust
// Exposing an interface to Swift and Kotlin via UniFFI
#[uniffi::export]
pub fn get_daily_notes(workspace_id: String) -> Result<Vec<DailyNote>, YntraError> {
    let conn = get_local_db_conn()?;
    let mut stmt = conn.prepare("SELECT id, workspace_id, title, content, updated_at FROM daily_notes WHERE workspace_id = ?")?;
    let note_iter = stmt.query_map([workspace_id], |row| {
        Ok(DailyNote {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            title: row.get(2)?,
            content: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;
    
    let mut notes = Vec::new();
    for note in note_iter {
        notes.push(note?);
    }
    Ok(notes)
}
```

---

## 4. Dioxus 0.7+ Reactive Subscription Template

When writing web/desktop views in Dioxus, use signals to dynamically re-render components on local database changes.

```rust
use dioxus::prelude::*;

#[component]
pub fn DailyNotesList(workspace_id: String) -> Element {
    // A signal containing our list of notes
    let mut notes = use_signal(Vec::new);

    // Dynamic database change listener
    use_effect(move || {
        let id = workspace_id.clone();
        // Spawns a task to poll database reactively or bind a change-feed listener
        spawn(async move {
            if let Ok(updated_notes) = get_daily_notes(id) {
                notes.set(updated_notes);
            }
        });
    });

    rsx! {
        div { class: "notes-container",
            h2 { "Daily Notes" }
            ul {
                for note in notes.read().iter() {
                    li { key: "{note.id}", class: "note-item",
                        h3 { "{note.title}" }
                        p { "{note.content}" }
                    }
                }
            }
        }
    }
}
```

---

## 5. Verification Checklist

Before ending any task involving this architecture, ensure you verify:
1. **Compilation**: Run `cargo check -p yntra-core` to verify the Rust backend.
2. **WASM Compatibility**: Run `cargo check --target wasm32-unknown-unknown -p yntra-core` to verify that there are no non-WASM calls (like thread blocking or native file-path manipulations).
3. **FFI Binding Generation**: Run `cargo run -p yntra-uniffi-bindgen -- generate --library target/debug/yntra_core.dll --language swift --out-dir generated_bindings` to confirm the Swift/Kotlin glue compiles without error.
