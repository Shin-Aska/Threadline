use super::OAuthError;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Default)]
pub struct OAuthCoordinator {
    inner: Arc<Mutex<HashMap<String, ActiveFlow>>>,
}

struct ActiveFlow {
    cancel: CancellationToken,
    deep_link: Option<oneshot::Sender<String>>,
}

pub struct OAuthFlow {
    flow_id: String,
    coordinator: OAuthCoordinator,
    pub cancel: CancellationToken,
    pub deep_link: Option<oneshot::Receiver<String>>,
}

impl OAuthCoordinator {
    pub fn start(&self, flow_id: &str, expects_deep_link: bool) -> Result<OAuthFlow, OAuthError> {
        if uuid::Uuid::parse_str(flow_id).is_err() {
            return Err(OAuthError::Configuration("OAuth flow ID is invalid".into()));
        }
        let cancel = CancellationToken::new();
        let (sender, receiver) = if expects_deep_link {
            let (sender, receiver) = oneshot::channel();
            (Some(sender), Some(receiver))
        } else {
            (None, None)
        };
        let mut flows = self
            .inner
            .lock()
            .map_err(|_| OAuthError::Listener("OAuth coordinator is unavailable".into()))?;
        if flows.contains_key(flow_id) {
            return Err(OAuthError::Configuration(
                "OAuth flow ID is already active".into(),
            ));
        }
        flows.insert(
            flow_id.into(),
            ActiveFlow {
                cancel: cancel.clone(),
                deep_link: sender,
            },
        );
        Ok(OAuthFlow {
            flow_id: flow_id.into(),
            coordinator: self.clone(),
            cancel,
            deep_link: receiver,
        })
    }

    pub fn cancel(&self, flow_id: &str) -> Result<(), OAuthError> {
        let flows = self
            .inner
            .lock()
            .map_err(|_| OAuthError::Listener("OAuth coordinator is unavailable".into()))?;
        let flow = flows
            .get(flow_id)
            .ok_or_else(|| OAuthError::Configuration("OAuth flow is not active".into()))?;
        flow.cancel.cancel();
        Ok(())
    }

    pub fn deliver_deep_link(&self, url: String) -> Result<(), OAuthError> {
        let mut flows = self
            .inner
            .lock()
            .map_err(|_| OAuthError::Listener("OAuth coordinator is unavailable".into()))?;
        let awaiting = flows
            .values()
            .filter(|flow| flow.deep_link.is_some())
            .count();
        if awaiting == 0 {
            return Err(OAuthError::Configuration(
                "no hosted OAuth login is awaiting a callback".into(),
            ));
        }
        if awaiting > 1 {
            return Err(OAuthError::Conflict(
                "more than one hosted OAuth login is awaiting a callback".into(),
            ));
        }
        let flow = flows
            .values_mut()
            .find(|flow| flow.deep_link.is_some())
            .ok_or_else(|| {
                OAuthError::Configuration("no hosted OAuth login is awaiting a callback".into())
            })?;
        let sender = flow.deep_link.take().ok_or_else(|| {
            OAuthError::Configuration("OAuth callback was already delivered".into())
        })?;
        sender.send(url).map_err(|_| OAuthError::Cancelled)
    }
}

impl Drop for OAuthFlow {
    fn drop(&mut self) {
        if let Ok(mut flows) = self.coordinator.inner.lock() {
            flows.remove(&self.flow_id);
        }
    }
}
