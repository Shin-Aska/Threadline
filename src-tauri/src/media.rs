use crate::{
    error::AppError,
    models::{Account, MediaAttachment, PreparedMedia},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;

pub const MAX_IMAGES: usize = 4;
pub const MAX_IMAGE_BYTES: usize = 2_000_000;
pub const MAX_ALT_CHARS: usize = 1_500;
const IMAGE_TYPES: [&str; 3] = ["image/jpeg", "image/png", "image/webp"];
const VIDEO_TYPE: &str = "video/mp4";

pub fn validate(media: &[MediaAttachment], accounts: &[&Account]) -> Result<(), AppError> {
    let video = media.iter().find(|item| item.mime_type == VIDEO_TYPE);
    if video.is_some() && media.len() != 1 {
        return Err(AppError::Validation(
            "attach either one video or up to four images".into(),
        ));
    }
    if video.is_none() && media.len() > MAX_IMAGES {
        return Err(AppError::Validation("attach at most four images".into()));
    }
    for item in media {
        let is_video = item.mime_type == VIDEO_TYPE;
        if !is_video && !IMAGE_TYPES.contains(&item.mime_type.as_str()) {
            return Err(AppError::Validation(
                "use JPEG, PNG, or WebP images, or one MP4 video".into(),
            ));
        }
        if item.size_bytes == 0 || (!is_video && item.size_bytes > MAX_IMAGE_BYTES) {
            return Err(AppError::Validation(
                "each image must be between 1 byte and 2 MB, and video must not be empty".into(),
            ));
        }
        if item.alt_text.chars().count() > MAX_ALT_CHARS {
            return Err(AppError::Validation(
                "media alt text must be 1,500 characters or fewer".into(),
            ));
        }
        if is_video && item.duration_ms == Some(0) {
            return Err(AppError::Validation(
                "video duration must be greater than zero".into(),
            ));
        }
    }
    for account in accounts {
        if media.len() > account.capabilities.max_media_attachments {
            return Err(AppError::Validation(format!(
                "{} allows at most {} attachments",
                account.handle, account.capabilities.max_media_attachments
            )));
        }
        if media.iter().any(|item| {
            !account
                .capabilities
                .supported_media_types
                .iter()
                .any(|mime| mime == &item.mime_type)
        }) {
            return Err(AppError::Validation(format!(
                "{} does not support one of these media types",
                account.handle
            )));
        }
        if let Some(item) = video {
            let Some(max_bytes) = account.capabilities.max_video_bytes else {
                return Err(AppError::Validation(format!(
                    "{} video limits are unavailable; reconnect the account before publishing",
                    account.handle
                )));
            };
            if item.size_bytes > max_bytes {
                return Err(AppError::Validation(format!(
                    "{} allows videos up to {max_bytes} bytes",
                    account.handle
                )));
            }
            if let (Some(duration_ms), Some(max_duration_ms)) =
                (item.duration_ms, account.capabilities.max_video_duration_ms)
            {
                if duration_ms > max_duration_ms {
                    return Err(AppError::Validation(format!(
                        "{} allows videos up to {max_duration_ms} milliseconds",
                        account.handle
                    )));
                }
            }
        }
    }
    Ok(())
}

pub fn prepare(media: &[MediaAttachment]) -> Result<Vec<PreparedMedia>, AppError> {
    validate(media, &[])?;
    media
        .iter()
        .map(|item| {
            let encoded_limit = item.size_bytes.div_ceil(3).saturating_mul(4);
            if item.data_base64.len() > encoded_limit.saturating_add(4) {
                return Err(AppError::Validation(
                    "media data exceeds its declared size".into(),
                ));
            }
            let bytes = STANDARD
                .decode(&item.data_base64)
                .map_err(|_| AppError::Validation("media data is not valid base64".into()))?;
            if bytes.len() != item.size_bytes {
                return Err(AppError::Validation(
                    "media size does not match its content".into(),
                ));
            }
            let valid = match item.mime_type.as_str() {
                "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
                "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
                VIDEO_TYPE => mp4_duration_ms(&bytes).is_some(),
                _ => false,
            };
            if !valid {
                let message = if item.mime_type == VIDEO_TYPE {
                    "video content is not a valid MP4"
                } else {
                    "image content does not match its file type"
                };
                return Err(AppError::Validation(message.into()));
            }
            Ok(PreparedMedia {
                mime_type: item.mime_type.clone(),
                data: Arc::from(bytes),
                alt_text: item.alt_text.clone(),
            })
        })
        .collect()
}

fn mp4_duration_ms(bytes: &[u8]) -> Option<u64> {
    find_box(bytes, *b"ftyp")?;
    let movie = find_box(bytes, *b"moov")?;
    let header = find_box(movie, *b"mvhd")?;
    let version = *header.first()?;
    let (timescale_offset, duration_offset, duration_size) = match version {
        0 => (12, 16, 4),
        1 => (20, 24, 8),
        _ => return None,
    };
    let timescale = read_u32(header.get(timescale_offset..timescale_offset + 4)?)? as u64;
    let duration = match duration_size {
        4 => read_u32(header.get(duration_offset..duration_offset + 4)?)? as u64,
        8 => read_u64(header.get(duration_offset..duration_offset + 8)?)?,
        _ => return None,
    };
    (timescale > 0 && duration > 0)
        .then(|| duration.saturating_mul(1_000).checked_div(timescale))
        .flatten()
}

fn find_box(bytes: &[u8], wanted: [u8; 4]) -> Option<&[u8]> {
    let mut offset = 0usize;
    while offset.checked_add(8)? <= bytes.len() {
        let size32 = read_u32(bytes.get(offset..offset + 4)?)? as usize;
        let kind = bytes.get(offset + 4..offset + 8)?;
        let (size, header_size) = if size32 == 1 {
            (read_u64(bytes.get(offset + 8..offset + 16)?)? as usize, 16)
        } else if size32 == 0 {
            (bytes.len().checked_sub(offset)?, 8)
        } else {
            (size32, 8)
        };
        if size < header_size {
            return None;
        }
        let end = offset.checked_add(size)?;
        if end > bytes.len() {
            return None;
        }
        if kind == wanted {
            return bytes.get(offset + header_size..end);
        }
        offset = end;
    }
    None
}

fn read_u32(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.try_into().ok()?))
}

fn read_u64(bytes: &[u8]) -> Option<u64> {
    Some(u64::from_be_bytes(bytes.try_into().ok()?))
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;
