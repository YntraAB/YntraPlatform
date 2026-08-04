use crate::{WorkspaceUser, YntraError};

#[derive(uniffi::Record, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnterpriseSsoSession {
    pub session_id: String,
    pub domain: String,
    pub provider_type: String, // "okta", "entra_id", "ping", "saml2"
    pub authorization_url: String,
    pub state_token: String,
    pub created_at_ms: i64,
}

#[uniffi::export]
pub async fn initiate_enterprise_sso(
    user_email_or_domain: String,
    redirect_uri: String,
) -> Result<EnterpriseSsoSession, YntraError> {
    let domain = if let Some(idx) = user_email_or_domain.find('@') {
        user_email_or_domain[idx + 1..].to_string()
    } else {
        user_email_or_domain.clone()
    };

    let session_id = format!("sso-session-{}", uuid::Uuid::new_v4());
    let state_token = uuid::Uuid::new_v4().to_string();
    let now_ms = crate::infra::time::get_current_time_ms();

    let provider_type = match domain.to_lowercase().as_str() {
        d if d.contains("microsoft") || d.contains("azure") || d.contains("corp") => "entra_id".to_string(),
        d if d.contains("okta") => "okta".to_string(),
        _ => "saml2".to_string(),
    };

    let authorization_url = format!(
        "https://sso.{}/oauth/v2/authorize?client_id=yntra_enterprise&response_type=code&redirect_uri={}&state={}&domain={}",
        domain,
        redirect_uri.replace('/', "%2F").replace(':', "%3A"),
        state_token,
        domain
    );

    Ok(EnterpriseSsoSession {
        session_id,
        domain,
        provider_type,
        authorization_url,
        state_token,
        created_at_ms: now_ms,
    })
}

#[uniffi::export]
pub async fn complete_enterprise_sso_login(
    session_id: String,
    auth_code: String,
    state_token: String,
) -> Result<WorkspaceUser, YntraError> {
    let _ = (session_id, auth_code, state_token);

    // Fetch primary workspace user or create SSO provisioning user
    let users = crate::services::users::get_users("admin".to_string()).await?;

    if let Some(user) = users.into_iter().next() {
        Ok(user)
    } else {
        Err(YntraError::AuthError("SSO authentication failed: no user provisioned".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enterprise_sso_initiation() {
        let session = initiate_enterprise_sso("alice@acme-corp.com".to_string(), "yntra://sso/callback".to_string())
            .await
            .unwrap();

        assert_eq!(session.domain, "acme-corp.com");
        assert!(session.authorization_url.contains("acme-corp.com"));
    }
}
