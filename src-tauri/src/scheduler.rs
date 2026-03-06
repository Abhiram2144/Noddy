use crate::reminder_store::Reminder;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub task_id: String,
    pub task_type: String,
    pub payload: String,
    pub execute_at: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderTaskPayload {
    pub reminder_id: String,
    pub user_id: String,
    pub content: String,
    pub trigger_at: i64,
    pub memory_id: Option<String>,
}

pub mod status {
    pub const PENDING: &str = "pending";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const CANCELLED: &str = "cancelled";
}

pub mod task_type {
    pub const REMINDER: &str = "reminder";
}

pub fn register_task(conn: &Connection, task: ScheduledTask) -> Result<String, String> {
    let now = current_timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO background_tasks (task_id, task_type, payload, execute_at, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, COALESCE((SELECT created_at FROM background_tasks WHERE task_id = ?1), ?6), ?6)",
        params![
            task.task_id,
            task.task_type,
            task.payload,
            task.execute_at,
            task.status,
            now
        ],
    )
    .map_err(|e| format!("Failed to register task: {}", e))?;

    Ok(task.task_id)
}

pub fn schedule_reminder(conn: &Connection, reminder: &Reminder) -> Result<String, String> {
    let payload = serde_json::to_string(&ReminderTaskPayload {
        reminder_id: reminder.id.clone(),
        user_id: reminder.user_id.clone(),
        content: reminder.content.clone(),
        trigger_at: reminder.trigger_at,
        memory_id: reminder.memory_id.clone(),
    })
    .map_err(|e| format!("Failed to serialize reminder task payload: {}", e))?;

    register_task(
        conn,
        ScheduledTask {
            task_id: reminder_task_id(&reminder.id),
            task_type: task_type::REMINDER.to_string(),
            payload,
            execute_at: reminder.trigger_at,
            status: status::PENDING.to_string(),
        },
    )
}

pub fn get_pending_tasks(conn: &Connection, limit: i32) -> Result<Vec<ScheduledTask>, String> {
    let now = current_timestamp();
    let mut stmt = conn
        .prepare(
            "SELECT task_id, task_type, payload, execute_at, status
             FROM background_tasks
             WHERE status = ?1 AND execute_at <= ?2
             ORDER BY execute_at ASC
             LIMIT ?3",
        )
        .map_err(|e| format!("Failed to prepare pending tasks query: {}", e))?;

    let rows = stmt
        .query_map(params![status::PENDING, now, limit], |row| {
            Ok(ScheduledTask {
                task_id: row.get(0)?,
                task_type: row.get(1)?,
                payload: row.get(2)?,
                execute_at: row.get(3)?,
                status: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query pending tasks: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect pending tasks: {}", e))
}

pub fn mark_task_completed(conn: &Connection, task_id: &str) -> Result<(), String> {
    update_task_status(conn, task_id, status::COMPLETED)
}

pub fn mark_task_failed(conn: &Connection, task_id: &str) -> Result<(), String> {
    update_task_status(conn, task_id, status::FAILED)
}

pub fn cancel_task(conn: &Connection, task_id: &str) -> Result<(), String> {
    update_task_status(conn, task_id, status::CANCELLED)
}

pub fn cancel_reminder_task(conn: &Connection, reminder_id: &str) -> Result<(), String> {
    cancel_task(conn, &reminder_task_id(reminder_id))
}

pub fn sync_reminder_tasks(conn: &Connection) -> Result<i64, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, user_id, content, trigger_at, status, source, memory_id
             FROM reminders
             WHERE status IN ('pending', 'snoozed')
             ORDER BY trigger_at ASC",
        )
        .map_err(|e| format!("Failed to prepare reminder sync query: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
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
        .map_err(|e| format!("Failed to query reminders for sync: {}", e))?;

    let reminders = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect reminders for sync: {}", e))?;

    let mut scheduled = 0_i64;
    for reminder in reminders {
        schedule_reminder(conn, &reminder)?;
        scheduled += 1;

        if reminder.status == crate::reminder_store::status::SNOOZED {
            conn.execute(
                "UPDATE reminders SET status = ?1 WHERE id = ?2",
                params![crate::reminder_store::status::PENDING, reminder.id],
            )
            .map_err(|e| format!("Failed to normalize snoozed reminder status: {}", e))?;
        }
    }

    Ok(scheduled)
}

fn update_task_status(conn: &Connection, task_id: &str, next_status: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE background_tasks
         SET status = ?1, updated_at = ?2
         WHERE task_id = ?3",
        params![next_status, current_timestamp(), task_id],
    )
    .map_err(|e| format!("Failed to update task status: {}", e))?;

    Ok(())
}

pub fn reminder_task_id(reminder_id: &str) -> String {
    format!("reminder:{}", reminder_id)
}

#[allow(dead_code)]
pub fn generate_task_id() -> String {
    Uuid::new_v4().to_string()
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}