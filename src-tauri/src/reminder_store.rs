use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: String,
    pub user_id: String,
    pub content: String,
    pub trigger_at: i64,
    pub status: String,
    pub source: String,
    pub memory_id: Option<String>,
}

pub mod status {
    pub const PENDING: &str = "pending";
    pub const TRIGGERED: &str = "triggered";
    pub const SNOOZED: &str = "snoozed";
}

pub fn create_reminder(
    conn: &Connection,
    user_id: &str,
    content: String,
    trigger_at: i64,
    memory_id: Option<String>,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();

    conn.execute(
        "INSERT INTO reminders (id, user_id, content, created_at, trigger_at, status, source, memory_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            user_id,
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

pub fn get_pending_reminders(conn: &Connection, user_id: &str) -> Result<Vec<Reminder>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, user_id, content, trigger_at, status, source, memory_id
             FROM reminders
             WHERE user_id = ?1 AND status = ?2
             ORDER BY trigger_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let reminders = stmt
        .query_map(params![user_id, status::PENDING], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                user_id: row.get(1)?,
                content: row.get(2)?,
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

pub fn get_due_reminders_global(conn: &Connection, seconds_ahead: i64) -> Result<Vec<Reminder>, String> {
    let now = current_timestamp();
    let deadline = now + seconds_ahead;

    let mut stmt = conn
        .prepare(
            "SELECT id, user_id, content, trigger_at, status, source, memory_id
             FROM reminders
             WHERE status = ?1 AND trigger_at <= ?2
             ORDER BY trigger_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let reminders = stmt
        .query_map(params![status::PENDING, deadline], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                user_id: row.get(1)?,
                content: row.get(2)?,
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

pub fn update_reminder_status(
    conn: &Connection,
    user_id: &str,
    reminder_id: &str,
    new_status: &str,
) -> Result<(), String> {
    match new_status {
        status::PENDING | status::TRIGGERED | status::SNOOZED => {}
        _ => return Err(format!("Invalid status: {}", new_status)),
    }

    conn.execute(
        "UPDATE reminders SET status = ?1 WHERE id = ?2 AND user_id = ?3",
        params![new_status, reminder_id, user_id],
    )
    .map_err(|e| format!("Failed to update reminder status: {}", e))?;

    Ok(())
}

pub fn snooze_reminder(
    conn: &Connection,
    user_id: &str,
    reminder_id: &str,
    snooze_minutes: i64,
) -> Result<(), String> {
    let now = current_timestamp();
    let new_trigger_time = now + (snooze_minutes * 60);

    conn.execute(
        "UPDATE reminders SET trigger_at = ?1, status = ?2 WHERE id = ?3 AND user_id = ?4",
        params![new_trigger_time, status::SNOOZED, reminder_id, user_id],
    )
    .map_err(|e| format!("Failed to snooze reminder: {}", e))?;

    Ok(())
}

pub fn delete_reminder(conn: &Connection, user_id: &str, reminder_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM reminders WHERE id = ?1 AND user_id = ?2",
        params![reminder_id, user_id],
    )
    .map_err(|e| format!("Failed to delete reminder: {}", e))?;

    Ok(())
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
}
