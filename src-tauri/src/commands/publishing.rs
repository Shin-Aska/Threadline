//! Tauri's durable publishing boundary.
//!
//! The commands accept frontend DTOs and delegate state transitions to the database. Revision
//! checks and dispatch claims live below this layer so manual and background callers obey the
//! same concurrency rules.

use crate::{
    database::publishing::now_epoch_ms,
    error::AppError,
    models::{
        CreateScheduleInput, DraftRecord, PublicationRecord, RescheduleInput, SaveDraftInput,
        ScheduleMutationInput, ScheduledPublication,
    },
    publishing, scheduling, AppState,
};
use tauri::State;

#[tauri::command]
pub fn save_draft(
    input: SaveDraftInput,
    state: State<'_, AppState>,
) -> Result<DraftRecord, AppError> {
    state.database.save_draft(input)
}

#[tauri::command]
pub fn list_drafts(state: State<'_, AppState>) -> Result<Vec<DraftRecord>, AppError> {
    state.database.list_drafts()
}

#[tauri::command]
pub fn get_draft(id: String, state: State<'_, AppState>) -> Result<DraftRecord, AppError> {
    state.database.get_draft(&id)
}

#[tauri::command]
pub fn delete_draft(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    state.database.delete_draft(&id)
}

#[tauri::command]
pub async fn publish_draft(
    id: String,
    state: State<'_, AppState>,
) -> Result<PublicationRecord, AppError> {
    publishing::publish_draft(&state, &id).await
}

#[tauri::command]
pub fn list_publications(state: State<'_, AppState>) -> Result<Vec<PublicationRecord>, AppError> {
    state.database.list_publications()
}

#[tauri::command]
pub fn delete_publication(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    state.database.delete_publication(&id)
}

#[tauri::command]
pub fn create_schedule(
    input: CreateScheduleInput,
    state: State<'_, AppState>,
) -> Result<ScheduledPublication, AppError> {
    if input.scheduled_for_epoch_ms <= now_epoch_ms() {
        return Err(AppError::Validation(
            "scheduled time must be in the future; use Send now for a missed time".into(),
        ));
    }
    state.database.create_schedule(input)
}

#[tauri::command]
pub fn list_schedules(state: State<'_, AppState>) -> Result<Vec<ScheduledPublication>, AppError> {
    state.database.list_schedules()
}

#[tauri::command]
pub fn reschedule_publication(
    input: RescheduleInput,
    state: State<'_, AppState>,
) -> Result<ScheduledPublication, AppError> {
    if input.scheduled_for_epoch_ms <= now_epoch_ms() {
        return Err(AppError::Validation(
            "scheduled time must be in the future; use Send now for a missed time".into(),
        ));
    }
    state.database.reschedule(input)
}

#[tauri::command]
pub fn cancel_schedule(
    input: ScheduleMutationInput,
    state: State<'_, AppState>,
) -> Result<ScheduledPublication, AppError> {
    state
        .database
        .cancel_schedule(&input.id, input.expected_revision)
}

#[tauri::command]
pub async fn send_schedule_now(
    input: ScheduleMutationInput,
    state: State<'_, AppState>,
) -> Result<ScheduledPublication, AppError> {
    let schedule = state
        .database
        .claim_schedule_now(&input.id, input.expected_revision)?
        .ok_or_else(|| AppError::Conflict("schedule changed or was already dispatched".into()))?;
    scheduling::dispatch_schedule(&state, schedule).await
}
