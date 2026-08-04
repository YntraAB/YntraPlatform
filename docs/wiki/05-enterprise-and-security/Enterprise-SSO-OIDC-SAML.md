# Enterprise SSO: OIDC & SAML 2.0 Integration

Yntra Platform supports enterprise **Single Sign-On (SSO)** connecting to identity providers like **Okta**, **Microsoft Entra ID (Azure AD)**, and **Ping Identity**.

---

## 1. Domain Auto-Detection Flow

```mermaid
sequenceDiagram
    participant User as Enterprise Worker
    participant Mobile as Mobile App / Dioxus
    participant Core as yntra-core (SSO)
    participant IdP as Enterprise IdP (Okta / Entra ID)

    User->>Mobile: Enters email (alice@acme-corp.com)
    Mobile->>Core: initiate_enterprise_sso("alice@acme-corp.com")
    Core-->>Mobile: EnterpriseSsoSession (Okta/Entra authorization_url)
    Mobile->>IdP: Redirects to Enterprise SSO Portal
    IdP-->>Mobile: Authorization Code Callback
    Mobile->>Core: complete_enterprise_sso_login(session_id, code)
    Core-->>Mobile: Authenticated WorkspaceUser
```

---

## 2. Configuration Parameters

Enterprise domains register OIDC client credentials via `initiate_enterprise_sso()`.
When a user enters an enterprise email domain (e.g. `@acme-corp.com`), Yntra automatically resolves the domain mapping and redirects to the enterprise authorization endpoint.
