// lti.rs - IMS LTI 1.3 Advantage Protocol Service Implementation
use crate::infra::errors::YntraError;
use serde::{Deserialize, Serialize};

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LtiConfiguration {
    pub client_id: String,
    pub deployment_id: String,
    pub issuer: String,
    pub auth_login_url: String,
    pub key_set_url: String,
    pub target_link_uri: String,
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LtiOidcLoginParams {
    pub redirect_url: String,
    pub state: String,
    pub nonce: String,
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LtiContext {
    pub id: String,
    pub label: String,
    pub title: String,
    pub type_names: Vec<String>,
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LtiResourceLink {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(uniffi::Record, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LtiLaunchSession {
    pub session_id: String,
    pub issuer: String,
    pub client_id: String,
    pub deployment_id: String,
    pub user_sub: String,
    pub mapped_role: String,
    pub context: Option<LtiContext>,
    pub resource_link: Option<LtiResourceLink>,
    pub raw_claims_json: String,
}

fn percent_encode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }
    encoded
}

/// Initiates an IMS LTI 1.3 3rd Party OIDC Login Flow.
#[uniffi::export]
pub fn initiate_lti_oidc_login(
    config: LtiConfiguration,
    login_hint: String,
    lti_message_hint: Option<String>,
) -> Result<LtiOidcLoginParams, YntraError> {
    if config.client_id.trim().is_empty() || config.issuer.trim().is_empty() {
        return Err(YntraError::ValidationError(
            "LTI 1.3 client_id and issuer cannot be empty".to_string(),
        ));
    }

    let state = format!("lti_state_{}", uuid::Uuid::new_v4());
    let nonce = format!("lti_nonce_{}", uuid::Uuid::new_v4());

    let mut url = format!(
        "{}?response_type=id_token&response_mode=form_post&client_id={}&redirect_uri={}&scope=openid&state={}&nonce={}&login_hint={}",
        config.auth_login_url.trim_end_matches('/'),
        percent_encode(&config.client_id),
        percent_encode(&config.target_link_uri),
        state,
        nonce,
        percent_encode(&login_hint)
    );

    if let Some(hint) = lti_message_hint {
        if !hint.trim().is_empty() {
            url.push_str("&lti_message_hint=");
            url.push_str(&percent_encode(&hint));
        }
    }

    Ok(LtiOidcLoginParams {
        redirect_url: url,
        state,
        nonce,
    })
}

/// Validates an incoming LTI 1.3 Advantage Launch ID Token JSON payload.
#[uniffi::export]
pub fn validate_lti_launch_token(
    config: LtiConfiguration,
    id_token_json: String,
    expected_nonce: String,
) -> Result<LtiLaunchSession, YntraError> {
    let payload: serde_json::Value = serde_json::from_str(&id_token_json)
        .map_err(|e| YntraError::ValidationError(format!("Invalid LTI 1.3 ID token JSON: {}", e)))?;

    let obj = payload.as_object().ok_or_else(|| {
        YntraError::ValidationError("LTI 1.3 payload must be a JSON object".to_string())
    })?;

    // Verify mandatory LTI 1.3 claims
    let iss = obj.get("iss").and_then(|v| v.as_str()).unwrap_or("");
    if !iss.eq_ignore_ascii_case(&config.issuer) {
        return Err(YntraError::AuthError(format!(
            "LTI 1.3 issuer mismatch: expected {}, got {}",
            config.issuer, iss
        )));
    }

    let aud = obj.get("aud").and_then(|v| {
        if let Some(s) = v.as_str() {
            Some(vec![s.to_string()])
        } else if let Some(arr) = v.as_array() {
            Some(arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        } else {
            None
        }
    }).unwrap_or_default();

    if !aud.iter().any(|a| a.eq_ignore_ascii_case(&config.client_id)) {
        return Err(YntraError::AuthError(format!(
            "LTI 1.3 audience mismatch: client_id {} not in aud claims",
            config.client_id
        )));
    }

    let nonce = obj.get("nonce").and_then(|v| v.as_str()).unwrap_or("");
    if nonce != expected_nonce {
        return Err(YntraError::AuthError("LTI 1.3 nonce mismatch or replay attack detected".to_string()));
    }

    let msg_type = obj.get("https://purl.imsglobal.org/spec/lti/claim/message_type")
        .and_then(|v| v.as_str()).unwrap_or("");
    if msg_type != "LtiResourceLinkRequest" && msg_type != "LtiDeepLinkingRequest" {
        return Err(YntraError::ValidationError(format!(
            "Unsupported LTI 1.3 message_type: {}", msg_type
        )));
    }

    let version = obj.get("https://purl.imsglobal.org/spec/lti/claim/version")
        .and_then(|v| v.as_str()).unwrap_or("");
    if version != "1.3.0" {
        return Err(YntraError::ValidationError(format!(
            "Unsupported LTI version: {}, expected 1.3.0", version
        )));
    }

    let sub = obj.get("sub").and_then(|v| v.as_str()).unwrap_or("anonymous_lti_user");

    // Extract LTI roles & map to Yntra roles
    let roles_vec = obj.get("https://purl.imsglobal.org/spec/lti/claim/roles")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect::<Vec<String>>())
        .unwrap_or_default();

    let mapped_role = map_lti_roles_to_yntra(&roles_vec);

    // Extract optional LTI context claim
    let context = obj.get("https://purl.imsglobal.org/spec/lti/claim/context")
        .and_then(|v| v.as_object())
        .map(|c| LtiContext {
            id: c.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            label: c.get("label").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            title: c.get("title").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            type_names: c.get("type").and_then(|x| x.as_array())
                .map(|arr| arr.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        });

    // Extract optional LTI resource_link claim
    let resource_link = obj.get("https://purl.imsglobal.org/spec/lti/claim/resource_link")
        .and_then(|v| v.as_object())
        .map(|r| LtiResourceLink {
            id: r.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            title: r.get("title").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            description: r.get("description").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        });

    let session_id = format!("lti_sess_{}", uuid::Uuid::new_v4());

    Ok(LtiLaunchSession {
        session_id,
        issuer: config.issuer,
        client_id: config.client_id,
        deployment_id: config.deployment_id,
        user_sub: sub.to_string(),
        mapped_role,
        context,
        resource_link,
        raw_claims_json: id_token_json,
    })
}

fn map_lti_roles_to_yntra(lti_roles: &[String]) -> String {
    for role in lti_roles {
        let r_lower = role.to_lowercase();
        if r_lower.contains("administrator") || r_lower.contains("systemadministrator") {
            return "admin".to_string();
        }
        if r_lower.contains("instructor") || r_lower.contains("faculty") || r_lower.contains("teacher") {
            return "teacher".to_string();
        }
        if r_lower.contains("learner") || r_lower.contains("student") {
            return "student".to_string();
        }
    }
    "student".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_sample_config() -> LtiConfiguration {
        LtiConfiguration {
            client_id: "canvas_client_101".to_string(),
            deployment_id: "deploy_202".to_string(),
            issuer: "https://canvas.instructure.com".to_string(),
            auth_login_url: "https://canvas.instructure.com/api/lti/authorize_redirect".to_string(),
            key_set_url: "https://canvas.instructure.com/api/lti/security/jwks".to_string(),
            target_link_uri: "https://yntra.school.internal/lti/launch".to_string(),
        }
    }

    #[test]
    fn test_initiate_lti_oidc_login() {
        let config = get_sample_config();
        let params = initiate_lti_oidc_login(config, "user_hint_123".to_string(), Some("msg_hint_456".to_string())).unwrap();

        assert!(params.redirect_url.contains("client_id=canvas_client_101"));
        assert!(params.redirect_url.contains("login_hint=user_hint_123"));
        assert!(params.redirect_url.contains("lti_message_hint=msg_hint_456"));
        assert!(params.state.starts_with("lti_state_"));
        assert!(params.nonce.starts_with("lti_nonce_"));
    }

    #[test]
    fn test_validate_valid_lti_launch_token() {
        let config = get_sample_config();
        let nonce = "test_nonce_999".to_string();

        let id_token_json = serde_json::json!({
            "iss": "https://canvas.instructure.com",
            "aud": "canvas_client_101",
            "sub": "canvas_user_888",
            "nonce": "test_nonce_999",
            "https://purl.imsglobal.org/spec/lti/claim/message_type": "LtiResourceLinkRequest",
            "https://purl.imsglobal.org/spec/lti/claim/version": "1.3.0",
            "https://purl.imsglobal.org/spec/lti/claim/roles": [
                "http://purl.imsglobal.org/vocab/lis/v2/membership#Instructor"
            ],
            "https://purl.imsglobal.org/spec/lti/claim/context": {
                "id": "course_101",
                "label": "CS101",
                "title": "Computer Science 101",
                "type": ["CourseOffering"]
            },
            "https://purl.imsglobal.org/spec/lti/claim/resource_link": {
                "id": "link_55",
                "title": "Final Project Assignment"
            }
        }).to_string();

        let session = validate_lti_launch_token(config, id_token_json, nonce).unwrap();
        assert_eq!(session.user_sub, "canvas_user_888");
        assert_eq!(session.mapped_role, "teacher");
        assert!(session.context.is_some());
        assert_eq!(session.context.unwrap().label, "CS101");
        assert!(session.resource_link.is_some());
        assert_eq!(session.resource_link.unwrap().title, "Final Project Assignment");
    }

    #[test]
    fn test_validate_lti_launch_rejects_issuer_mismatch() {
        let config = get_sample_config();
        let id_token_json = serde_json::json!({
            "iss": "https://untrusted-lms.com",
            "aud": "canvas_client_101",
            "sub": "user_1",
            "nonce": "nonce_1",
            "https://purl.imsglobal.org/spec/lti/claim/message_type": "LtiResourceLinkRequest",
            "https://purl.imsglobal.org/spec/lti/claim/version": "1.3.0"
        }).to_string();

        let res = validate_lti_launch_token(config, id_token_json, "nonce_1".to_string());
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("issuer mismatch"));
    }

    #[test]
    fn test_validate_lti_launch_rejects_nonce_mismatch() {
        let config = get_sample_config();
        let id_token_json = serde_json::json!({
            "iss": "https://canvas.instructure.com",
            "aud": "canvas_client_101",
            "sub": "user_1",
            "nonce": "nonce_replay_attack",
            "https://purl.imsglobal.org/spec/lti/claim/message_type": "LtiResourceLinkRequest",
            "https://purl.imsglobal.org/spec/lti/claim/version": "1.3.0"
        }).to_string();

        let res = validate_lti_launch_token(config, id_token_json, "expected_fresh_nonce".to_string());
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("nonce mismatch"));
    }
}
