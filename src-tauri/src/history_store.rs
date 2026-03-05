use rusqlite::{Connection, params};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// Service module for command history and telemetry
/// Tracks executed commands, intents, performance metrics, and success rates

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

/// Command status constants
pub mod status {
    pub const PENDING: &str = "pending";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const PARTIAL: &str = "partial";
}

/// Log a command execution
pub fn log_command(
    conn: &Connection,
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
         (id, command_text, intent_name, duration_ms, success, timestamp, status, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
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

/// Get all command records
pub fn get_command_history(
    conn: &Connection,
    limit: i32,
    offset: i32,
) -> Result<Vec<CommandRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, command_text, intent_name, duration_ms, success, timestamp, status, error_message
             FROM command_history
             ORDER BY timestamp DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let records = stmt
        .query_map(params![limit, offset], |row| {
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

/// Get recent command history (last N records)
pub fn get_recent_commands(
    conn: &Connection,
    limit: i32,
) -> Result<Vec<CommandRecord>, String> {
    get_command_history(conn, limit, 0)
}

/// Get command history by intent name
pub fn get_commands_by_intent(
    conn: &Connection,
    intent_name: &str,
    limit: i32,
) -> Result<Vec<CommandRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, command_text, intent_name, duration_ms, success, timestamp, status, error_message
             FROM command_history
             WHERE intent_name = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let records = stmt
        .query_map(params![intent_name, limit], |row| {
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
        .map_err(|e| format!("Failed to query commands by intent: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map command records: {}", e))?;
    
    Ok(records)
}

/// Get failed commands
pub fn get_failed_commands(
    conn: &Connection,
    limit: i32,
) -> Result<Vec<CommandRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, command_text, intent_name, duration_ms, success, timestamp, status, error_message
             FROM command_history
             WHERE success = 0
             ORDER BY timestamp DESC
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let records = stmt
        .query_map(params![limit], |row| {
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
        .map_err(|e| format!("Failed to query failed commands: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map command records: {}", e))?;
    
    Ok(records)
}

/// Get command statistics
#[derive(Debug)]
pub struct CommandStats {
    pub total_commands: i64,
    pub successful_commands: i64,
    pub failed_commands: i64,
    pub average_duration_ms: f64,
    pub success_rate: f64,
}

pub fn get_command_stats(conn: &Connection) -> Result<CommandStats, String> {
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM command_history",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count total commands: {}", e))?;
    
    let successful: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM command_history WHERE success = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count successful commands: {}", e))?;
    
    let failed = total - successful;
    
    let avg_duration: f64 = conn
        .query_row(
            "SELECT AVG(duration_ms) FROM command_history",
            [],
            |row| {
                let val: Option<f64> = row.get(0).ok().flatten();
                Ok(val.unwrap_or(0.0))
            },
        )
        .map_err(|e| format!("Failed to get average duration: {}", e))?;
    
    let success_rate = if total > 0 {
        (successful as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    
    Ok(CommandStats {
        total_commands: total,
        successful_commands: successful,
        failed_commands: failed,
        average_duration_ms: avg_duration,
        success_rate,
    })
}

/// Get per-intent statistics
#[derive(Debug)]
pub struct IntentStats {
    pub intent_name: String,
    pub count: i64,
    pub success_count: i64,
    pub average_duration_ms: f64,
}

pub fn get_intent_stats(conn: &Connection) -> Result<Vec<IntentStats>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT intent_name, COUNT(*) as count,
                    SUM(CASE WHEN success=1 THEN 1 ELSE 0 END) as success_count,
                    AVG(duration_ms) as avg_duration
             FROM command_history
             GROUP BY intent_name
             ORDER BY count DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let stats = stmt
        .query_map([], |row| {
            Ok(IntentStats {
                intent_name: row.get(0)?,
                count: row.get(1)?,
                success_count: row.get(2)?,
                average_duration_ms: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to query intent stats: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map intent stats: {}", e))?;
    
    Ok(stats)
}

/// Get slowest commands
pub fn get_slowest_commands(
    conn: &Connection,
    limit: i32,
) -> Result<Vec<CommandRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, command_text, intent_name, duration_ms, success, timestamp, status, error_message
             FROM command_history
             ORDER BY duration_ms DESC
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let records = stmt
        .query_map(params![limit], |row| {
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
        .map_err(|e| format!("Failed to query slowest commands: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map command records: {}", e))?;
    
    Ok(records)
}

/// Delete old command records (older than specified days)
pub fn delete_old_commands(
    conn: &Connection,
    days_old: i64,
) -> Result<i64, String> {
    let cutoff_time = current_timestamp() - (days_old * 86400);
    
    let changes = conn
        .execute(
            "DELETE FROM command_history WHERE timestamp < ?1",
            params![cutoff_time],
        )
        .map_err(|e| format!("Failed to delete old commands: {}", e))?;
    
    Ok(changes as i64)
}

/// Get total command count
pub fn get_command_count(conn: &Connection) -> Result<i64, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM command_history",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count commands: {}", e))?;
    
    Ok(count)
}

// ============================================================================
// Helper Functions
// ============================================================================

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_status_constants() {
        assert_eq!(status::COMPLETED, "completed");
        assert_eq!(status::FAILED, "failed");
    }
    
    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }
}
