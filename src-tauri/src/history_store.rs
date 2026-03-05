use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CommandRecord {
    pub id: String,
    pub command_text: String,
    pub intent_name: String,
    pub duration_ms: i64,
    pub success: bool,
    pub timestamp: i64,
    pub status: String,
    pub error_message: Option<String>,
}

pub mod status {
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
}

pub fn log_command(
    conn: &Connection,
    user_id: &str,
    command_text: String,
    intent_name: String,
    duration_ms: u128,
    success: bool,
    error_message: Option<String>,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();
    let cmd_status = if success { status::COMPLETED } else { status::FAILED };

    conn.execute(
        "INSERT INTO command_history
         (id, user_id, command_text, intent_name, duration_ms, success, timestamp, status, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            user_id,
            command_text,
            intent_name,
            duration_ms as i64,
            if success { 1 } else { 0 },
            now,
            cmd_status,
            error_message
        ],
    )
    .map_err(|e| format!("Failed to log command: {}", e))?;

    Ok(id)
}

pub fn get_command_history(
    conn: &Connection,
    user_id: &str,
    limit: i32,
    offset: i32,
) -> Result<Vec<CommandRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, command_text, intent_name, duration_ms, success, timestamp, status, error_message
             FROM command_history
             WHERE user_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2 OFFSET ?3",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let records = stmt
        .query_map(params![user_id, limit, offset], |row| {
            Ok(CommandRecord {
                id: row.get(0)?,
                command_text: row.get(1)?,
                intent_name: row.get(2)?,
                duration_ms: row.get(3)?,
                success: row.get::<_, i32>(4)? == 1,
                timestamp: row.get(5)?,
                status: row.get(6)?,
                error_message: row.get(7)?,
            })
        })
        .map_err(|e| format!("Failed to query command history: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map command records: {}", e))?;

    Ok(records)
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
}
