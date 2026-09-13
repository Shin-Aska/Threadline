use crate::{error::AppError, models::*, providers::SocialProvider};
use async_trait::async_trait;
use serde::Deserialize;
pub struct MastodonProvider {
    pub capabilities: PlatformCapabilities,
    pub client: reqwest::Client,
    pub base_url: String,
    pub access_token: String,
}

#[derive(Deserialize)]
struct StatusResponse {
    id: String,
}

#[derive(Deserialize)]
struct AccountResponse {
    id: String,
    username: String,
    acct: String,
    display_name: String,
}

impl MastodonProvider {
    pub async fn account(&self) -> Result<(String, String, String), AppError> {
        let response = self
            .client
            .get(format!(
                "{}/api/v1/accounts/verify_credentials",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Mastodon connection failed: {error}")))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::Provider(format!("Mastodon response could not be read: {error}"))
        })?;
        if !status.is_success() {
            return Err(AppError::Provider(format!(
                "Mastodon credentials were rejected ({status}): {}",
                crate::providers::safe_error_body(&body)
            )));
        }
        let profile: AccountResponse = serde_json::from_str(&body)
            .map_err(|error| AppError::Provider(format!("Invalid Mastodon profile: {error}")))?;
        let handle = if profile.acct.contains('@') {
            format!("@{}", profile.acct)
        } else {
            let host = reqwest::Url::parse(&self.base_url)
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned))
                .unwrap_or_default();
            format!("@{}@{host}", profile.username)
        };
        let display_name = if profile.display_name.trim().is_empty() {
            profile.username
        } else {
            profile.display_name
        };
        Ok((profile.id, handle, display_name))
    }

    async fn create_status(
        &self,
        post: PreparedPost,
        in_reply_to_id: Option<&str>,
    ) -> Result<PublishedPost, AppError> {
        let mut form = vec![("status", post.text)];
        if let Some(parent) = in_reply_to_id {
            form.push(("in_reply_to_id", parent.to_owned()));
        }
        let response = self
            .client
            .post(format!(
                "{}/api/v1/statuses",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.access_token)
            .header("Idempotency-Key", uuid::Uuid::new_v4().to_string())
            .form(&form)
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Mastodon request failed: {error}")))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::Provider(format!("Mastodon response could not be read: {error}"))
        })?;
        if !status.is_success() {
            return Err(AppError::Provider(format!(
                "Mastodon returned {status}: {}",
                crate::providers::safe_error_body(&body)
            )));
        }
        let status: StatusResponse = serde_json::from_str(&body)
            .map_err(|error| AppError::Provider(format!("Invalid Mastodon response: {error}")))?;
        Ok(PublishedPost {
            remote_id: status.id,
            remote_cid: None,
            root_id: None,
            root_cid: None,
        })
    }
}
#[async_trait]
impl SocialProvider for MastodonProvider {
    async fn capabilities(&self) -> Result<PlatformCapabilities, AppError> {
        Ok(self.capabilities.clone())
    }
    async fn publish(&self, post: PreparedPost) -> Result<PublishedPost, AppError> {
        self.create_status(post, None).await
    }
    async fn reply(
        &self,
        parent: &PublishedPost,
        post: PreparedPost,
    ) -> Result<PublishedPost, AppError> {
        self.create_status(post, Some(&parent.remote_id)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[tokio::test]
    async fn publishes_status_with_authentication() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().expect("accept");
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("timeout");
            let mut bytes = [0; 4096];
            let count = socket.read(&mut bytes).expect("request");
            let request = String::from_utf8_lossy(&bytes[..count]);
            assert!(request.starts_with("POST /api/v1/statuses HTTP/1.1"));
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer token"));
            assert!(request.contains("status=hello"));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"id\":\"42\"}")
                .expect("response");
        });
        let provider = MastodonProvider {
            capabilities: crate::accounts::mock_accounts()[1].capabilities.clone(),
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("client"),
            base_url: format!("http://{address}"),
            access_token: "token".into(),
        };
        let published = provider
            .publish(PreparedPost {
                text: "hello".into(),
            })
            .await
            .expect("publish");
        assert_eq!(published.remote_id, "42");
        server.join().expect("server");
    }
}
