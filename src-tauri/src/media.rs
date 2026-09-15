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

pub fn validate(images: &[MediaAttachment], accounts: &[&Account]) -> Result<(), AppError> {
    if images.len() > MAX_IMAGES {
        return Err(AppError::Validation("attach at most four images".into()));
    }
    for image in images {
        if !IMAGE_TYPES.contains(&image.mime_type.as_str()) {
            return Err(AppError::Validation("use JPEG, PNG, or WebP images".into()));
        }
        if image.size_bytes == 0 || image.size_bytes > MAX_IMAGE_BYTES {
            return Err(AppError::Validation(
                "each image must be between 1 byte and 2 MB".into(),
            ));
        }
        if image.alt_text.chars().count() > MAX_ALT_CHARS {
            return Err(AppError::Validation(
                "image alt text must be 1,500 characters or fewer".into(),
            ));
        }
    }
    for account in accounts {
        if images.len() > account.capabilities.max_media_attachments {
            return Err(AppError::Validation(format!(
                "{} allows at most {} attachments",
                account.handle, account.capabilities.max_media_attachments
            )));
        }
        if images.iter().any(|image| {
            !account
                .capabilities
                .supported_media_types
                .iter()
                .any(|mime| mime == &image.mime_type)
        }) {
            return Err(AppError::Validation(format!(
                "{} does not support one of these image types",
                account.handle
            )));
        }
    }
    Ok(())
}

pub fn prepare(images: &[MediaAttachment]) -> Result<Vec<PreparedMedia>, AppError> {
    validate(images, &[])?;
    images
        .iter()
        .map(|image| {
            if image.data_base64.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 {
                return Err(AppError::Validation("image exceeds 2 MB".into()));
            }
            let bytes = STANDARD
                .decode(&image.data_base64)
                .map_err(|_| AppError::Validation("image data is not valid base64".into()))?;
            if bytes.len() != image.size_bytes {
                return Err(AppError::Validation(
                    "image size does not match its content".into(),
                ));
            }
            let valid = match image.mime_type.as_str() {
                "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
                "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
                _ => false,
            };
            if !valid {
                return Err(AppError::Validation(
                    "image content does not match its file type".into(),
                ));
            }
            Ok(PreparedMedia {
                mime_type: image.mime_type.clone(),
                data: Arc::from(bytes),
                alt_text: image.alt_text.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image() -> MediaAttachment {
        MediaAttachment {
            id: "one".into(),
            mime_type: "image/png".into(),
            size_bytes: 8,
            alt_text: "A diagram".into(),
            data_base64: STANDARD.encode(b"\x89PNG\r\n\x1a\n"),
        }
    }
    #[test]
    fn rejects_mismatched_bytes_before_upload() {
        let mut item = image();
        item.data_base64 = STANDARD.encode(b"notimage");
        assert!(prepare(&[item]).is_err());
    }
    #[test]
    fn checks_count_size_alt_and_destination_support() {
        assert!(validate(&vec![image(); 5], &[]).is_err());
        let mut item = image();
        item.size_bytes = MAX_IMAGE_BYTES + 1;
        assert!(validate(&[item], &[]).is_err());
        let mut item = image();
        item.alt_text = "x".repeat(MAX_ALT_CHARS + 1);
        assert!(validate(&[item], &[]).is_err());
        let mut account = crate::accounts::mock_accounts().remove(0);
        account.capabilities.supported_media_types.clear();
        assert!(validate(&[image()], &[&account]).is_err());
    }
    #[test]
    fn prepares_bytes_and_preserves_unicode_alt_text() {
        let mut item = image();
        item.alt_text = "猫 and a tree 🌳".into();
        let prepared = prepare(&[item]).expect("image");
        assert_eq!(&*prepared[0].data, b"\x89PNG\r\n\x1a\n");
        assert_eq!(prepared[0].alt_text, "猫 and a tree 🌳");
    }
}
