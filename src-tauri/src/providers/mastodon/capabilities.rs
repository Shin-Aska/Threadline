use serde::Deserialize;

use crate::models::{CountingPolicy, PlatformCapabilities};

use super::MastodonProvider;

#[derive(Deserialize)]
struct Instance {
    configuration: Configuration,
}

#[derive(Deserialize)]
struct Configuration {
    statuses: Statuses,
    media_attachments: MediaAttachments,
}

#[derive(Deserialize)]
struct Statuses {
    max_characters: usize,
    max_media_attachments: usize,
    characters_reserved_per_url: usize,
}

#[derive(Deserialize)]
struct MediaAttachments {
    supported_mime_types: Vec<String>,
    video_size_limit: Option<usize>,
}

impl MastodonProvider {
    pub async fn discovered_capabilities(&self) -> PlatformCapabilities {
        let safe = PlatformCapabilities {
            max_text_length: 500,
            counting_policy: CountingPolicy::Grapheme,
            reserved_url_length: Some(23),
            max_media_attachments: 4,
            supported_media_types: vec![
                "image/jpeg".into(),
                "image/png".into(),
                "image/webp".into(),
            ],
            supports_polls: true,
            supports_content_warnings: true,
            max_video_bytes: None,
            max_video_duration_ms: None,
        };
        let response = match self
            .client
            .get(format!("{}/api/v2/instance", self.base_url))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => response,
            Ok(_) | Err(_) => return safe,
        };
        let instance: Instance = match response.json().await {
            Ok(instance) => instance,
            Err(_) => return safe,
        };
        let supports_video = instance
            .configuration
            .media_attachments
            .supported_mime_types
            .iter()
            .any(|mime| mime == "video/mp4");
        PlatformCapabilities {
            max_text_length: instance.configuration.statuses.max_characters,
            counting_policy: CountingPolicy::Grapheme,
            reserved_url_length: Some(instance.configuration.statuses.characters_reserved_per_url),
            max_media_attachments: instance.configuration.statuses.max_media_attachments,
            supported_media_types: instance
                .configuration
                .media_attachments
                .supported_mime_types
                .into_iter()
                .filter(|mime| {
                    mime.starts_with("image/") || (supports_video && mime == "video/mp4")
                })
                .collect(),
            supports_polls: true,
            supports_content_warnings: true,
            max_video_bytes: supports_video
                .then_some(instance.configuration.media_attachments.video_size_limit)
                .flatten(),
            max_video_duration_ms: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn discovers_instance_video_limit_from_http_fixture() {
        // Given an instance that explicitly supports MP4 with a byte limit.
        let body = r#"{"configuration":{"statuses":{"max_characters":700,"max_media_attachments":5,"characters_reserved_per_url":24},"media_attachments":{"supported_mime_types":["image/jpeg","video/mp4"],"video_size_limit":123456}}}"#;
        let (url, server) = crate::providers::test_http::server(vec![(200, body)]);
        let provider = MastodonProvider {
            capabilities: crate::accounts::mock_accounts()[1].capabilities.clone(),
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("client"),
            base_url: url,
            access_token: "token".into(),
        };

        // When capabilities are discovered through the provider boundary.
        let capabilities = provider.discovered_capabilities().await;

        // Then text, attachment, URL, MIME, and video limits use native values.
        assert_eq!(capabilities.max_text_length, 700);
        assert_eq!(capabilities.max_media_attachments, 5);
        assert_eq!(capabilities.reserved_url_length, Some(24));
        assert_eq!(capabilities.max_video_bytes, Some(123456));
        assert!(capabilities
            .supported_media_types
            .iter()
            .any(|mime| mime == "video/mp4"));
        server.join().expect("server");
    }
}
