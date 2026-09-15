use crate::{error::AppError, models::Account};

pub(super) fn mastodon_account_id(
    base_url: &str,
    remote_id: &str,
    existing: &[Account],
) -> Result<String, AppError> {
    let canonical = reqwest::Url::parse(base_url)
        .map_err(|error| AppError::Validation(format!("Invalid Mastodon server: {error}")))?;
    let server = canonical.as_str().trim_end_matches('/');
    let legacy = format!("mastodon-{remote_id}");
    if existing.iter().any(|account| {
        account.id == legacy
            && matches!(account.provider, crate::models::ProviderKind::Mastodon)
            && account
                .instance_url
                .as_deref()
                .and_then(|url| reqwest::Url::parse(url).ok())
                .is_some_and(|url| url.as_str().trim_end_matches('/') == server)
    }) {
        return Ok(legacy);
    }
    Ok(format!("mastodon-{remote_id}@{server}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ProviderKind;
    #[test]
    fn same_remote_id_on_different_instances_has_distinct_identity() {
        let first = mastodon_account_id("https://one.example", "42", &[]).expect("id");
        let second = mastodon_account_id("https://two.example", "42", &[]).expect("id");
        assert_ne!(first, second);
    }
    #[test]
    fn reconnect_preserves_matching_legacy_account_and_normalizes_server() {
        let mut account = crate::accounts::mock_accounts().remove(1);
        account.id = "mastodon-42".into();
        account.provider = ProviderKind::Mastodon;
        account.instance_url = Some("https://one.example".into());
        assert_eq!(
            mastodon_account_id("https://ONE.example/", "42", &[account.clone()]).expect("id"),
            account.id
        );
        assert_ne!(
            mastodon_account_id("https://two.example", "42", &[account.clone()]).expect("id"),
            account.id
        );
        assert_eq!(
            mastodon_account_id("https://ONE.example/", "42", &[]).expect("id"),
            mastodon_account_id("https://one.example", "42", &[]).expect("id")
        );
    }
}
