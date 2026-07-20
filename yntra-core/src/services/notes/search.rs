use crate::database;
use crate::infra::errors::YntraError;
use crate::DailyNote;

fn sanitize_fts_query(query: &str) -> String {
    let cleaned: String = query
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect();

    let words: Vec<String> = cleaned
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .filter(|w| w != "and" && w != "or" && w != "not")
        .map(|w| format!("\"{}\"*", w))
        .collect();

    if words.is_empty() {
        "".to_string()
    } else {
        words.join(" ")
    }
}

#[uniffi::export]
pub async fn search_notes(
    requester_user_id: String,
    team_id: String,
    query: String,
) -> Result<Vec<DailyNote>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    let ws_id = auth.workspace_id.clone();

    if auth.role != "platform_admin" && auth.role != "admin" {
        let is_member: i64 = conn.query_row(
            "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
            crate::params![&team_id, &requester_user_id],
            |r| r.get(0)
        ).await.unwrap_or(0);
        if is_member == 0 {
            return Err(YntraError::AuthError("Access denied: you are not a member of this team".to_string()));
        }
    }

    let clean_query = sanitize_fts_query(&query);
    if clean_query.is_empty() {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare(
        "SELECT n.id, n.workspace_id, n.team_id, n.author_id, n.subject, 
                snippet(notes_fts, 2, '***', '***', '...', 32), 
                n.edit_history, n.created_at, n.updated_at, n.sync_status
         FROM notes n
         JOIN notes_fts f ON n.id = f.id
         WHERE n.workspace_id = ?1 AND n.team_id = ?2 AND notes_fts MATCH ?3
         ORDER BY rank"
    ).await?;

    let mut rows = stmt.query(crate::params![ws_id, team_id, clean_query]).await?;
    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
        results.push(DailyNote {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            team_id: row.get(2)?,
            author_id: row.get(3)?,
            subject: row.get(4)?,
            content: row.get(5).unwrap_or_default(),
            edit_history: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            sync_status: row.get(9)?,
        });
    }

    Ok(results)
}
