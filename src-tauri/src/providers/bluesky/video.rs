mod auth;
use crate::{error::AppError, models::PreparedMedia};
use auth::ServiceAuthSource;
use serde::Deserialize;
use std::time::Duration;

const VIDEO_SERVICE_DID: &str = "did:web:video.bsky.app";
#[cfg(not(test))]
const VIDEO_SERVICE_URL: &str = "https://video.bsky.app";
const VIDEO_MIME: &str = "video/mp4";
const PROCESSING_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadLimits {
    can_upload: bool,
    remaining_daily_videos: Option<u64>,
    remaining_daily_bytes: Option<u64>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobStatus {
    job_id: String,
    state: String,
    progress: Option<u8>,
    blob: Option<serde_json::Value>,
    error: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobResponse {
    job_status: JobStatus,
}

pub(super) fn service_url(pds_url: &str) -> &str {
    #[cfg(test)]
    {
        pds_url
    }
    #[cfg(not(test))]
    {
        let _ = pds_url;
        VIDEO_SERVICE_URL
    }
}

pub(super) async fn upload_video(
    client: &reqwest::Client,
    pds_url: &str,
    video_service_url: &str,
    access_jwt: &str,
    did: &str,
    video: &PreparedMedia,
) -> Result<serde_json::Value, AppError> {
    upload_video_with_auth(
        client,
        video_service_url,
        did,
        video,
        ServiceAuthSource::Bearer {
            pds_url,
            access_jwt,
        },
    )
    .await
}

pub(super) async fn upload_video_oauth(
    oauth: &crate::oauth::bluesky::BlueskyOAuthRuntime,
    client: &reqwest::Client,
    video_service_url: &str,
    did: &str,
    video: &PreparedMedia,
) -> Result<serde_json::Value, AppError> {
    upload_video_with_auth(
        client,
        video_service_url,
        did,
        video,
        ServiceAuthSource::OAuth(oauth),
    )
    .await
}

async fn upload_video_with_auth(
    client: &reqwest::Client,
    video_service_url: &str,
    did: &str,
    video: &PreparedMedia,
    auth: ServiceAuthSource<'_>,
) -> Result<serde_json::Value, AppError> {
    if video.mime_type != VIDEO_MIME {
        return Err(AppError::Validation("Bluesky video must be MP4".into()));
    }
    let limits_token = auth.token(client, "app.bsky.video.getUploadLimits").await?;
    let limits: UploadLimits = response_json(
        client
            .get(format!(
                "{}/xrpc/app.bsky.video.getUploadLimits",
                video_service_url.trim_end_matches('/')
            ))
            .bearer_auth(limits_token)
            .timeout(Duration::from_secs(30)),
        "Bluesky video upload limits",
    )
    .await?;
    validate_limits(&limits, video.data.len())?;

    let upload_token = auth.token(client, "app.bsky.video.uploadVideo").await?;
    let response: JobResponse = response_json(
        client
            .post(format!(
                "{}/xrpc/app.bsky.video.uploadVideo",
                video_service_url.trim_end_matches('/')
            ))
            .query(&[
                ("did", did),
                ("name", &format!("{}.mp4", uuid::Uuid::new_v4())),
            ])
            .bearer_auth(upload_token)
            .header(reqwest::header::CONTENT_TYPE, VIDEO_MIME)
            .timeout(Duration::from_secs(120))
            .body(video.data.to_vec()),
        "Bluesky video upload",
    )
    .await?;
    wait_for_blob(client, video_service_url, response.job_status).await
}

fn validate_limits(limits: &UploadLimits, size_bytes: usize) -> Result<(), AppError> {
    if !limits.can_upload || limits.remaining_daily_videos == Some(0) {
        return Err(AppError::Provider(
            limits
                .message
                .as_deref()
                .or(limits.error.as_deref())
                .unwrap_or("Bluesky video uploads are unavailable for this account")
                .to_owned(),
        ));
    }
    if limits
        .remaining_daily_bytes
        .is_some_and(|remaining| size_bytes as u64 > remaining)
    {
        return Err(AppError::Provider(
            "Bluesky daily video byte allowance is too low for this file".into(),
        ));
    }
    Ok(())
}

async fn wait_for_blob(
    client: &reqwest::Client,
    video_service_url: &str,
    initial: JobStatus,
) -> Result<serde_json::Value, AppError> {
    tokio::time::timeout(PROCESSING_TIMEOUT, async {
        let mut status = initial;
        loop {
            if status.state == "JOB_STATE_FAILED" {
                return Err(processing_error(&status));
            }
            if status.state == "JOB_STATE_COMPLETED" {
                return status.blob.ok_or_else(|| {
                    AppError::Provider(
                        "Bluesky video processing completed without a usable blob".into(),
                    )
                });
            }
            tokio::time::sleep(POLL_INTERVAL).await;
            let response: JobResponse = response_json(
                client
                    .get(format!(
                        "{}/xrpc/app.bsky.video.getJobStatus",
                        video_service_url.trim_end_matches('/')
                    ))
                    .query(&[("jobId", status.job_id.as_str())])
                    .timeout(Duration::from_secs(30)),
                "Bluesky video processing status",
            )
            .await?;
            status = response.job_status;
        }
    })
    .await
    .map_err(|_| {
        AppError::Provider(
            "Bluesky video processing timed out; the post was not sent and the draft was kept"
                .into(),
        )
    })?
}

fn processing_error(status: &JobStatus) -> AppError {
    let detail = status
        .message
        .as_deref()
        .or(status.error.as_deref())
        .unwrap_or("the video service rejected the file");
    let progress = status
        .progress
        .map(|value| format!(" at {value}%"))
        .unwrap_or_default();
    AppError::Provider(format!(
        "Bluesky video processing failed{progress}: {detail}; the post was not sent"
    ))
}

async fn response_json<T: serde::de::DeserializeOwned>(
    request: reqwest::RequestBuilder,
    operation: &str,
) -> Result<T, AppError> {
    let response = request
        .send()
        .await
        .map_err(|error| AppError::Provider(format!("{operation} failed: {error}")))?;
    let status = response.status();
    let body = response.text().await.map_err(|error| {
        AppError::Provider(format!("{operation} response could not be read: {error}"))
    })?;
    if !status.is_success() {
        return Err(AppError::Provider(format!(
            "{operation} returned {status}: {}",
            crate::providers::safe_error_body(&body)
        )));
    }
    serde_json::from_str(&body)
        .map_err(|error| AppError::Provider(format!("Invalid {operation} response: {error}")))
}

#[cfg(test)]
#[path = "video_tests.rs"]
mod tests;
