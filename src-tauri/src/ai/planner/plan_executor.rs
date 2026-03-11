use super::action_plan::ActionPlan;
use crate::ai::intent_router;
use crate::ai::orchestrator::StructuredIntent;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
struct PlanExecutionState {
    recent_signatures: HashMap<String, (String, i64)>,
}

fn plan_state() -> &'static Mutex<PlanExecutionState> {
    static STATE: OnceLock<Mutex<PlanExecutionState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(PlanExecutionState::default()))
}

pub async fn execute_action_plan(
    user_message: &str,
    plan: ActionPlan,
    user_id: &str,
    app_handle: &tauri::AppHandle,
    registry: &crate::AppRegistry,
    memory_store: &crate::MemoryStore,
    plugin_registry: &crate::plugin_registry::PluginRegistry,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    if plan.actions.is_empty() {
        return Err("Planner returned an empty action plan".to_string());
    }

    let plan_started = std::time::Instant::now();
    let signature = plan_signature(&plan);
    let now_ts = current_timestamp();
    {
        let mut state = plan_state().lock().map_err(|e| format!("Plan state lock error: {}", e))?;
        if let Some((last_sig, ts)) = state.recent_signatures.get(user_id) {
            if *last_sig == signature && now_ts - *ts <= 20 {
                return Ok("I just executed this same plan recently, so I skipped repeating it to avoid loops.".to_string());
            }
        }
        state
            .recent_signatures
            .insert(user_id.to_string(), (signature, now_ts));
    }

    let mut responses = Vec::new();

    for (idx, step) in plan.actions.iter().enumerate() {
        let started = std::time::Instant::now();
        if step.requires_confirmation {
            let summary = if responses.is_empty() {
                ""
            } else {
                &format!(" Completed so far: {}", responses.join(" | "))
            };
            let message = format!(
                "Step {} requires confirmation before continuing: {}.{}",
                idx + 1,
                step.intent,
                summary
            );
            emit_plan_telemetry(event_bus, plan_started.elapsed().as_millis(), "confirmation_required", Some(&step.intent));
            return Ok(message);
        }

        let structured = StructuredIntent {
            intent: step.intent.clone(),
            parameters: step.parameters.clone(),
            confidence: 1.0,
        };

        match intent_router::route_intent(
            user_message,
            structured,
            user_id,
            app_handle,
            registry,
            memory_store,
            plugin_registry,
            event_bus,
            permissions,
        )
        .await
        {
            Ok(response) => {
                let duration_ms = started.elapsed().as_millis();
                if let Ok(conn) = memory_store.conn.lock() {
                    let _ = crate::command_history_service::record_command_execution(
                        &conn,
                        user_id,
                        &step.intent,
                        &step.parameters.to_string(),
                        true,
                        duration_ms,
                    );
                }
                responses.push(response)
            }
            Err(err) => {
                let duration_ms = started.elapsed().as_millis();
                if let Ok(conn) = memory_store.conn.lock() {
                    let _ = crate::command_history_service::record_command_execution(
                        &conn,
                        user_id,
                        &step.intent,
                        &step.parameters.to_string(),
                        false,
                        duration_ms,
                    );
                }
                let completed = if responses.is_empty() {
                    "none".to_string()
                } else {
                    responses.join(" | ")
                };
                let message = format!(
                    "Completed {} step(s), but failed on {}: {}.",
                    idx + 1,
                    step.intent,
                    err
                );
                let friendly = format!("{} Completed steps: {}", message, completed);
                emit_plan_telemetry(event_bus, plan_started.elapsed().as_millis(), "failed", Some(&step.intent));
                return Ok(friendly);
            }
        }
    }

    emit_plan_telemetry(event_bus, plan_started.elapsed().as_millis(), "completed", None);

    if responses.len() == 1 {
        Ok(responses.remove(0))
    } else {
        Ok(format!("Plan completed: {}", responses.join(" | ")))
    }
}

fn plan_signature(plan: &ActionPlan) -> String {
    plan.actions
        .iter()
        .map(|a| format!("{}:{}", a.intent, a.parameters))
        .collect::<Vec<_>>()
        .join("|")
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn emit_plan_telemetry(event_bus: &crate::EventBus, duration_ms: u128, status: &str, failed_intent: Option<&str>) {
    event_bus.emit(&crate::Event::IntentExecuted {
        intent_name: format!(
            "plan_execution:{}{}",
            status,
            failed_intent
                .map(|i| format!(":{}", i))
                .unwrap_or_default()
        ),
        duration_ms,
    });
}
