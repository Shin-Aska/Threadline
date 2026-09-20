//! Publication orchestration over the durable ledger.
//!
//! Network dispatch begins only after a destination and each segment are claimed. Any provider
//! error after a claim is recorded as `UNCERTAIN`; it is not automatically retried because the
//! remote service may have created the post before returning the error.

use crate::{
    composer,
    error::AppError,
    media,
    models::{
        CanonicalPost, DestinationPublication, PreparedMedia, PreparedPost, PublicationOutcome,
        PublicationRecord, PublishedPost, ScheduledPublication,
    },
    providers::SocialProvider,
    AppState,
};

pub async fn publish_draft(
    state: &AppState,
    draft_id: &str,
) -> Result<PublicationRecord, AppError> {
    let draft = state.database.get_draft(draft_id)?;
    let ledger = state.database.begin_publication(draft_id)?;
    dispatch_post(state, draft.post, ledger).await
}

pub async fn publish_scheduled(
    state: &AppState,
    schedule: &ScheduledPublication,
) -> Result<PublicationRecord, AppError> {
    let ledger = state.database.begin_scheduled_publication(schedule)?;
    dispatch_post(state, schedule.post.clone(), ledger).await
}

async fn dispatch_post(
    state: &AppState,
    post: CanonicalPost,
    ledger: PublicationRecord,
) -> Result<PublicationRecord, AppError> {
    if ledger
        .destinations
        .iter()
        .all(|destination| destination.status != PublicationOutcome::Pending)
    {
        return Ok(ledger);
    }
    let accounts = state.database.accounts()?;
    let available_ids = accounts
        .iter()
        .map(|account| account.id.clone())
        .collect::<Vec<_>>();
    let mut available_post = post.clone();
    available_post
        .destination_account_ids
        .retain(|account_id| available_ids.contains(account_id));
    let preview = if available_post.destination_account_ids.is_empty() {
        None
    } else {
        Some(composer::preview(&available_post, &accounts))
    };
    let prepared_media = media::prepare(&post.media);
    let providers = state
        .providers
        .read()
        .map_err(|_| AppError::StateUnavailable)?
        .clone();
    for destination in ledger.destinations {
        if destination.status != PublicationOutcome::Pending
            || !state
                .database
                .claim_destination(&ledger.id, &destination.account_id)?
        {
            continue;
        }
        let provider = providers.get(&destination.account_id).cloned();
        let parts = preview
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .and_then(|preview| {
                preview
                    .destinations
                    .iter()
                    .find(|item| item.account_id == destination.account_id)
                    .map(|item| item.parts.clone())
            });
        match (provider, parts, prepared_media.as_ref()) {
            (Some(provider), Some(parts), Ok(media)) => {
                dispatch_destination(
                    state,
                    &ledger.id,
                    &destination.account_id,
                    parts,
                    media.clone(),
                    provider.as_ref(),
                )
                .await?;
            }
            (provider, _, media) => {
                let (status, error) = if provider.is_none() {
                    (
                        PublicationOutcome::Blocked,
                        "Account is removed or disconnected; reconnect it before publishing".into(),
                    )
                } else {
                    let message = match media {
                        Err(error) => error.to_string(),
                        Ok(_) => preview
                            .as_ref()
                            .and_then(|result| result.as_ref().err())
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "Destination preview is unavailable".into()),
                    };
                    (PublicationOutcome::Failed, message)
                };
                state.database.finish_destination(
                    &ledger.id,
                    &DestinationPublication {
                        account_id: destination.account_id,
                        status,
                        remote_post_ids: Vec::new(),
                        error: Some(error),
                        segments: Vec::new(),
                    },
                )?;
            }
        }
    }
    state.database.get_publication(&ledger.id)
}

async fn dispatch_destination(
    state: &AppState,
    batch_id: &str,
    account_id: &str,
    parts: Vec<String>,
    media: Vec<PreparedMedia>,
    provider: &dyn SocialProvider,
) -> Result<(), AppError> {
    let mut parent: Option<PublishedPost> = None;
    for (index, text) in parts.into_iter().enumerate() {
        if !state.database.begin_segment(batch_id, account_id, index)? {
            return Err(AppError::Conflict(
                "publication segment was already attempted".into(),
            ));
        }
        let post = PreparedPost {
            text,
            media: if index == 0 {
                media.clone()
            } else {
                Vec::new()
            },
        };
        let result = match parent.as_ref() {
            Some(parent) => provider.reply(parent, post).await,
            None => provider.publish(post).await,
        };
        match result {
            Ok(remote) => {
                state.database.finish_segment(
                    batch_id,
                    account_id,
                    index,
                    PublicationOutcome::Published,
                    Some(&remote.remote_id),
                    None,
                )?;
                parent = Some(remote);
            }
            Err(error) => {
                state.database.finish_segment(
                    batch_id,
                    account_id,
                    index,
                    PublicationOutcome::Uncertain,
                    None,
                    Some(&error.to_string()),
                )?;
                let current = state.database.get_publication(batch_id)?;
                let remote_ids = current
                    .destinations
                    .iter()
                    .find(|destination| destination.account_id == account_id)
                    .map(|destination| destination.remote_post_ids.clone())
                    .unwrap_or_default();
                return state.database.finish_destination(
                    batch_id,
                    &DestinationPublication {
                        account_id: account_id.into(),
                        status: PublicationOutcome::Uncertain,
                        remote_post_ids: remote_ids,
                        error: Some(error.to_string()),
                        segments: Vec::new(),
                    },
                );
            }
        }
    }
    let current = state.database.get_publication(batch_id)?;
    let remote_ids = current
        .destinations
        .iter()
        .find(|destination| destination.account_id == account_id)
        .map(|destination| destination.remote_post_ids.clone())
        .unwrap_or_default();
    state.database.finish_destination(
        batch_id,
        &DestinationPublication {
            account_id: account_id.into(),
            status: PublicationOutcome::Published,
            remote_post_ids: remote_ids,
            error: None,
            segments: Vec::new(),
        },
    )
}

#[cfg(test)]
mod tests;
