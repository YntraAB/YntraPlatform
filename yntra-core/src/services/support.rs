use crate::database;
use crate::observer::notify_observers;
use crate::{
    HelpdeskArticle, ProductTourStep, SupportTicket, SupportTicketMessage, UserTourProgress,
    YntraError,
};

/// Seed default helpdesk knowledge base articles if table is empty
async fn seed_helpdesk_articles_if_empty(conn: &database::DbConnection) -> Result<(), YntraError> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM helpdesk_articles", (), |r| r.get(0))
        .await
        .unwrap_or(0);

    if count > 0 {
        return Ok(());
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let default_articles = vec![
        (
            "article-1",
            "Getting Started with Yntra Workspace Onboarding",
            "Onboarding",
            "Learn how to set up your team workspace, activate operational modules, and invite members.",
            "# Getting Started with Yntra Platform\n\nWelcome to Yntra! Follow these steps to configure your workspace:\n\n1. **Workspace Setup**: Configure team identity and brand colors in Admin Panel.\n2. **Module Activation**: Toggle Messaging, Shift Scheduling, RUT Exports, and Care Journaling.\n3. **Team Invites**: Generate secure email invitation links for team members.\n4. **Security & RBAC**: Assign Role-Based Access Control permissions.",
            "[\"onboarding\", \"setup\", \"admin\"]",
        ),
        (
            "article-2",
            "SITHS Smart Card & NFC Hardware Authentication Guide",
            "Security",
            "Step-by-step instructions for hardware passkey and SITHS smart card authentication.",
            "# SITHS Smart Card Authentication\n\nYntra supports hardware-backed smart card authentication:\n\n* Insert your SITHS card into the connected smart card reader.\n* Select **Hardware Login** on the login screen.\n* Enter your 6-digit PIN when prompted.\n* Your cryptographic Ed25519 signature will be validated automatically.",
            "[\"security\", \"siths\", \"hardware\", \"nfc\"]",
        ),
        (
            "article-3",
            "Managing Seat Licenses & Payment Gateways (Stripe, RevenueCat, Chargebee)",
            "Billing",
            "How to upgrade subscription tiers, allocate seat licenses, and export automated invoices.",
            "# Subscription & Billing Management\n\nNavigate to **Settings -> Billing & Subscriptions** to manage your subscription:\n\n* **Select Gateway**: Choose Stripe for web card payments, RevenueCat for mobile In-App purchases, or Chargebee for B2B portal billing.\n* **Adjust Seats**: Use the seat allocation counter to scale your active user licenses.\n* **Export Invoices**: Download automated PDF and JSON platform invoices at any time.",
            "[\"billing\", \"subscription\", \"stripe\", \"invoices\"]",
        ),
        (
            "article-4",
            "CalDAV Roster Sync & Shift Dispatch Setup",
            "Scheduling",
            "How to synchronize employee shift calendars with external CalDAV software.",
            "# CalDAV Shift Scheduling Sync\n\nTo connect Yntra Shift Scheduling to Apple Calendar or Outlook:\n\n1. Open **Settings -> Scheduler**.\n2. Copy your workspace CalDAV Feed URL.\n3. Subscribe in your calendar app using your Yntra application password.",
            "[\"caldav\", \"calendar\", \"scheduling\", \"shifts\"]",
        ),
    ];

    for (id, title, cat, summary, content, tags) in default_articles {
        let _ = conn.execute(
            "INSERT INTO helpdesk_articles (id, title, category, summary, content_markdown, tags_json, views_count, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 12, ?7)",
            crate::params![id, title, cat, summary, content, tags, now_ms],
        ).await;
    }

    Ok(())
}

