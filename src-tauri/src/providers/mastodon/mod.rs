mod hashtags;
mod media;
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
        let media_ids = self.upload_images(&post.media).await?;
        let mut form = vec![("status", post.text)];
        form.extend(media_ids.into_iter().map(|id| ("media_ids[]", id)));
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
            .timeout(std::time::Duration::from_secs(60))
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
    async fn timeline(
        &self,
        account_id: &str,
        account_handle: &str,
        cursor: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        let mut req = self
            .client
            .get(format!(
                "{}/api/v1/timelines/home",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.access_token)
            .query(&[("limit", "40")]);
        if let Some(max_id) = cursor {
            req = req.query(&[("max_id", max_id)]);
        }
        let response = req
            .send()
            .await
            .map_err(|e| AppError::Provider(format!("Mastodon timeline failed: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Provider(format!("Mastodon timeline failed: {e}")))?;
        let items: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| AppError::Provider(format!("Invalid Mastodon timeline: {e}")))?;
        let cleaner =
            regex::Regex::new("<[^>]+>").map_err(|e| AppError::Provider(e.to_string()))?;
        let posts=items.iter().filter_map(|wrapper| {let p=wrapper.get("reblog").filter(|v|!v.is_null()).unwrap_or(wrapper);let id=p["id"].as_str()?;let actor=&p["account"];let content=cleaner.replace_all(p["content"].as_str().unwrap_or(""),"").to_string();let media=p["media_attachments"].as_array().into_iter().flatten().filter_map(|m|Some(serde_json::json!({"url":m["url"].as_str()?,"alt":m["description"].as_str().unwrap_or(""),"type":m["type"].as_str().unwrap_or("image")}))).collect::<Vec<_>>();Some(serde_json::json!({"canonicalKey":format!("MASTODON:{}:{id}",self.base_url),"provider":"MASTODON","remoteId":id,"remoteUrl":p["url"].as_str().unwrap_or(""),"author":{"id":actor["id"],"displayName":actor["display_name"].as_str().filter(|v|!v.is_empty()).unwrap_or(actor["acct"].as_str().unwrap_or("")),"handle":actor["acct"].as_str().unwrap_or(""),"avatarUrl":actor["avatar"].as_str()},"text":content,"createdAt":p["created_at"].as_str().unwrap_or(""),"media":media,"sources":[{"accountId":account_id,"accountHandle":account_handle,"provider":"MASTODON"}],"metrics":{"replies":p["replies_count"],"reposts":p["reblogs_count"],"likes":p["favourites_count"]},"capabilities":{"openOriginal":true,"reply":false,"like":false,"repost":false}}))}).collect::<Vec<_>>();
        let cursor = items
            .last()
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str());
        Ok(serde_json::json!({"posts":posts,"cursor":cursor}))
    }
    async fn discovery(
        &self,
        account_id: &str,
        account_handle: &str,
    ) -> Result<serde_json::Value, AppError> {
        let tags: Vec<serde_json::Value> = self
            .client
            .get(format!(
                "{}/api/v1/trends/tags",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.access_token)
            .query(&[("limit", "20")])
            .send()
            .await
            .map_err(|e| AppError::Provider(format!("Mastodon trends failed: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Provider(format!("Mastodon trends failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Provider(format!("Invalid Mastodon trends: {e}")))?;
        let topics=tags.into_iter().filter_map(|t|{let name=t["name"].as_str()?;let history=t["history"].as_array().map(|h|h.iter().filter_map(|x|x["uses"].as_str()?.parse::<u64>().ok()).collect::<Vec<_>>()).unwrap_or_default();let count=history.iter().sum::<u64>();Some(serde_json::json!({"key":name.to_lowercase(),"name":name,"sources":[{"accountId":account_id,"accountHandle":account_handle,"provider":"MASTODON"}],"postCount":count,"history":history}))}).collect::<Vec<_>>();
        Ok(serde_json::json!({"topics":topics,"suggestedAccounts":[],"popularPosts":[]}))
    }
    async fn following_sources(&self, account_id: &str) -> Result<serde_json::Value, AppError> {
        let tags: Vec<serde_json::Value> = self
            .client
            .get(format!(
                "{}/api/v1/followed_tags",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| AppError::Provider(format!("Mastodon followed tags failed: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Provider(format!("Mastodon followed tags failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Provider(format!("Invalid Mastodon followed tags: {e}")))?;
        Ok(serde_json::json!(tags.into_iter().filter_map(|t|{let name=t["name"].as_str()?;Some(serde_json::json!({"id":format!("MASTODON:{}:{name}",self.base_url),"provider":"MASTODON","type":"TOPIC","title":format!("#{name}"),"description":"Followed Mastodon hashtag","accountId":account_id,"remoteId":name}))}).collect::<Vec<_>>()))
    }
    async fn hashtags(
        &self,
        query: &str,
    ) -> Result<Vec<crate::hashtags::HashtagSuggestion>, AppError> {
        self.search_hashtags(query).await
    }
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
                media: vec![],
            })
            .await
            .expect("publish");
        assert_eq!(published.remote_id, "42");
        server.join().expect("server");
    }
}
