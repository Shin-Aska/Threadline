use super::OAuthError;
use serde::Deserialize;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;
use url::Url;

const MAX_REQUEST_BYTES: usize = 8_192;

#[derive(Debug, PartialEq, Eq)]
pub struct CallbackPayload {
    pub code: String,
    pub state: String,
    pub issuer: Option<String>,
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    iss: Option<String>,
    error: Option<String>,
}

pub struct LoopbackCallback {
    listener: TcpListener,
    path: &'static str,
}

impl LoopbackCallback {
    pub async fn bind(path: &'static str) -> Result<Self, OAuthError> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|error| OAuthError::Listener(error.to_string()))?;
        Ok(Self { listener, path })
    }

    pub fn redirect_uri(&self) -> Result<String, OAuthError> {
        let address = self
            .listener
            .local_addr()
            .map_err(|error| OAuthError::Listener(error.to_string()))?;
        Ok(format!("http://127.0.0.1:{}{}", address.port(), self.path))
    }

    pub async fn wait(
        self,
        expected_state: Option<&str>,
        timeout: Duration,
        cancel: CancellationToken,
    ) -> Result<CallbackPayload, OAuthError> {
        let accepted = tokio::select! {
            _ = cancel.cancelled() => return Err(OAuthError::Cancelled),
            result = tokio::time::timeout(timeout, self.listener.accept()) => {
                result.map_err(|_| OAuthError::Timeout)?
                    .map_err(|error| OAuthError::Listener(error.to_string()))?
            }
        };
        read_callback(accepted.0, self.path, expected_state).await
    }
}

pub fn parse_callback_target(
    target: &str,
    expected_path: &str,
    expected_state: &str,
) -> Result<CallbackPayload, OAuthError> {
    parse_target(target, expected_path, Some(expected_state))
}

fn parse_target(
    target: &str,
    expected_path: &str,
    expected_state: Option<&str>,
) -> Result<CallbackPayload, OAuthError> {
    let url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| OAuthError::InvalidResponse("malformed callback URL".into()))?;
    if url.path() != expected_path {
        return Err(OAuthError::CallbackPath);
    }
    let query: CallbackQuery = serde_html_form::from_str(url.query().unwrap_or_default())
        .map_err(|_| OAuthError::InvalidResponse("malformed callback query".into()))?;
    if query.error.is_some() {
        return Err(OAuthError::ProviderDenied);
    }
    let state = query
        .state
        .ok_or_else(|| OAuthError::InvalidResponse("callback state is missing".into()))?;
    if expected_state.is_some_and(|expected| expected != state) {
        return Err(OAuthError::StateMismatch);
    }
    let code = query
        .code
        .ok_or_else(|| OAuthError::InvalidResponse("authorization code is missing".into()))?;
    Ok(CallbackPayload {
        code,
        state,
        issuer: query.iss,
    })
}

async fn read_callback(
    mut stream: TcpStream,
    expected_path: &str,
    expected_state: Option<&str>,
) -> Result<CallbackPayload, OAuthError> {
    let mut request = vec![0_u8; MAX_REQUEST_BYTES];
    let count = stream
        .read(&mut request)
        .await
        .map_err(|error| OAuthError::Listener(error.to_string()))?;
    let line = std::str::from_utf8(&request[..count])
        .ok()
        .and_then(|text| text.lines().next())
        .ok_or_else(|| OAuthError::InvalidResponse("malformed callback request".into()))?;
    let mut parts = line.split_whitespace();
    let method = parts.next();
    let target = parts.next();
    let result = match (method, target) {
        (Some("GET"), Some(target)) => parse_target(target, expected_path, expected_state),
        _ => Err(OAuthError::InvalidResponse("callback must use GET".into())),
    };
    let (status, message) = if result.is_ok() {
        (
            "200 OK",
            "Threadline login completed. You may close this window.",
        )
    } else {
        (
            "400 Bad Request",
            "Threadline could not complete this login.",
        )
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{message}",
        message.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|error| OAuthError::Listener(error.to_string()))?;
    result
}
