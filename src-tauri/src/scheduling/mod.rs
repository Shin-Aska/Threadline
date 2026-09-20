//! Background dispatch for durable schedule claims.
//!
//! The loop polls every 15 seconds, but eligibility and the 60-second grace window are enforced
//! by SQLite. It dispatches only records already transitioned to `DISPATCHING`, preserving the
//! same claim semantics used by the manual send-now command.

use crate::{
    database::publishing::now_epoch_ms, error::AppError, models::ScheduledPublication, publishing,
    AppState,
};

pub async fn dispatch_schedule(
    state: &AppState,
    schedule: ScheduledPublication,
) -> Result<ScheduledPublication, AppError> {
    let result = match publishing::publish_scheduled(state, &schedule).await {
        Ok(result) => result,
        Err(error) => {
            state
                .database
                .fail_schedule(&schedule.id, &error.to_string())?;
            return Err(error);
        }
    };
    state.database.complete_schedule(&schedule.id, &result)
}

pub async fn run(app: tauri::AppHandle) {
    use tauri::Manager;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
    interval.tick().await;
    loop {
        interval.tick().await;
        let state = app.state::<AppState>();
        loop {
            let schedule = match state.database.claim_due_schedule(now_epoch_ms()) {
                Ok(Some(schedule)) => schedule,
                Ok(None) | Err(_) => break,
            };
            let _ = dispatch_schedule(&state, schedule).await;
        }
    }
}
