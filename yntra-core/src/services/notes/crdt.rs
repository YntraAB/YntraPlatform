use crate::YntraError;
use crate::database;

pub fn parse_loro_state(state: &str) -> (i64, &str) {
    if state.starts_with("loro:") {
        let clean = &state[5..];
        let parts: Vec<&str> = clean.splitn(2, ':').collect();
        if parts.len() == 2 {
            if let Ok(seq) = parts[0].parse::<i64>() {
                return (seq, parts[1]);
            }
        }
        (-1, clean)
    } else {
        (-1, state)
    }
}

pub fn apply_diff_to_loro(
    text: &loro::LoroText,
    old_str: &str,
    new_str: &str,
) -> Result<(), YntraError> {
    let old_chars: Vec<char> = old_str.chars().collect();
    let new_chars: Vec<char> = new_str.chars().collect();

    let mut common_prefix = 0;
    while common_prefix < old_chars.len()
        && common_prefix < new_chars.len()
        && old_chars[common_prefix] == new_chars[common_prefix]
    {
        common_prefix += 1;
    }

    let mut common_suffix = 0;
    while common_suffix < (old_chars.len() - common_prefix)
        && common_suffix < (new_chars.len() - common_prefix)
    {
        let old_idx = old_chars.len() - 1 - common_suffix;
        let new_idx = new_chars.len() - 1 - common_suffix;
        if old_chars[old_idx] == new_chars[new_idx] {
            common_suffix += 1;
        } else {
            break;
        }
    }

    let del_len = old_chars.len() - common_prefix - common_suffix;
    let ins_len = new_chars.len() - common_prefix - common_suffix;

    if del_len > 0 {
        text.delete(common_prefix, del_len)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    if ins_len > 0 {
        let ins_str: String = new_chars[common_prefix..(common_prefix + ins_len)]
            .iter()
            .collect();
        text.insert(common_prefix, &ins_str)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    Ok(())
}

pub async fn get_merged_loro_doc(
    conn: &database::DbConnection,
    note_id: &str,
) -> Result<loro::LoroDoc, YntraError> {
    let doc = loro::LoroDoc::new();

    // Fetch base note content snapshot
    let base_content: String = conn
        .query_row(
            "SELECT content FROM notes WHERE id = ?1",
            crate::params![note_id],
            |r| r.get(0),
        )
        .await
        .map_err(|_| YntraError::NotFoundError(format!("Note not found: {}", note_id)))?;

    let (last_merged_seq, hex_or_plain) = parse_loro_state(&base_content);
    if base_content.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain) {
            doc.import(&bytes)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else {
        doc.get_text("content")
            .insert(0, hex_or_plain)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }

    // Fetch and import only newer append-only updates
    let mut stmt = conn.prepare(
        "SELECT update_data FROM note_updates WHERE note_id = ?1 AND seq > ?2 ORDER BY seq ASC, created_at ASC"
    ).await?;
    let mut rows = stmt.query(crate::params![note_id, last_merged_seq]).await?;
    while let Some(row) = rows.next().await? {
        let update_data_hex: String = row.get(0)?;
        if let Some(bytes) = crate::infra::crypto::hex_decode(&update_data_hex) {
            doc.import(&bytes)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    }

    Ok(doc)
}

pub async fn compact_note_crdt_snapshot(
    conn: &database::DbConnection,
    note_id: &str,
    threshold_count: usize,
) -> Result<bool, YntraError> {
    let pending_updates_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM note_updates WHERE note_id = ?1",
            crate::params![note_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(0);

    if (pending_updates_count as usize) < threshold_count {
        return Ok(false);
    }

    let doc = get_merged_loro_doc(conn, note_id).await?;
    let snapshot_bytes = doc
        .export(loro::ExportMode::Snapshot)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;

    let max_seq: i64 = conn
        .query_row(
            "SELECT IFNULL(MAX(seq), -1) FROM note_updates WHERE note_id = ?1",
            crate::params![note_id],
            |r| r.get(0),
        )
        .await
        .unwrap_or(-1);

    let compressed_content = format!(
        "loro:{}:{}",
        max_seq,
        crate::infra::crypto::hex_encode(&snapshot_bytes)
    );

    conn.begin_transaction().await?;

    let res = async {
        conn.execute(
            "UPDATE notes SET content = ?1, updated_at = ?2 WHERE id = ?3",
            crate::params![
                compressed_content,
                crate::infra::time::get_current_time_ms(),
                note_id
            ],
        )
        .await?;

        conn.execute(
            "DELETE FROM note_updates WHERE note_id = ?1 AND seq <= ?2",
            crate::params![note_id, max_seq],
        )
        .await?;

        Ok::<(), YntraError>(())
    }
    .await;

    if let Err(e) = res {
        let _ = conn.rollback().await;
        Err(e)
    } else {
        conn.commit().await?;
        Ok(true)
    }
}

#[uniffi::export]
pub async fn compact_all_note_crdt_logs(
    workspace_id: String,
    threshold: u32,
) -> Result<u32, YntraError> {
    let conn = database::acquire_connection().await?;
    let mut stmt = conn
        .prepare("SELECT id FROM notes WHERE workspace_id = ?1")
        .await?;
    let mut rows = stmt.query(crate::params![workspace_id]).await?;

    let mut compacted_count = 0u32;
    let thresh = if threshold == 0 { 10 } else { threshold as usize };

    while let Some(row) = rows.next().await? {
        let note_id: String = row.get(0)?;
        if compact_note_crdt_snapshot(&conn, &note_id, thresh).await? {
            compacted_count += 1;
        }
    }

    Ok(compacted_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_loro_merge() {
        // Create initial doc state
        let doc1 = loro::LoroDoc::new();
        let text1 = doc1.get_text("content");
        text1.insert(0, "Hello").unwrap();
        let state1 = format!(
            "loro:{}",
            crate::infra::crypto::hex_encode(&doc1.export(loro::ExportMode::Snapshot).unwrap())
        );

        // Create concurrent update state from doc1's state
        let doc2 = loro::LoroDoc::new();
        let (_, hex1) = parse_loro_state(&state1);
        let bytes1 = crate::infra::crypto::hex_decode(hex1).unwrap();
        doc2.import(&bytes1).unwrap();
        let text2 = doc2.get_text("content");
        text2.insert(5, " World").unwrap();
        let state2 = format!(
            "loro:{}",
            crate::infra::crypto::hex_encode(&doc2.export(loro::ExportMode::Snapshot).unwrap())
        );

        // Create another concurrent update state from doc1's state
        let doc3 = loro::LoroDoc::new();
        doc3.import(&bytes1).unwrap();
        let text3 = doc3.get_text("content");
        text3.insert(0, "CRDT ").unwrap();
        let state3 = format!(
            "loro:{}",
            crate::infra::crypto::hex_encode(&doc3.export(loro::ExportMode::Snapshot).unwrap())
        );

        // Merge state2 and state3
        let state2_c = state2.clone();
        let state3_c = state3.clone();
        let merged1 = merge_loro_notes_test_helper(state2_c, state3_c).unwrap();
        let (_, hex_merged) = parse_loro_state(&merged1);
        let merged_bytes = crate::infra::crypto::hex_decode(hex_merged).unwrap();

        // Load merged state into a final document
        let doc_final = loro::LoroDoc::new();
        doc_final.import(&merged_bytes).unwrap();
        let final_text = doc_final.get_text("content").to_string();

        // The text should contain edits from both users resolved conflict-free
        assert!(final_text.contains("World"));
        assert!(final_text.contains("CRDT"));
    }

    fn merge_loro_notes_test_helper(state1: String, state2: String) -> Result<String, YntraError> {
        let doc1 = loro::LoroDoc::new();
        let (seq1, hex_or_plain1) = parse_loro_state(&state1);
        if state1.starts_with("loro:") {
            if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain1) {
                doc1.import(&bytes)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }
        } else {
            doc1.get_text("content")
                .insert(0, hex_or_plain1)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }

        let doc2 = loro::LoroDoc::new();
        let (seq2, hex_or_plain2) = parse_loro_state(&state2);
        if state2.starts_with("loro:") {
            if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain2) {
                doc2.import(&bytes)
                    .map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }
        } else {
            doc2.get_text("content")
                .insert(0, hex_or_plain2)
                .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }

        let bytes2 = doc2
            .export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        doc1.import(&bytes2)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;

        let merged_bytes = doc1
            .export(loro::ExportMode::Snapshot)
            .map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let max_seq = seq1.max(seq2);
        if max_seq >= 0 {
            Ok(format!(
                "loro:{}:{}",
                max_seq,
                crate::infra::crypto::hex_encode(&merged_bytes)
            ))
        } else {
            Ok(format!(
                "loro:{}",
                crate::infra::crypto::hex_encode(&merged_bytes)
            ))
        }
    }

    #[test]
    fn test_note_loro_merge_symmetric_plaintext() {
        let doc1 = loro::LoroDoc::new();
        let text1 = doc1.get_text("content");
        text1.insert(0, "LoroState").unwrap();
        let state1 = format!(
            "loro:{}",
            crate::infra::crypto::hex_encode(&doc1.export(loro::ExportMode::Snapshot).unwrap())
        );

        let state2 = "PlaintextState".to_string();

        // Merge state1 (Loro) and state2 (Plaintext)
        let merged_1_2 = merge_loro_notes_test_helper(state1.clone(), state2.clone()).unwrap();
        let (_, hex_1_2) = parse_loro_state(&merged_1_2);
        let merged_bytes_1_2 = crate::infra::crypto::hex_decode(hex_1_2).unwrap();
        let doc_final_1_2 = loro::LoroDoc::new();
        doc_final_1_2.import(&merged_bytes_1_2).unwrap();
        let text_final_1_2 = doc_final_1_2.get_text("content").to_string();

        assert!(text_final_1_2.contains("LoroState"));
        assert!(text_final_1_2.contains("PlaintextState"));

        // Merge state2 (Plaintext) and state1 (Loro) - Should yield both results symmetrically!
        let merged_2_1 = merge_loro_notes_test_helper(state2, state1).unwrap();
        let (_, hex_2_1) = parse_loro_state(&merged_2_1);
        let merged_bytes_2_1 = crate::infra::crypto::hex_decode(hex_2_1).unwrap();
        let doc_final_2_1 = loro::LoroDoc::new();
        doc_final_2_1.import(&merged_bytes_2_1).unwrap();
        let text_final_2_1 = doc_final_2_1.get_text("content").to_string();

        assert!(text_final_2_1.contains("LoroState"));
        assert!(text_final_2_1.contains("PlaintextState"));
    }

    #[test]
    fn test_apply_diff_to_loro() {
        let doc = loro::LoroDoc::new();
        let text = doc.get_text("content");

        // Initial insert
        text.insert(0, "Hello World").unwrap();

        // Test insertion in middle
        apply_diff_to_loro(&text, "Hello World", "Hello CRDT World").unwrap();
        assert_eq!(text.to_string(), "Hello CRDT World");

        // Test deletion in middle
        apply_diff_to_loro(&text, "Hello CRDT World", "Hello World").unwrap();
        assert_eq!(text.to_string(), "Hello World");

        // Test replacement
        apply_diff_to_loro(&text, "Hello World", "Goodbye World").unwrap();
        assert_eq!(text.to_string(), "Goodbye World");

        // Test empty string handling
        apply_diff_to_loro(&text, "Goodbye World", "").unwrap();
        assert_eq!(text.to_string(), "");

        // Test restore from empty
        apply_diff_to_loro(&text, "", "Back again").unwrap();
        assert_eq!(text.to_string(), "Back again");

        // Test with multi-byte Unicode characters (emojis)
        let doc_emoji = loro::LoroDoc::new();
        let text_emoji = doc_emoji.get_text("content");
        text_emoji.insert(0, "😅Hello World").unwrap();

        apply_diff_to_loro(&text_emoji, "😅Hello World", "😅Hello CRDT World").unwrap();
        assert_eq!(text_emoji.to_string(), "😅Hello CRDT World");

        apply_diff_to_loro(&text_emoji, "😅Hello CRDT World", "😅Goodbye World").unwrap();
        assert_eq!(text_emoji.to_string(), "😅Goodbye World");
    }

    #[tokio::test]
    async fn test_crdt_snapshot_compaction_and_pruning() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        let ws_id = "ws-compaction-test";
        let note_id = "note-compaction-1";

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings, brand_color, updated_at) VALUES (?1, 'WS Compaction', '{}', '{}', 'blue', 100)",
            crate::params![ws_id],
        ).await.unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO notes (id, workspace_id, team_id, subject, content, created_at, updated_at) VALUES (?1, ?2, 'team-1', 'Compaction Test', 'Base Compaction Content', '2026-08-05', 100)",
            crate::params![note_id, ws_id],
        ).await.unwrap();

        let _ = conn
            .execute(
                "DELETE FROM note_updates WHERE note_id = ?1",
                crate::params![note_id],
            )
            .await;

        for i in 1..=12 {
            let doc = loro::LoroDoc::new();
            let text = doc.get_text("content");
            text.insert(0, &format!("Edit {}", i)).unwrap();
            let hex = crate::infra::crypto::hex_encode(
                &doc.export(loro::ExportMode::Snapshot).unwrap(),
            );

            conn.execute(
                "INSERT INTO note_updates (id, note_id, client_id, seq, update_data, created_at) VALUES (?1, ?2, 'client-1', ?3, ?4, ?5)",
                crate::params![format!("upd-comp-{}", i), note_id, i as i64, hex, 100 + i as i64],
            ).await.unwrap();
        }

        let count_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM note_updates WHERE note_id = ?1",
                crate::params![note_id],
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert_eq!(count_before, 12);

        let compacted = compact_note_crdt_snapshot(&conn, note_id, 10)
            .await
            .unwrap();
        assert!(compacted);

        let count_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM note_updates WHERE note_id = ?1",
                crate::params![note_id],
                |r| r.get(0),
            )
            .await
            .unwrap();
        assert_eq!(count_after, 0);

        let merged_doc = get_merged_loro_doc(&conn, note_id).await.unwrap();
        let text_res = merged_doc.get_text("content").to_string();
        assert!(text_res.contains("Edit 12"));
    }
}
