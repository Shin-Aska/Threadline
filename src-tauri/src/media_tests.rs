use super::*;

fn mp4() -> Vec<u8> {
    let ftyp = [
        0, 0, 0, 16, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0, 0, 0, 0,
    ];
    let mvhd = [
        0, 0, 0, 32, b'm', b'v', b'h', b'd', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 232, 0,
        0, 7, 208, 0, 0, 0, 0,
    ];
    let mut bytes = Vec::from(ftyp);
    bytes.extend([0, 0, 0, 40, b'm', b'o', b'o', b'v']);
    bytes.extend(mvhd);
    bytes
}

fn video() -> MediaAttachment {
    let bytes = mp4();
    MediaAttachment {
        id: "video".into(),
        name: "video.mp4".into(),
        mime_type: "video/mp4".into(),
        size_bytes: bytes.len(),
        alt_text: "A short demo".into(),
        data_base64: STANDARD.encode(bytes),
        duration_ms: Some(2_000),
    }
}

fn image() -> MediaAttachment {
    MediaAttachment {
        id: "one".into(),
        name: "diagram.png".into(),
        mime_type: "image/png".into(),
        size_bytes: 8,
        alt_text: "A diagram".into(),
        data_base64: STANDARD.encode(b"\x89PNG\r\n\x1a\n"),
        duration_ms: None,
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

#[test]
fn accepts_one_well_formed_mp4_when_destination_limits_allow_it() {
    // Given a destination that explicitly advertises an MP4 byte limit.
    let mut account = crate::accounts::mock_accounts().remove(0);
    account.capabilities.supported_media_types = vec!["video/mp4".into()];
    account.capabilities.max_video_bytes = Some(1_000_000);

    // When the draft contains one valid MP4.
    let item = video();
    let result = validate(std::slice::from_ref(&item), &[&account]);

    // Then validation and binary preparation both succeed.
    assert!(result.is_ok());
    assert_eq!(prepare(&[item]).expect("video")[0].mime_type, "video/mp4");
}

#[test]
fn rejects_video_mixed_with_images_or_without_destination_limits() {
    // Given one video, one image, and a destination with unknown video limits.
    let account = crate::accounts::mock_accounts().remove(0);

    // When those invalid attachment combinations are validated.
    let mixed = validate(&[video(), image()], &[&account]);
    let unknown_limit = validate(&[video()], &[&account]);

    // Then both are rejected before any provider upload.
    assert!(mixed.is_err());
    assert!(unknown_limit
        .expect_err("unknown limit")
        .to_string()
        .contains("video limits are unavailable"));
}

#[test]
fn rejects_mp4_without_a_movie_header_before_upload() {
    // Given bytes with an MP4 file-type box but no movie metadata box.
    let mut item = video();
    let bytes = &mp4()[..16];
    item.size_bytes = bytes.len();
    item.data_base64 = STANDARD.encode(bytes);

    // When the attachment is prepared for a provider.
    let result = prepare(&[item]);

    // Then malformed video content is rejected locally.
    assert!(result
        .expect_err("malformed MP4")
        .to_string()
        .contains("valid MP4"));
}