#[uniffi::export]
pub async fn create_support_ticket(
    requester_user_id: String,
    workspace_id: String,
    subject: String,
    category: String,
    priority: String,
    initial_message: String,
) -> Result<SupportTicket, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let ticket_id = format!("ticket-{}", uuid::Uuid::new_v4());
    let now_ms = crate::infra::time::get_current_time_ms();
    let cat_norm = category.to_lowercase();
    let prio_norm = priority.to_lowercase();

    let (user_name, user_email): (String, String) = conn
        .query_row(
            "SELECT COALESCE(full_name, 'Workspace Member'), COALESCE(email, 'user@workspace.io') FROM users WHERE id = ?1",
            crate::params![&requester_user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .await
        .unwrap_or(("Workspace Member".to_string(), "user@workspace.io".to_string()));

    let first_msg = SupportTicketMessage {
        sender_id: requester_user_id.clone(),
        sender_name: user_name.clone(),
        is_staff: false,
        message: initial_message,
        timestamp: now_ms,
    };

    let messages_json =
        serde_json::to_string(&vec![first_msg]).map_err(|e| YntraError::DbError(e.to_string()))?;

    conn.execute(
        "INSERT INTO support_tickets (\
         id, workspace_id, user_id, user_name, user_email, subject, category, priority, status, messages_json, created_at, updated_at, sync_status\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'open', ?9, ?10, ?11, 'synced')",
        crate::params![
            &ticket_id,
            &workspace_id,
            &requester_user_id,
            &user_name,
            &user_email,
            &subject,
            &cat_norm,
            &prio_norm,
            &messages_json,
            now_ms,
            now_ms
        ],
    )
    .await?;

    notify_observers();

    Ok(SupportTicket {
        id: ticket_id,
        workspace_id,
        user_id: requester_user_id,
        user_name,
        user_email,
        subject,
        category: cat_norm,
        priority: prio_norm,
        status: "open".to_string(),
        messages_json,
        created_at: now_ms,
        updated_at: now_ms,
    })
}

#[uniffi::export]
pub async fn add_support_ticket_message(
    requester_user_id: String,
    workspace_id: String,
    ticket_id: String,
    message: String,
) -> Result<SupportTicket, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let mut ticket = conn
        .query_row(
            "SELECT id, workspace_id, user_id, user_name, user_email, subject, category, priority, status, messages_json, created_at, updated_at \
             FROM support_tickets WHERE id = ?1 AND workspace_id = ?2",
            crate::params![&ticket_id, &workspace_id],
            |row| {
                Ok(SupportTicket {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    user_id: row.get(2)?,
                    user_name: row.get(3)?,
                    user_email: row.get(4)?,
                    subject: row.get(5)?,
                    category: row.get(6)?,
                    priority: row.get(7)?,
                    status: row.get(8)?,
                    messages_json: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .await?;

    let mut messages: Vec<SupportTicketMessage> =
        serde_json::from_str(&ticket.messages_json).unwrap_or_default();

    let now_ms = crate::infra::time::get_current_time_ms();
    let is_staff = auth.role == "admin" || auth.role == "platform_admin";
    let sender_name = if is_staff {
        "Yntra Support Specialist".to_string()
    } else {
        ticket.user_name.clone()
    };

    messages.push(SupportTicketMessage {
        sender_id: requester_user_id,
        sender_name,
        is_staff,
        message,
        timestamp: now_ms,
    });

    let new_messages_json =
        serde_json::to_string(&messages).map_err(|e| YntraError::DbError(e.to_string()))?;

    let new_status = if is_staff { "in_progress" } else { "open" };

    conn.execute(
        "UPDATE support_tickets SET messages_json = ?1, status = ?2, updated_at = ?3, sync_status = 'pending' WHERE id = ?4",
        crate::params![&new_messages_json, new_status, now_ms, &ticket_id],
    )
    .await?;

    ticket.messages_json = new_messages_json;
    ticket.status = new_status.to_string();
    ticket.updated_at = now_ms;

    notify_observers();

    Ok(ticket)
}

#[uniffi::export]
pub async fn get_user_support_tickets(
    requester_user_id: String,
    workspace_id: String,
) -> Result<Vec<SupportTicket>, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let (sql, params) = if auth.is_admin {
        (
            "SELECT id, workspace_id, user_id, user_name, user_email, subject, category, priority, status, messages_json, created_at, updated_at \
             FROM support_tickets WHERE workspace_id = ?1 ORDER BY updated_at DESC",
            crate::params![&workspace_id],
        )
    } else {
        (
            "SELECT id, workspace_id, user_id, user_name, user_email, subject, category, priority, status, messages_json, created_at, updated_at \
             FROM support_tickets WHERE workspace_id = ?1 AND user_id = ?2 ORDER BY updated_at DESC",
            crate::params![&workspace_id, &requester_user_id],
        )
    };

    let mut stmt = conn.prepare(sql).await?;
    let tickets = stmt
        .query_map(params, |row| {
            Ok(SupportTicket {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                user_id: row.get(2)?,
                user_name: row.get(3)?,
                user_email: row.get(4)?,
                subject: row.get(5)?,
                category: row.get(6)?,
                priority: row.get(7)?,
                status: row.get(8)?,
                messages_json: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .await?;

    Ok(tickets)
}

#[uniffi::export]
pub async fn update_support_ticket_status(
    requester_user_id: String,
    workspace_id: String,
    ticket_id: String,
    status: String,
) -> Result<SupportTicket, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let now_ms = crate::infra::time::get_current_time_ms();
    let status_norm = status.to_lowercase();

    conn.execute(
        "UPDATE support_tickets SET status = ?1, updated_at = ?2, sync_status = 'pending' WHERE id = ?3 AND workspace_id = ?4",
        crate::params![&status_norm, now_ms, &ticket_id, &workspace_id],
    )
    .await?;

    let tickets = get_user_support_tickets(requester_user_id, workspace_id).await?;
    let ticket = tickets
        .into_iter()
        .find(|t| t.id == ticket_id)
        .ok_or_else(|| YntraError::NotFoundError(format!("Ticket {} not found", ticket_id)))?;

    notify_observers();

    Ok(ticket)
}

#[uniffi::export]
pub async fn search_helpdesk_articles(
    requester_user_id: String,
    query: String,
    category: String,
) -> Result<Vec<HelpdeskArticle>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    seed_helpdesk_articles_if_empty(&conn).await?;

    let q_clean = query.trim().to_lowercase();
    let cat_clean = category.trim().to_lowercase();

    let mut stmt = conn
        .prepare(
            "SELECT id, title, category, summary, content_markdown, tags_json, views_count, created_at \
             FROM helpdesk_articles ORDER BY views_count DESC",
        )
        .await?;

    let articles = stmt
        .query_map((), |row| {
            Ok(HelpdeskArticle {
                id: row.get(0)?,
                title: row.get(1)?,
                category: row.get(2)?,
                summary: row.get(3)?,
                content_markdown: row.get(4)?,
                tags_json: row.get(5)?,
                views_count: row.get::<i64>(6)? as u32,
                created_at: row.get(7)?,
            })
        })
        .await?;

    let filtered = articles
        .into_iter()
        .filter(|art| {
            let matches_cat = cat_clean.is_empty()
                || cat_clean == "all"
                || art.category.to_lowercase() == cat_clean;
            let matches_q = q_clean.is_empty()
                || art.title.to_lowercase().contains(&q_clean)
                || art.summary.to_lowercase().contains(&q_clean)
                || art.content_markdown.to_lowercase().contains(&q_clean);
            matches_cat && matches_q
        })
        .collect();

    Ok(filtered)
}

#[uniffi::export]
pub async fn get_helpdesk_article(
    requester_user_id: String,
    article_id: String,
) -> Result<HelpdeskArticle, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    seed_helpdesk_articles_if_empty(&conn).await?;

    let art = conn
        .query_row(
            "SELECT id, title, category, summary, content_markdown, tags_json, views_count, created_at \
             FROM helpdesk_articles WHERE id = ?1",
            crate::params![&article_id],
            |row| {
                Ok(HelpdeskArticle {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    category: row.get(2)?,
                    summary: row.get(3)?,
                    content_markdown: row.get(4)?,
                    tags_json: row.get(5)?,
                    views_count: row.get::<i64>(6)? as u32,
                    created_at: row.get(7)?,
                })
            },
        )
        .await?;

    // Increment view count
    let _ = conn
        .execute(
            "UPDATE helpdesk_articles SET views_count = views_count + 1 WHERE id = ?1",
            crate::params![&article_id],
        )
        .await;

    Ok(art)
}

#[uniffi::export]
pub async fn get_product_tour_steps(
    _requester_user_id: String,
    tour_name: String,
) -> Result<Vec<ProductTourStep>, YntraError> {
    let _tour_norm = tour_name.to_lowercase();
    let steps = vec![
        ProductTourStep {
            step_number: 1,
            tour_name: tour_name.clone(),
            target_element_id: "sidebar-navigation".to_string(),
            title: "Welcome to Yntra Platform".to_string(),
            description: "Navigate through workspace modules, team channels, and app features using the interactive sidebar.".to_string(),
            position: "right".to_string(),
        },
        ProductTourStep {
            step_number: 2,
            tour_name: tour_name.clone(),
            target_element_id: "workspace-switcher".to_string(),
            title: "Workspace & Team Identity".to_string(),
            description: "Switch between active organization workspaces, check operational status, and manage team identity.".to_string(),
            position: "bottom".to_string(),
        },
        ProductTourStep {
            step_number: 3,
            tour_name: tour_name.clone(),
            target_element_id: "dispatch-board-nav".to_string(),
            title: "Shift Scheduling & Dispatch Board".to_string(),
            description: "Book employee shifts, manage CalDAV roster calendar feeds, and assign vehicles or equipment.".to_string(),
            position: "bottom".to_string(),
        },
        ProductTourStep {
            step_number: 4,
            tour_name: tour_name.clone(),
            target_element_id: "admin-panel-nav".to_string(),
            title: "Admin Panel & RBAC Security".to_string(),
            description: "Manage team invitation links, configure role-based access control, and inspect Ed25519 audit chains.".to_string(),
            position: "left".to_string(),
        },
        ProductTourStep {
            step_number: 5,
            tour_name: tour_name.clone(),
            target_element_id: "billing-engine-nav".to_string(),
            title: "Billing Engine & Feature Tiers".to_string(),
            description: "Scale allocated seat licenses, configure Stripe / RevenueCat / Chargebee gateways, and download automated invoices.".to_string(),
            position: "top".to_string(),
        },
    ];

    Ok(steps)
}

#[uniffi::export]
pub async fn get_user_tour_progress(
    requester_user_id: String,
    workspace_id: String,
    tour_name: String,
) -> Result<UserTourProgress, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let existing = conn
        .query_row(
            "SELECT workspace_id, user_id, tour_name, current_step, total_steps, completed, updated_at \
             FROM user_tour_progress WHERE workspace_id = ?1 AND user_id = ?2 AND tour_name = ?3",
            crate::params![&workspace_id, &requester_user_id, &tour_name],
            |row| {
                Ok(UserTourProgress {
                    workspace_id: row.get(0)?,
                    user_id: row.get(1)?,
                    tour_name: row.get(2)?,
                    current_step: row.get::<i64>(3)? as u32,
                    total_steps: row.get::<i64>(4)? as u32,
                    completed: row.get::<i64>(5)? != 0,
                    updated_at: row.get(6)?,
                })
            },
        )
        .await
        .ok();

    if let Some(prog) = existing {
        return Ok(prog);
    }

    Ok(UserTourProgress {
        workspace_id,
        user_id: requester_user_id,
        tour_name,
        current_step: 1,
        total_steps: 5,
        completed: false,
        updated_at: crate::infra::time::get_current_time_ms(),
    })
}

#[uniffi::export]
pub async fn complete_product_tour_step(
    requester_user_id: String,
    workspace_id: String,
    tour_name: String,
    step_number: u32,
) -> Result<UserTourProgress, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let now_ms = crate::infra::time::get_current_time_ms();
    let total_steps = 5u32;
    let is_completed = step_number >= total_steps;

    conn.execute(
        "INSERT INTO user_tour_progress (workspace_id, user_id, tour_name, current_step, total_steps, completed, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
         ON CONFLICT(workspace_id, user_id, tour_name) DO UPDATE SET current_step = ?4, completed = ?6, updated_at = ?7",
        crate::params![
            &workspace_id,
            &requester_user_id,
            &tour_name,
            step_number as i64,
            total_steps as i64,
            if is_completed { 1i64 } else { 0i64 },
            now_ms
        ],
    )
    .await?;

    notify_observers();

    Ok(UserTourProgress {
        workspace_id,
        user_id: requester_user_id,
        tour_name,
        current_step: step_number,
        total_steps,
        completed: is_completed,
        updated_at: now_ms,
    })
}

#[uniffi::export]
pub async fn reset_product_tour(
    requester_user_id: String,
    workspace_id: String,
    tour_name: String,
) -> Result<UserTourProgress, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let now_ms = crate::infra::time::get_current_time_ms();

    conn.execute(
        "INSERT INTO user_tour_progress (workspace_id, user_id, tour_name, current_step, total_steps, completed, updated_at) \
         VALUES (?1, ?2, ?3, 1, 5, 0, ?4) \
         ON CONFLICT(workspace_id, user_id, tour_name) DO UPDATE SET current_step = 1, completed = 0, updated_at = ?4",
        crate::params![&workspace_id, &requester_user_id, &tour_name, now_ms],
    )
    .await?;

    notify_observers();

    Ok(UserTourProgress {
        workspace_id,
        user_id: requester_user_id,
        tour_name,
        current_step: 1,
        total_steps: 5,
        completed: false,
        updated_at: now_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_support_guidance_engine_flow() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = crate::database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-supp-{}", uuid::Uuid::new_v4());
        let user_uid = format!("u-supp-{}", uuid::Uuid::new_v4());

        conn.execute(
            "INSERT OR REPLACE INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Support WS', '{}', '{}')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO users (id, workspace_id, email, role) VALUES (?1, ?2, 'user@supp.io', 'member')",
            crate::params![&user_uid, &ws_id],
        )
        .await
        .unwrap();

        crate::services::users::ensure_user_role_signature(&conn, &user_uid, "member", &ws_id)
            .await
            .unwrap();

        // 1. Create support ticket
        let ticket = create_support_ticket(
            user_uid.clone(),
            ws_id.clone(),
            "CalDAV Roster Sync Issue".to_string(),
            "technical".to_string(),
            "high".to_string(),
            "Cannot connect Apple Calendar to sync feed.".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(ticket.subject, "CalDAV Roster Sync Issue");
        assert_eq!(ticket.status, "open");

        // 2. Add follow-up message
        let updated_ticket = add_support_ticket_message(
            user_uid.clone(),
            ws_id.clone(),
            ticket.id.clone(),
            "Additional logs attached for investigation.".to_string(),
        )
        .await
        .unwrap();
        assert!(updated_ticket.messages_json.contains("Additional logs"));

        // 3. Retrieve user tickets
        let tickets = get_user_support_tickets(user_uid.clone(), ws_id.clone())
            .await
            .unwrap();
        assert!(!tickets.is_empty());
        assert_eq!(tickets[0].id, ticket.id);

        // 4. Update ticket status
        let resolved = update_support_ticket_status(
            user_uid.clone(),
            ws_id.clone(),
            ticket.id,
            "resolved".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(resolved.status, "resolved");

        // 5. Search helpdesk articles
        let articles = search_helpdesk_articles(
            user_uid.clone(),
            "onboarding".to_string(),
            "all".to_string(),
        )
        .await
        .unwrap();
        assert!(!articles.is_empty());
        assert!(articles[0].title.contains("Onboarding"));

        // 6. Test Product Tour steps and progress
        let steps = get_product_tour_steps(user_uid.clone(), "onboarding_tour".to_string())
            .await
            .unwrap();
        assert_eq!(steps.len(), 5);

        let prog1 = get_user_tour_progress(
            user_uid.clone(),
            ws_id.clone(),
            "onboarding_tour".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(prog1.current_step, 1);
        assert!(!prog1.completed);

        let prog5 = complete_product_tour_step(
            user_uid.clone(),
            ws_id.clone(),
            "onboarding_tour".to_string(),
            5,
        )
        .await
        .unwrap();
        assert!(prog5.completed);

        let reset_prog = reset_product_tour(
            user_uid.clone(),
            ws_id.clone(),
            "onboarding_tour".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(reset_prog.current_step, 1);
        assert!(!reset_prog.completed);
    }
}
