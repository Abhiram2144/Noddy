use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ChatMessageRecord {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

pub fn create_message(
    conn: &Connection,
    user_id: &str,
    role: &str,
    content: String,
) -> Result<String, String> {
    if role != "user" && role != "assistant" {
        return Err(format!("Invalid chat role: {}", role));
    }

    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();

    conn.execute(
        "INSERT INTO chat_messages (id, user_id, role, content, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, user_id, role, content, now],
    )
    .map_err(|e| format!("Failed to create chat message: {}", e))?;

    Ok(id)
}

pub fn get_messages(
    conn: &Connection,
    user_id: &str,
    limit: i32,
) -> Result<Vec<ChatMessageRecord>, String> {
    let safe_limit = limit.max(1).min(500);

    let mut stmt = conn
        .prepare(
            "SELECT id, role, content, created_at
             FROM chat_messages
             WHERE user_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare chat message query: {}", e))?;

    let mut rows = stmt
        .query_map(params![user_id, safe_limit], |row| {
            Ok(ChatMessageRecord {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to query chat messages: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map chat messages: {}", e))?;

    rows.reverse();
    Ok(rows)
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
