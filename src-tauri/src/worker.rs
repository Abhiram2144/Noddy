use crate::reminder_store;
use crate::scheduler::{self, ReminderTaskPayload, ScheduledTask};
use crate::{Event, EventBus};
use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const WORKER_LOOP_INTERVAL: Duration = Duration::from_secs(10);
const PENDING_TASK_BATCH_SIZE: i32 = 50;

pub fn start_worker_loop(db_path: PathBuf, event_bus: EventBus, app_handle: AppHandle) {
    std::thread::spawn(move || {
        let conn = match Connection::open(&db_path) {
            Ok(conn) => conn,
            Err(error) => {
                eprintln!("⚠️  Worker failed to open database: {}", error);
                return;
            }
        };

        loop {
            if let Err(error) = run_pending_tasks_once(&conn, &event_bus, Some(&app_handle)) {
                eprintln!("⚠️  Worker execution failed: {}", error);
            }

            if let Err(error) = crate::suggestions::suggestion_worker::maybe_run_suggestion_cycle(
                &conn,
                &event_bus,
                Some(&app_handle),
            ) {
                eprintln!("⚠️  Suggestion worker failed: {}", error);
            }

            std::thread::sleep(WORKER_LOOP_INTERVAL);
        }
    });
}

pub fn run_pending_tasks_once(
    conn: &Connection,
    event_bus: &EventBus,
    app_handle: Option<&AppHandle>,
) -> Result<usize, String> {
    let tasks = scheduler::get_pending_tasks(conn, PENDING_TASK_BATCH_SIZE)?;
    let mut executed = 0_usize;

    for task in tasks {
        execute_task(conn, task, event_bus, app_handle)?;
        executed += 1;
    }

    Ok(executed)
}

pub fn execute_task(
    conn: &Connection,
    task: ScheduledTask,
    event_bus: &EventBus,
    app_handle: Option<&AppHandle>,
) -> Result<(), String> {
    let execution = match task.task_type.as_str() {
        scheduler::task_type::REMINDER => execute_reminder_task(conn, &task, event_bus, app_handle),
        _ => Err(format!("Unsupported task type: {}", task.task_type)),
    };

    match execution {
        Ok(()) => {
            scheduler::mark_task_completed(conn, &task.task_id)?;
            emit_events_for_completed_tasks(event_bus, app_handle, &task)?;
            Ok(())
        }
        Err(error) => {
            scheduler::mark_task_failed(conn, &task.task_id)?;
            Err(error)
        }
    }
}

pub fn emit_events_for_completed_tasks(
    event_bus: &EventBus,
    app_handle: Option<&AppHandle>,
    task: &ScheduledTask,
) -> Result<(), String> {
    event_bus.emit(&Event::TaskCompleted {
        task_id: task.task_id.clone(),
        task_type: task.task_type.clone(),
    });

    if let Some(app) = app_handle {
        let _ = app.emit(
            "background_task_completed",
            serde_json::json!({
                "task_id": task.task_id,
                "task_type": task.task_type,
                "status": scheduler::status::COMPLETED,
            }),
        );
    }

    Ok(())
}

fn execute_reminder_task(
    conn: &Connection,
    task: &ScheduledTask,
    event_bus: &EventBus,
    app_handle: Option<&AppHandle>,
) -> Result<(), String> {
    let payload: ReminderTaskPayload = serde_json::from_str(&task.payload)
        .map_err(|e| format!("Failed to deserialize reminder task payload: {}", e))?;

    reminder_store::update_reminder_status(
        conn,
        &payload.user_id,
        &payload.reminder_id,
        reminder_store::status::TRIGGERED,
    )?;

    if let Some(app) = app_handle {
        let _ = app.emit(
            "reminder_fired",
            serde_json::json!({
                "id": payload.reminder_id,
                "content": payload.content,
                "user_id": payload.user_id,
            }),
        );
    }

    event_bus.emit(&Event::ReminderTriggered(payload.content));
    Ok(())
}