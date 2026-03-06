use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Canonical record returned from command_history table queries.
#[derive(Debug, Clone)]
pub struct CommandRecord {
    pub id: String,
    pub user_id: String,
    pub command_text: String,
    pub intent_name: String,
    pub success: bool,
    pub duration_ms: i64,
    /// Unix timestamp stored in the `timestamp` column.
    pub created_at: i64,
    pub status: String,
    pub error_message: Option<String>,
}

/// Aggregated usage statistics for a single user.
#[allow(dead_code)]
#[derive(Debug)]
pub struct CommandStats {
    pub total_commands: i64,
    pub successful_commands: i64,
    pub failed_commands: i64,
    pub avg_duration_ms: f64,
    pub most_used_intent: String,
}

// ============================================================================
// WRITE
// ============================================================================

/// Persist a single command execution record to the database.
/// Returns the generated UUID for the new row.
pub fn insert_command_record(
    conn: &Connection,
    user_id: &str,
    command_text: &str,
    intent_name: &str,
    success: bool,
    duration_ms: u128,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_unix();
    let status = if success { "completed" } else { "failed" };

    conn.execute(
        "INSERT INTO command_history
         (id, user_id, command_text, intent_name, success, duration_ms, timestamp, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            user_id,
            command_text,
            intent_name,
            if success { 1 } else { 0 },
            duration_ms as i64,
            now,
            status,
        ],
    )
    .map_err(|e| format!("Failed to insert command record: {}", e))?;

    Ok(id)
}

// ============================================================================
// READ
// ============================================================================

/// Fetch the most recent commands for a user in descending timestamp order.
pub fn get_recent_commands(
    conn: &Connection,
    user_id: &str,
    limit: i32,
) -> Result<Vec<CommandRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, user_id, command_text, intent_name, success,
                    duration_ms, timestamp, status, error_message
             FROM command_history
             WHERE user_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let records = stmt
        .query_map(params![user_id, limit], |row| {
            Ok(CommandRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                command_text: row.get(2)?,
                intent_name: row.get(3)?,
                success: row.get::<_, i32>(4)? == 1,
                duration_ms: row.get(5)?,
                created_at: row.get(6)?,
                status: row.get(7)?,
                error_message: row.get(8)?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    Ok(records)
}

/// Aggregate statistics over all commands for a user.
#[allow(dead_code)]
pub fn get_command_stats(
    conn: &Connection,
    user_id: &str,
) -> Result<CommandStats, String> {
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM command_history WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let successful: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM command_history WHERE user_id = ?1 AND success = 1",
            params![user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let avg_duration: f64 = conn
        .query_row(
            "SELECT COALESCE(AVG(CAST(duration_ms AS REAL)), 0.0)
             FROM command_history WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let most_used: String = conn
        .query_row(
            "SELECT intent_name
             FROM command_history
             WHERE user_id = ?1
             GROUP BY intent_name
             ORDER BY COUNT(*) DESC
             LIMIT 1",
            params![user_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "none".to_string());

    Ok(CommandStats {
        total_commands: total,
        successful_commands: successful,
        failed_commands: total - successful,
        avg_duration_ms: avg_duration,
        most_used_intent: most_used,
    })
}

// ============================================================================
// HELPERS
// ============================================================================

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
}
