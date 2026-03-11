use rusqlite::Connection;

use crate::command_history_repository::{self, CommandRecord};

// ============================================================================
// WRITE
// ============================================================================

/// Record a completed or failed command execution to persistent storage.
///
/// This is the primary integration point called after every intent dispatch.
/// Delegates validation-free writes to the repository layer.
pub fn record_command_execution(
    conn: &Connection,
    user_id: &str,
    intent_name: &str,
    command_text: &str,
    success: bool,
    duration_ms: u128,
) -> Result<(), String> {
    command_history_repository::insert_command_record(
        conn,
        user_id,
        command_text,
        intent_name,
        success,
        duration_ms,
    )?;
    Ok(())
}

// ============================================================================
// READ
// ============================================================================

/// Retrieve recent command history for a user.
///
/// Returns records newest-first, up to `limit` entries.
pub fn fetch_recent_history(
    conn: &Connection,
    user_id: &str,
    limit: i32,
) -> Result<Vec<CommandRecord>, String> {
    command_history_repository::get_recent_commands(conn, user_id, limit)
}

pub fn get_recent_commands(
    conn: &Connection,
    user_id: &str,
    limit: i32,
) -> Result<Vec<String>, String> {
    let records = command_history_repository::get_recent_commands(conn, user_id, limit)?;
    Ok(records
        .into_iter()
        .map(|record| record.command_text)
        .collect())
}

/// Calculate aggregated metrics for a user's command history.
///
/// Returns a JSON value with total/success/failure counts, average duration,
/// most-used intent, and success rate percentage.
#[allow(dead_code)]
pub fn calculate_command_metrics(
    conn: &Connection,
    user_id: &str,
) -> Result<serde_json::Value, String> {
    let stats = command_history_repository::get_command_stats(conn, user_id)?;
    let success_rate = if stats.total_commands > 0 {
        (stats.successful_commands as f64 / stats.total_commands as f64 * 100.0).round()
    } else {
        0.0
    };
    Ok(serde_json::json!({
        "total_commands":       stats.total_commands,
        "successful_commands":  stats.successful_commands,
        "failed_commands":      stats.failed_commands,
        "avg_duration_ms":      stats.avg_duration_ms.round() as i64,
        "most_used_intent":     stats.most_used_intent,
        "success_rate":         success_rate,
    }))
}
