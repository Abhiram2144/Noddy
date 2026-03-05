use rusqlite::{Connection, params, OptionalExtension};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// Service module for reminder CRUD operations
/// Handles creation, querying, and status updates for reminders

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: String,
    pub content: String,
    pub created_at: i64,
    pub trigger_at: i64,
    pub status: String,
    pub source: String,
    pub memory_id: Option<String>,
}

/// Reminder status constants
pub mod status {
    pub const PENDING: &str = "pending";
    pub const TRIGGERED: &str = "triggered";
    pub const DISMISSED: &str = "dismissed";
    pub const SNOOZED: &str = "snoozed";
    pub const COMPLETED: &str = "completed";
}

/// Create a new reminder
pub fn create_reminder(
    conn: &Connection,
    content: String,
    trigger_at: i64,
    memory_id: Option<String>,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();
    
    conn.execute(
        "INSERT INTO reminders (id, content, created_at, trigger_at, status, source, memory_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            content,
            now,
            trigger_at,
            status::PENDING,
            "user_request",
            memory_id
        ],
    )
    .map_err(|e| format!("Failed to create reminder: {}", e))?;
    
    Ok(id)
}

/// Get all pending reminders
pub fn get_pending_reminders(conn: &Connection) -> Result<Vec<Reminder>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, trigger_at, status, source, memory_id
             FROM reminders
             WHERE status = ?1
             ORDER BY trigger_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let reminders = stmt
        .query_map(params![status::PENDING], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                trigger_at: row.get(3)?,
                status: row.get(4)?,
                source: row.get(5)?,
                memory_id: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query reminders: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map reminders: {}", e))?;
    
    Ok(reminders)
}

/// Get reminders that should trigger now or soon
pub fn get_due_reminders(conn: &Connection, seconds_ahead: i64) -> Result<Vec<Reminder>, String> {
    let now = current_timestamp();
    let deadline = now + seconds_ahead;
    
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, trigger_at, status, source, memory_id
             FROM reminders
             WHERE status = ?1 AND trigger_at <= ?2
             ORDER BY trigger_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let reminders = stmt
        .query_map(params![status::PENDING, deadline], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                trigger_at: row.get(3)?,
                status: row.get(4)?,
                source: row.get(5)?,
                memory_id: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query due reminders: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map due reminders: {}", e))?;
    
    Ok(reminders)
}

/// Get a specific reminder by ID
pub fn get_reminder_by_id(
    conn: &Connection,
    reminder_id: &str,
) -> Result<Option<Reminder>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, trigger_at, status, source, memory_id
             FROM reminders
             WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let reminder = stmt
        .query_row(params![reminder_id], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                trigger_at: row.get(3)?,
                status: row.get(4)?,
                source: row.get(5)?,
                memory_id: row.get(6)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to query reminder: {}", e))?;
    
    Ok(reminder)
}

/// Update reminder status
pub fn update_reminder_status(
    conn: &Connection,
    reminder_id: &str,
    new_status: &str,
) -> Result<(), String> {
    // Validate status
    match new_status {
        status::PENDING | status::TRIGGERED | status::DISMISSED | status::SNOOZED | status::COMPLETED => {},
        _ => return Err(format!("Invalid status: {}", new_status)),
    }
    
    conn.execute(
        "UPDATE reminders SET status = ?1 WHERE id = ?2",
        params![new_status, reminder_id],
    )
    .map_err(|e| format!("Failed to update reminder status: {}", e))?;
    
    Ok(())
}

/// Mark reminder as completed
pub fn mark_reminder_completed(
    conn: &Connection,
    reminder_id: &str,
) -> Result<(), String> {
    update_reminder_status(conn, reminder_id, status::COMPLETED)
}

/// Mark reminder as dismissed
pub fn mark_reminder_dismissed(
    conn: &Connection,
    reminder_id: &str,
) -> Result<(), String> {
    update_reminder_status(conn, reminder_id, status::DISMISSED)
}

/// Snooze a reminder (reschedule for later)
pub fn snooze_reminder(
    conn: &Connection,
    reminder_id: &str,
    snooze_minutes: i64,
) -> Result<(), String> {
    let now = current_timestamp();
    let new_trigger_time = now + (snooze_minutes * 60);
    
    conn.execute(
        "UPDATE reminders SET trigger_at = ?1, status = ?2 WHERE id = ?3",
        params![new_trigger_time, status::SNOOZED, reminder_id],
    )
    .map_err(|e| format!("Failed to snooze reminder: {}", e))?;
    
    Ok(())
}

/// Delete a reminder
pub fn delete_reminder(
    conn: &Connection,
    reminder_id: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM reminders WHERE id = ?1",
        params![reminder_id],
    )
    .map_err(|e| format!("Failed to delete reminder: {}", e))?;
    
    Ok(())
}

/// Get all reminders for a specific memory
pub fn get_memory_reminders(
    conn: &Connection,
    memory_id: &str,
) -> Result<Vec<Reminder>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, trigger_at, status, source, memory_id
             FROM reminders
             WHERE memory_id = ?1
             ORDER BY trigger_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let reminders = stmt
        .query_map(params![memory_id], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                trigger_at: row.get(3)?,
                status: row.get(4)?,
                source: row.get(5)?,
                memory_id: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query memory reminders: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map memory reminders: {}", e))?;
    
    Ok(reminders)
}

/// Get count of pending reminders
pub fn get_pending_reminder_count(conn: &Connection) -> Result<i64, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM reminders WHERE status = ?1",
            params![status::PENDING],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count pending reminders: {}", e))?;
    
    Ok(count)
}

/// Get total reminder count
pub fn get_total_reminder_count(conn: &Connection) -> Result<i64, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM reminders",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count reminders: {}", e))?;
    
    Ok(count)
}

/// Get reminders by status
pub fn get_reminders_by_status(
    conn: &Connection,
    status_filter: &str,
    limit: i32,
) -> Result<Vec<Reminder>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, trigger_at, status, source, memory_id
             FROM reminders
             WHERE status = ?1
             ORDER BY trigger_at ASC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let reminders = stmt
        .query_map(params![status_filter, limit], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                trigger_at: row.get(3)?,
                status: row.get(4)?,
                source: row.get(5)?,
                memory_id: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query reminders: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map reminders: {}", e))?;
    
    Ok(reminders)
}

/// Convert timestamp to human readable format (for testing)
pub fn format_timestamp(timestamp: i64) -> String {
    use std::time::{UNIX_EPOCH, Duration};
    let duration = Duration::from_secs(timestamp as u64);
    let system_time = UNIX_EPOCH + duration;
    format!("{:?}", system_time)
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
        assert_eq!(status::PENDING, "pending");
        assert_eq!(status::COMPLETED, "completed");
    }
    
    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }
}
