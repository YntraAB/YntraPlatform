use crate::database;
use crate::YntraError;

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

pub fn apply_diff_to_loro(text: &loro::LoroText, old_str: &str, new_str: &str) -> Result<(), YntraError> {
    let old_chars: Vec<char> = old_str.chars().collect();
    let new_chars: Vec<char> = new_str.chars().collect();
    
    let diffs = diff::slice(&old_chars, &new_chars);
    
    let mut pos_utf16 = 0;
    let mut i = 0;
    while i < diffs.len() {
        match diffs[i] {
            diff::Result::Both(c, _) => {
                pos_utf16 += c.len_utf16();
                i += 1;
            }
            diff::Result::Left(_) => {
                let mut del_utf16_len = 0;
                while i < diffs.len() {
                    if let diff::Result::Left(c) = diffs[i] {
                        del_utf16_len += c.len_utf16();
                        i += 1;
                    } else {
                        break;
                    }
                }
                text.delete(pos_utf16, del_utf16_len).map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }
            diff::Result::Right(_) => {
                let mut ins_str = String::new();
                while i < diffs.len() {
                    if let diff::Result::Right(c) = diffs[i] {
                        ins_str.push(*c);
                        i += 1;
                    } else {
                        break;
                    }
                }
                let ins_utf16_len = ins_str.encode_utf16().count();
                text.insert(pos_utf16, &ins_str).map_err(|e| YntraError::SerializationError(e.to_string()))?;
                pos_utf16 += ins_utf16_len;
            }
        }
    }
    Ok(())
}

pub async fn get_merged_loro_doc(conn: &database::DbConnection, note_id: &str) -> Result<loro::LoroDoc, YntraError> {
    let doc = loro::LoroDoc::new();
    
    // Fetch base note content snapshot
    let base_content: String = conn.query_row(
        "SELECT content FROM notes WHERE id = ?1",
        crate::params![note_id],
        |r| r.get(0)
    ).await.map_err(|_| YntraError::NotFoundError(format!("Note not found: {}", note_id)))?;
    
    let (last_merged_seq, hex_or_plain) = parse_loro_state(&base_content);
    if base_content.starts_with("loro:") {
        if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain) {
            doc.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    } else {
        doc.get_text("content").insert(0, hex_or_plain).map_err(|e| YntraError::SerializationError(e.to_string()))?;
    }
    
    // Fetch and import only newer append-only updates
    let mut stmt = conn.prepare(
        "SELECT update_data FROM note_updates WHERE note_id = ?1 AND seq > ?2 ORDER BY seq ASC, created_at ASC"
    ).await?;
    let mut rows = stmt.query(crate::params![note_id, last_merged_seq]).await?;
    while let Some(row) = rows.next().await? {
        let update_data_hex: String = row.get(0)?;
        if let Some(bytes) = crate::infra::crypto::hex_decode(&update_data_hex) {
            doc.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }
    }
    
    Ok(doc)
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
        let state1 = format!("loro:{}", crate::infra::crypto::hex_encode(&doc1.export(loro::ExportMode::Snapshot).unwrap()));

        // Create concurrent update state from doc1's state
        let doc2 = loro::LoroDoc::new();
        let (_, hex1) = parse_loro_state(&state1);
        let bytes1 = crate::infra::crypto::hex_decode(hex1).unwrap();
        doc2.import(&bytes1).unwrap();
        let text2 = doc2.get_text("content");
        text2.insert(5, " World").unwrap();
        let state2 = format!("loro:{}", crate::infra::crypto::hex_encode(&doc2.export(loro::ExportMode::Snapshot).unwrap()));

        // Create another concurrent update state from doc1's state
        let doc3 = loro::LoroDoc::new();
        doc3.import(&bytes1).unwrap();
        let text3 = doc3.get_text("content");
        text3.insert(0, "CRDT ").unwrap();
        let state3 = format!("loro:{}", crate::infra::crypto::hex_encode(&doc3.export(loro::ExportMode::Snapshot).unwrap()));

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
                doc1.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }
        } else {
            doc1.get_text("content").insert(0, hex_or_plain1).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }

        let doc2 = loro::LoroDoc::new();
        let (seq2, hex_or_plain2) = parse_loro_state(&state2);
        if state2.starts_with("loro:") {
            if let Some(bytes) = crate::infra::crypto::hex_decode(hex_or_plain2) {
                doc2.import(&bytes).map_err(|e| YntraError::SerializationError(e.to_string()))?;
            }
        } else {
            doc2.get_text("content").insert(0, hex_or_plain2).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        }

        let bytes2 = doc2.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        doc1.import(&bytes2).map_err(|e| YntraError::SerializationError(e.to_string()))?;

        let merged_bytes = doc1.export(loro::ExportMode::Snapshot).map_err(|e| YntraError::SerializationError(e.to_string()))?;
        let max_seq = seq1.max(seq2);
        if max_seq >= 0 {
            Ok(format!("loro:{}:{}", max_seq, crate::infra::crypto::hex_encode(&merged_bytes)))
        } else {
            Ok(format!("loro:{}", crate::infra::crypto::hex_encode(&merged_bytes)))
        }
    }

    #[test]
    fn test_note_loro_merge_symmetric_plaintext() {
        let doc1 = loro::LoroDoc::new();
        let text1 = doc1.get_text("content");
        text1.insert(0, "LoroState").unwrap();
        let state1 = format!("loro:{}", crate::infra::crypto::hex_encode(&doc1.export(loro::ExportMode::Snapshot).unwrap()));

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
    }
}
