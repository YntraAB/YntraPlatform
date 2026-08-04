use crate::database;
use crate::infra::errors::YntraError;
use crate::infra::observer::notify_observers;

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct SemanticConflictRecord {
    pub id: String,
    pub workspace_id: String,
    pub domain: String,
    pub entity_table: String,
    pub entity_id: String,
    pub colliding_entity_id: Option<String>,
    pub conflict_type: String,
    pub severity: String,
    pub conflict_details_json: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct PreCommitConflictResult {
    pub can_commit_immediately: bool,
    pub is_lease_held_by_peer: bool,
    pub peer_holder_id: Option<String>,
    pub peer_holder_name: Option<String>,
    pub requires_tentative_badge: bool,
    pub predicted_conflict_type: Option<String>,
    pub warning_message: Option<String>,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct IntentLeaseRecord {
    pub id: String,
    pub workspace_id: String,
    pub entity_table: String,
    pub entity_id: String,
    pub user_id: String,
    pub user_name: String,
    pub expires_at_ms: i64,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct ResolutionPolicyConfig {
    pub workspace_id: String,
    pub default_strategy: String, // "higher_role_wins", "earliest_timestamp_wins", "demote_to_draft", "manual_review"
    pub auto_resolve_high_severity: bool,
}


/// Run post-merge semantic guardrails check over a workspace's data tables without authorization checks (for system background sync tasks).
pub async fn trigger_post_sync_guardrails(workspace_id: &str) -> Result<u32, YntraError> {
    let conn = database::acquire_connection().await?;
    let newly_flagged = execute_guardrail_scan(&conn, workspace_id).await?;
    if newly_flagged > 0 {
        notify_observers();
    }
    Ok(newly_flagged)
}

/// Run post-merge semantic guardrails check over a workspace's data tables.
/// Returns the number of newly detected logical conflicts flagged for review.
#[uniffi::export]
pub async fn run_semantic_guardrail_check(
    requester_user_id: String,
    workspace_id: String,
) -> Result<u32, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let newly_flagged = execute_guardrail_scan(&conn, &workspace_id).await?;
    if newly_flagged > 0 {
        notify_observers();
    }
    Ok(newly_flagged)
}

pub async fn execute_guardrail_scan(
    conn: &database::DbConnection,
    workspace_id: &str,
) -> Result<u32, YntraError> {
    let mut new_conflicts = 0u32;
    new_conflicts += check_overlapping_events(conn, workspace_id).await?;
    new_conflicts += check_double_booked_vehicles(conn, workspace_id).await?;
    new_conflicts += check_timetable_collisions(conn, workspace_id).await?;
    new_conflicts += check_inventory_overallocation(conn, workspace_id).await?;
    Ok(new_conflicts)
}

async fn check_overlapping_events(
    conn: &database::DbConnection,
    workspace_id: &str,
) -> Result<u32, YntraError> {
    struct EventRow {
        id: String,
        user_id: Option<String>,
        assignee_id: Option<String>,
        title: String,
        start_time: String,
        end_time: String,
    }

    let mut stmt = conn
        .prepare("SELECT id, user_id, assignee_id, title, start_time, end_time FROM events WHERE workspace_id = ?1")
        .await?;
    let mut rows = stmt.query(crate::params![workspace_id]).await?;

    let mut events = Vec::new();
    while let Some(row) = rows.next().await? {
        events.push(EventRow {
            id: row.get(0)?,
            user_id: row.get(1)?,
            assignee_id: row.get(2)?,
            title: row.get(3)?,
            start_time: row.get(4)?,
            end_time: row.get(5)?,
        });
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let mut flagged_count = 0u32;

    for i in 0..events.len() {
        for j in (i + 1)..events.len() {
            let e1 = &events[i];
            let e2 = &events[j];

            let same_user = (e1.user_id.is_some() && e1.user_id == e2.user_id)
                || (e1.assignee_id.is_some() && e1.assignee_id == e2.assignee_id);

            if !same_user {
                continue;
            }

            // Simple ISO string overlap check: start1 < end2 AND start2 < end1
            let overlap = e1.start_time < e2.end_time && e2.start_time < e1.end_time;
            if overlap {
                let conflict_id = format!("semantic-event-{}-{}", e1.id, e2.id);

                let existing: Option<i64> = conn
                    .query_row(
                        "SELECT 1 FROM crdt_semantic_conflicts WHERE id = ?1",
                        crate::params![&conflict_id],
                        |r| r.get(0),
                    )
                    .await
                    .ok();

                if existing.is_none() {
                    let assigned_to = e1
                        .assignee_id
                        .clone()
                        .or_else(|| e1.user_id.clone())
                        .unwrap_or_default();
                    let details_json = serde_json::json!({
                        "assigned_to": assigned_to,
                        "entity1": { "id": e1.id, "title": e1.title, "start": e1.start_time, "end": e1.end_time },
                        "entity2": { "id": e2.id, "title": e2.title, "start": e2.start_time, "end": e2.end_time }
                    })
                    .to_string();

                    conn.execute(
                        "INSERT INTO crdt_semantic_conflicts (id, workspace_id, domain, entity_table, entity_id, colliding_entity_id, conflict_type, severity, conflict_details_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        crate::params![
                            &conflict_id,
                            workspace_id,
                            "calendar",
                            "events",
                            &e1.id,
                            &e2.id,
                            "OVERLAPPING_SCHEDULE",
                            "high",
                            &details_json,
                            "flagged_for_review",
                            now_ms,
                            now_ms
                        ],
                    )
                    .await?;

                    flagged_count += 1;
                }
            }
        }
    }

    Ok(flagged_count)
}

async fn check_double_booked_vehicles(
    conn: &database::DbConnection,
    workspace_id: &str,
) -> Result<u32, YntraError> {
    struct TicketRow {
        id: String,
        title: String,
        assigned_vehicle_id: String,
        scheduled_date: String,
    }

    let mut stmt = conn
        .prepare("SELECT id, title, assigned_vehicle_id, COALESCE(scheduled_date, '') FROM job_tickets WHERE workspace_id = ?1 AND assigned_vehicle_id IS NOT NULL AND status != 'cancelled'")
        .await?;
    let mut rows = stmt.query(crate::params![workspace_id]).await?;

    let mut tickets = Vec::new();
    while let Some(row) = rows.next().await? {
        let vehicle_id: String = row.get(2)?;
        if !vehicle_id.is_empty() {
            tickets.push(TicketRow {
                id: row.get(0)?,
                title: row.get(1)?,
                assigned_vehicle_id: vehicle_id,
                scheduled_date: row.get(3)?,
            });
        }
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let mut flagged_count = 0u32;

    for i in 0..tickets.len() {
        for j in (i + 1)..tickets.len() {
            let t1 = &tickets[i];
            let t2 = &tickets[j];

            if t1.assigned_vehicle_id != t2.assigned_vehicle_id {
                continue;
            }

            let overlap = if !t1.scheduled_date.is_empty() && !t2.scheduled_date.is_empty() {
                t1.scheduled_date == t2.scheduled_date
            } else {
                true // Same vehicle assigned simultaneously to active tickets
            };

            if overlap {
                let conflict_id = format!("semantic-vehicle-{}-{}", t1.id, t2.id);

                let existing: Option<i64> = conn
                    .query_row(
                        "SELECT 1 FROM crdt_semantic_conflicts WHERE id = ?1",
                        crate::params![&conflict_id],
                        |r| r.get(0),
                    )
                    .await
                    .ok();

                if existing.is_none() {
                    let details_json = serde_json::json!({
                        "vehicle_id": t1.assigned_vehicle_id,
                        "ticket1": { "id": t1.id, "title": t1.title, "scheduled_date": t1.scheduled_date },
                        "ticket2": { "id": t2.id, "title": t2.title, "scheduled_date": t2.scheduled_date }
                    })
                    .to_string();

                    conn.execute(
                        "INSERT INTO crdt_semantic_conflicts (id, workspace_id, domain, entity_table, entity_id, colliding_entity_id, conflict_type, severity, conflict_details_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        crate::params![
                            &conflict_id,
                            workspace_id,
                            "dispatch",
                            "job_tickets",
                            &t1.id,
                            &t2.id,
                            "DOUBLE_BOOKED_RESOURCE",
                            "high",
                            &details_json,
                            "flagged_for_review",
                            now_ms,
                            now_ms
                        ],
                    )
                    .await?;

                    flagged_count += 1;
                }
            }
        }
    }

    Ok(flagged_count)
}

async fn check_timetable_collisions(
    conn: &database::DbConnection,
    workspace_id: &str,
) -> Result<u32, YntraError> {
    struct SlotRow {
        id: String,
        course_id: String,
        day_of_week: i64,
        start_time: String,
        end_time: String,
        classroom: Option<String>,
    }

    let mut stmt = conn
        .prepare("SELECT id, course_id, day_of_week, start_time, end_time, classroom FROM timetable_slots WHERE workspace_id = ?1")
        .await?;
    let mut rows = stmt.query(crate::params![workspace_id]).await?;

    let mut slots = Vec::new();
    while let Some(row) = rows.next().await? {
        slots.push(SlotRow {
            id: row.get(0)?,
            course_id: row.get(1)?,
            day_of_week: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            classroom: row.get(5)?,
        });
    }

    let now_ms = crate::infra::time::get_current_time_ms();
    let mut flagged_count = 0u32;

    for i in 0..slots.len() {
        for j in (i + 1)..slots.len() {
            let s1 = &slots[i];
            let s2 = &slots[j];

            if s1.day_of_week != s2.day_of_week {
                continue;
            }

            let same_classroom = s1.classroom.is_some() && s1.classroom == s2.classroom;
            if same_classroom {
                let overlap = s1.start_time < s2.end_time && s2.start_time < s1.end_time;
                if overlap {
                    let conflict_id = format!("semantic-timetable-{}-{}", s1.id, s2.id);

                    let existing: Option<i64> = conn
                        .query_row(
                            "SELECT 1 FROM crdt_semantic_conflicts WHERE id = ?1",
                            crate::params![&conflict_id],
                            |r| r.get(0),
                        )
                        .await
                        .ok();

                    if existing.is_none() {
                        let details_json = serde_json::json!({
                            "classroom": s1.classroom,
                            "day_of_week": s1.day_of_week,
                            "slot1": { "id": s1.id, "course_id": s1.course_id, "start": s1.start_time, "end": s1.end_time },
                            "slot2": { "id": s2.id, "course_id": s2.course_id, "start": s2.start_time, "end": s2.end_time }
                        })
                        .to_string();

                        conn.execute(
                            "INSERT INTO crdt_semantic_conflicts (id, workspace_id, domain, entity_table, entity_id, colliding_entity_id, conflict_type, severity, conflict_details_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                            crate::params![
                                &conflict_id,
                                workspace_id,
                                "academics",
                                "timetable_slots",
                                &s1.id,
                                &s2.id,
                                "CLASSROOM_OVERLAP",
                                "medium",
                                &details_json,
                                "flagged_for_review",
                                now_ms,
                                now_ms
                            ],
                        )
                        .await?;

                        flagged_count += 1;
                    }
                }
            }
        }
    }

    Ok(flagged_count)
}

async fn check_inventory_overallocation(
    conn: &database::DbConnection,
    workspace_id: &str,
) -> Result<u32, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let mut flagged_count = 0u32;

    struct PackagingRow {
        id: String,
        job_ticket_id: String,
        item_name: String,
        quantity: i64,
        returned_quantity: i64,
    }

    let mut stmt = conn
        .prepare("SELECT id, job_ticket_id, item_name, quantity, returned_quantity FROM job_packaging_items WHERE workspace_id = ?1 AND (returned_quantity > quantity OR quantity < 0)")
        .await?;
    let mut rows = stmt.query(crate::params![workspace_id]).await?;

    while let Some(row) = rows.next().await? {
        let pkg = PackagingRow {
            id: row.get(0)?,
            job_ticket_id: row.get(1)?,
            item_name: row.get(2)?,
            quantity: row.get(3)?,
            returned_quantity: row.get(4)?,
        };

        let conflict_id = format!("semantic-pkg-{}", pkg.id);

        let existing: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM crdt_semantic_conflicts WHERE id = ?1",
                crate::params![&conflict_id],
                |r| r.get(0),
            )
            .await
            .ok();

        if existing.is_none() {
            let details_json = serde_json::json!({
                "item_name": pkg.item_name,
                "job_ticket_id": pkg.job_ticket_id,
                "allocated_quantity": pkg.quantity,
                "returned_quantity": pkg.returned_quantity,
                "reason": if pkg.quantity < 0 { "Negative allocated quantity" } else { "Returned quantity exceeds allocated quantity" }
            })
            .to_string();

            conn.execute(
                "INSERT INTO crdt_semantic_conflicts (id, workspace_id, domain, entity_table, entity_id, colliding_entity_id, conflict_type, severity, conflict_details_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                crate::params![
                    &conflict_id,
                    workspace_id,
                    "inventory",
                    "job_packaging_items",
                    &pkg.id,
                    &pkg.job_ticket_id,
                    "INVENTORY_OVERALLOCATION",
                    "high",
                    &details_json,
                    "flagged_for_review",
                    now_ms,
                    now_ms
                ],
            )
            .await?;

            flagged_count += 1;
        }
    }

    struct InventoryRow {
        id: String,
        job_ticket_id: String,
        item_name: String,
        quantity: i64,
    }

    let mut stmt_inv = conn
        .prepare("SELECT id, job_ticket_id, item_name, quantity FROM move_inventory WHERE workspace_id = ?1 AND quantity < 0")
        .await?;
    let mut rows_inv = stmt_inv.query(crate::params![workspace_id]).await?;

    while let Some(row) = rows_inv.next().await? {
        let inv = InventoryRow {
            id: row.get(0)?,
            job_ticket_id: row.get(1)?,
            item_name: row.get(2)?,
            quantity: row.get(3)?,
        };

        let conflict_id = format!("semantic-inv-{}", inv.id);

        let existing: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM crdt_semantic_conflicts WHERE id = ?1",
                crate::params![&conflict_id],
                |r| r.get(0),
            )
            .await
            .ok();

        if existing.is_none() {
            let details_json = serde_json::json!({
                "item_name": inv.item_name,
                "job_ticket_id": inv.job_ticket_id,
                "quantity": inv.quantity,
                "reason": "Negative inventory quantity post-sync merge"
            })
            .to_string();

            conn.execute(
                "INSERT INTO crdt_semantic_conflicts (id, workspace_id, domain, entity_table, entity_id, colliding_entity_id, conflict_type, severity, conflict_details_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                crate::params![
                    &conflict_id,
                    workspace_id,
                    "inventory",
                    "move_inventory",
                    &inv.id,
                    &inv.job_ticket_id,
                    "INVENTORY_OVERALLOCATION",
                    "high",
                    &details_json,
                    "flagged_for_review",
                    now_ms,
                    now_ms
                ],
            )
            .await?;

            flagged_count += 1;
        }
    }

    Ok(flagged_count)
}

#[uniffi::export]
pub async fn get_semantic_conflicts(
    requester_user_id: String,
    workspace_id: String,
    status_filter: Option<String>,
) -> Result<Vec<SemanticConflictRecord>, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let filter = status_filter.unwrap_or_else(|| "flagged_for_review".to_string());
    let mut stmt = conn
        .prepare("SELECT id, workspace_id, domain, entity_table, entity_id, colliding_entity_id, conflict_type, severity, conflict_details_json, status, created_at, updated_at FROM crdt_semantic_conflicts WHERE workspace_id = ?1 AND status = ?2 ORDER BY created_at DESC")
        .await?;

    let mut rows = stmt.query(crate::params![&workspace_id, &filter]).await?;
    let mut records = Vec::new();
    while let Some(row) = rows.next().await? {
        records.push(SemanticConflictRecord {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            domain: row.get(2)?,
            entity_table: row.get(3)?,
            entity_id: row.get(4)?,
            colliding_entity_id: row.get(5)?,
            conflict_type: row.get(6)?,
            severity: row.get(7)?,
            conflict_details_json: row.get(8)?,
            status: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        });
    }

    Ok(records)
}

#[uniffi::export]
pub async fn resolve_semantic_conflict(
    requester_user_id: String,
    workspace_id: String,
    conflict_id: String,
    resolution_strategy: String,
    chosen_entity_id: Option<String>,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let now_ms = crate::infra::time::get_current_time_ms();

    let conflict: (String, String, Option<String>) = conn
        .query_row(
            "SELECT entity_table, entity_id, colliding_entity_id FROM crdt_semantic_conflicts WHERE id = ?1 AND workspace_id = ?2",
            crate::params![&conflict_id, &workspace_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .await
        .map_err(|_| YntraError::NotFoundError("Semantic conflict record not found".to_string()))?;

    let (entity_table, entity_id, colliding_id) = conflict;

    if resolution_strategy == "keep_chosen" {
        if let Some(keep_id) = chosen_entity_id {
            let cancel_id = if keep_id == entity_id {
                colliding_id
            } else {
                Some(entity_id)
            };

            if let Some(cid) = cancel_id {
                match entity_table.as_str() {
                    "events" => {
                        conn.execute("DELETE FROM events WHERE id = ?1", crate::params![&cid])
                            .await?;
                    }
                    "job_tickets" => {
                        conn.execute(
                            "UPDATE job_tickets SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
                            crate::params![now_ms, &cid],
                        )
                        .await?;
                    }
                    "timetable_slots" => {
                        conn.execute(
                            "DELETE FROM timetable_slots WHERE id = ?1",
                            crate::params![&cid],
                        )
                        .await?;
                    }
                    _ => {}
                }
            }
        }
    }

    conn.execute(
        "UPDATE crdt_semantic_conflicts SET status = 'resolved', updated_at = ?1 WHERE id = ?2",
        crate::params![now_ms, &conflict_id],
    )
    .await?;

    notify_observers();
    Ok(())
}

#[uniffi::export]
pub async fn dismiss_semantic_conflict(
    requester_user_id: String,
    workspace_id: String,
    conflict_id: String,
) -> Result<(), YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let now_ms = crate::infra::time::get_current_time_ms();
    conn.execute(
        "UPDATE crdt_semantic_conflicts SET status = 'dismissed', updated_at = ?1 WHERE id = ?2 AND workspace_id = ?3",
        crate::params![now_ms, &conflict_id, &workspace_id],
    )
    .await?;

    notify_observers();
    Ok(())
}

// In-memory Intent Lease Store for fast P2P and local pre-flight checks
static INTENT_LEASES: std::sync::OnceLock<std::sync::Mutex<Vec<IntentLeaseRecord>>> = std::sync::OnceLock::new();

fn get_leases_lock() -> &'static std::sync::Mutex<Vec<IntentLeaseRecord>> {
    INTENT_LEASES.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

#[uniffi::export]
pub async fn acquire_intent_lease(
    requester_user_id: String,
    workspace_id: String,
    entity_table: String,
    entity_id: String,
    user_name: String,
    lease_duration_sec: Option<u32>,
) -> Result<IntentLeaseRecord, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    let now_ms = crate::infra::time::get_current_time_ms();
    let duration = (lease_duration_sec.unwrap_or(30) as i64) * 1000;
    let expires_at_ms = now_ms + duration;

    let mut lock = get_leases_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Lease lock poisoned".to_string()))?;

    // Clean expired leases
    lock.retain(|l| l.expires_at_ms > now_ms);

    // Check if another user holds an active lease on this entity
    if let Some(existing) = lock.iter().find(|l| {
        l.workspace_id == workspace_id
            && l.entity_table == entity_table
            && l.entity_id == entity_id
            && l.user_id != requester_user_id
    }) {
        return Err(YntraError::ValidationError(format!(
            "Resource is currently being edited by {}",
            existing.user_name
        )));
    }

    // Remove old lease by same user on same entity if present
    lock.retain(|l| !(l.workspace_id == workspace_id && l.entity_table == entity_table && l.entity_id == entity_id && l.user_id == requester_user_id));

    let lease = IntentLeaseRecord {
        id: format!("lease-{}-{}", entity_id, now_ms),
        workspace_id,
        entity_table,
        entity_id,
        user_id: requester_user_id,
        user_name,
        expires_at_ms,
    };

    lock.push(lease.clone());
    notify_observers();
    Ok(lease)
}

#[uniffi::export]
pub async fn release_intent_lease(
    requester_user_id: String,
    workspace_id: String,
    entity_id: String,
) -> Result<(), YntraError> {
    let mut lock = get_leases_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Lease lock poisoned".to_string()))?;

    lock.retain(|l| !(l.workspace_id == workspace_id && l.entity_id == entity_id && l.user_id == requester_user_id));
    notify_observers();
    Ok(())
}

#[uniffi::export]
pub fn get_active_intent_leases(workspace_id: String) -> Result<Vec<IntentLeaseRecord>, YntraError> {
    let now_ms = crate::infra::time::get_current_time_ms();
    let lock = get_leases_lock()
        .lock()
        .map_err(|_| YntraError::CryptoError("Lease lock poisoned".to_string()))?;

    let active: Vec<IntentLeaseRecord> = lock
        .iter()
        .filter(|l| l.workspace_id == workspace_id && l.expires_at_ms > now_ms)
        .cloned()
        .collect();

    Ok(active)
}

#[uniffi::export]
pub async fn preflight_check_intent(
    requester_user_id: String,
    workspace_id: String,
    entity_table: String,
    entity_id: String,
    proposed_start_time: Option<String>,
    proposed_end_time: Option<String>,
    proposed_assigned_vehicle_id: Option<String>,
) -> Result<PreCommitConflictResult, YntraError> {
    let conn = database::acquire_connection().await?;
    let _auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;

    // 1. Check intent leases
    let leases = get_active_intent_leases(workspace_id.clone())?;
    if let Some(held) = leases.iter().find(|l| l.entity_table == entity_table && l.entity_id == entity_id && l.user_id != requester_user_id) {
        return Ok(PreCommitConflictResult {
            can_commit_immediately: false,
            is_lease_held_by_peer: true,
            peer_holder_id: Some(held.user_id.clone()),
            peer_holder_name: Some(held.user_name.clone()),
            requires_tentative_badge: true,
            predicted_conflict_type: Some("ACTIVE_PEER_LEASE".to_string()),
            warning_message: Some(format!("Peer {} is actively editing this item.", held.user_name)),
        });
    }

    // 2. Check local database collisions for proposed schedule
    if let (Some(start), Some(end)) = (proposed_start_time, proposed_end_time) {
        if entity_table == "events" {
            let mut stmt = conn
                .prepare("SELECT id, title, start_time, end_time FROM events WHERE workspace_id = ?1 AND user_id = ?2 AND id != ?3")
                .await?;
            let mut rows = stmt.query(crate::params![&workspace_id, &requester_user_id, &entity_id]).await?;

            while let Some(row) = rows.next().await? {
                let s: String = row.get(2)?;
                let e: String = row.get(3)?;
                if start < e && s < end {
                    let title: String = row.get(1)?;
                    return Ok(PreCommitConflictResult {
                        can_commit_immediately: true,
                        is_lease_held_by_peer: false,
                        peer_holder_id: None,
                        peer_holder_name: None,
                        requires_tentative_badge: true,
                        predicted_conflict_type: Some("OVERLAPPING_SCHEDULE".to_string()),
                        warning_message: Some(format!("Overlaps with existing meeting: '{}'", title)),
                    });
                }
            }
        }
    }

    if let Some(vehicle_id) = proposed_assigned_vehicle_id {
        if !vehicle_id.is_empty() {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM job_tickets WHERE workspace_id = ?1 AND assigned_vehicle_id = ?2 AND id != ?3 AND status != 'cancelled'",
                    crate::params![&workspace_id, &vehicle_id, &entity_id],
                    |r| r.get(0),
                )
                .await
                .unwrap_or(0);

            if count > 0 {
                return Ok(PreCommitConflictResult {
                    can_commit_immediately: true,
                    is_lease_held_by_peer: false,
                    peer_holder_id: None,
                    peer_holder_name: None,
                    requires_tentative_badge: true,
                    predicted_conflict_type: Some("DOUBLE_BOOKED_RESOURCE".to_string()),
                    warning_message: Some(format!("Vehicle {} is already assigned to an active ticket.", vehicle_id)),
                });
            }
        }
    }

    Ok(PreCommitConflictResult {
        can_commit_immediately: true,
        is_lease_held_by_peer: false,
        peer_holder_id: None,
        peer_holder_name: None,
        requires_tentative_badge: false,
        predicted_conflict_type: None,
        warning_message: None,
    })
}

#[uniffi::export]
pub async fn auto_resolve_semantic_conflicts(
    requester_user_id: String,
    workspace_id: String,
    strategy: Option<String>,
) -> Result<u32, YntraError> {
    let conn = database::acquire_connection().await?;
    let auth = crate::AuthContext::authorize(&conn, &requester_user_id).await?;
    if !auth.is_admin {
        return Err(YntraError::AuthError("Admin privileges required for auto-resolution".to_string()));
    }

    let policy_strategy = strategy.unwrap_or_else(|| "earliest_timestamp_wins".to_string());
    let conflicts = get_semantic_conflicts(requester_user_id.clone(), workspace_id.clone(), Some("flagged_for_review".to_string())).await?;

    let mut auto_resolved = 0u32;

    for conflict in conflicts {
        if policy_strategy == "earliest_timestamp_wins" || policy_strategy == "higher_role_wins" {
            // Keep original entity_id, cancel colliding_entity_id
            resolve_semantic_conflict(
                requester_user_id.clone(),
                workspace_id.clone(),
                conflict.id,
                "keep_chosen".to_string(),
                Some(conflict.entity_id),
            )
            .await?;
            auto_resolved += 1;
        } else if policy_strategy == "dismiss_all" {
            dismiss_semantic_conflict(requester_user_id.clone(), workspace_id.clone(), conflict.id).await?;
            auto_resolved += 1;
        }
    }

    Ok(auto_resolved)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_semantic_guardrail_overlapping_events() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-guardrail-{}", crate::infra::time::get_current_time_ms());
        conn.execute(
            "INSERT INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Test WS', '[]', '{}')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        // Insert two overlapping events for user-1
        conn.execute(
            "INSERT INTO events (id, workspace_id, user_id, title, start_time, end_time) VALUES (?1, ?2, 'user-1', 'Meeting A', '2026-08-04T10:00:00Z', '2026-08-04T11:30:00Z')",
            crate::params!["evt-1", &ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO events (id, workspace_id, user_id, title, start_time, end_time) VALUES (?1, ?2, 'user-1', 'Meeting B', '2026-08-04T11:00:00Z', '2026-08-04T12:00:00Z')",
            crate::params!["evt-2", &ws_id],
        )
        .await
        .unwrap();

        let flagged = execute_guardrail_scan(&conn, &ws_id).await.unwrap();
        assert_eq!(flagged, 1, "Should flag 1 overlapping event conflict");

        let conflicts = get_semantic_conflicts("user-1".to_string(), ws_id.clone(), None)
            .await
            .unwrap_or_default();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, "OVERLAPPING_SCHEDULE");

        // Resolve conflict by keeping evt-1
        resolve_semantic_conflict(
            "user-1".to_string(),
            ws_id.clone(),
            conflicts[0].id.clone(),
            "keep_chosen".to_string(),
            Some("evt-1".to_string()),
        )
        .await
        .unwrap();

        let active_conflicts = get_semantic_conflicts("user-1".to_string(), ws_id.clone(), None)
            .await
            .unwrap_or_default();
        assert_eq!(
            active_conflicts.len(),
            0,
            "No pending conflicts should remain"
        );
    }

    #[tokio::test]
    async fn test_intent_lease_and_preflight_check() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-lease-{}", crate::infra::time::get_current_time_ms());
        conn.execute(
            "INSERT INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Lease WS', '[]', '{}')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO users (id, workspace_id, email, role) VALUES ('u-user1', ?1, 'user1@yntra.se', 'user')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO users (id, workspace_id, email, role) VALUES ('u-user2', ?1, 'user2@yntra.se', 'user')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        // 1. User 1 acquires lease
        let lease1 = acquire_intent_lease(
            "u-user1".to_string(),
            ws_id.clone(),
            "events".to_string(),
            "evt-target".to_string(),
            "Alice".to_string(),
            Some(30),
        )
        .await
        .unwrap();
        assert_eq!(lease1.user_name, "Alice");

        // 2. Pre-flight check by User 2 returns lease warning
        let check_res = preflight_check_intent(
            "u-user2".to_string(),
            ws_id.clone(),
            "events".to_string(),
            "evt-target".to_string(),
            None,
            None,
            None,
        )
        .await
        .unwrap();

        assert!(!check_res.can_commit_immediately);
        assert!(check_res.is_lease_held_by_peer);
        assert_eq!(check_res.peer_holder_name.unwrap(), "Alice");
        assert!(check_res.requires_tentative_badge);

        // 3. User 1 releases lease
        release_intent_lease("u-user1".to_string(), ws_id.clone(), "evt-target".to_string())
            .await
            .unwrap();

        let check_res2 = preflight_check_intent(
            "u-user2".to_string(),
            ws_id.clone(),
            "events".to_string(),
            "evt-target".to_string(),
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert!(check_res2.can_commit_immediately);
        assert!(!check_res2.is_lease_held_by_peer);
    }

    #[tokio::test]
    async fn test_auto_resolution_policy() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-autores-{}", crate::infra::time::get_current_time_ms());
        conn.execute(
            "INSERT INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'AutoRes WS', '[]', '{}')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO users (id, workspace_id, email, role) VALUES ('u-admin-autores', ?1, 'admin@autores.se', 'platform_admin')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        // Insert overlapping events
        conn.execute(
            "INSERT INTO events (id, workspace_id, user_id, title, start_time, end_time) VALUES ('evt-autores-1', ?1, 'u-admin-autores', 'Shift 1', '2026-08-04T10:00:00Z', '2026-08-04T11:30:00Z')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO events (id, workspace_id, user_id, title, start_time, end_time) VALUES ('evt-autores-2', ?1, 'u-admin-autores', 'Shift 2', '2026-08-04T11:00:00Z', '2026-08-04T12:00:00Z')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        let flagged = execute_guardrail_scan(&conn, &ws_id).await.unwrap();
        assert_eq!(flagged, 1);

        // Auto resolve using default policy
        let resolved_count = auto_resolve_semantic_conflicts(
            "u-admin-autores".to_string(),
            ws_id.clone(),
            Some("earliest_timestamp_wins".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(resolved_count, 1);

        let active_conflicts = get_semantic_conflicts("u-admin-autores".to_string(), ws_id, None)
            .await
            .unwrap_or_default();
        assert_eq!(active_conflicts.len(), 0);
    }

    #[tokio::test]
    async fn test_post_sync_inventory_guardrail_detection() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();
        let conn = database::acquire_connection().await.unwrap();

        let ws_id = format!("ws-inv-{}", crate::infra::time::get_current_time_ms());
        conn.execute(
            "INSERT INTO workspaces (id, name, modules_active, settings) VALUES (?1, 'Inventory WS', '[]', '{}')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO users (id, workspace_id, email, role) VALUES ('u-inv-admin', ?1, 'admin@inv.se', 'platform_admin')",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO job_tickets (id, workspace_id, title, description, location_address, priority, status, scheduled_date, checklist_json, created_at, updated_at) VALUES ('jt-1', ?1, 'Job Ticket 1', 'Test Job', 'Site A', 'high', 'active', '2026-08-05', '[]', '2026-08-05T00:00:00Z', 1700000000)",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        // Insert overallocated packaging item (returned_quantity 15 > quantity 10)
        conn.execute(
            "INSERT INTO job_packaging_items (id, workspace_id, job_ticket_id, item_name, quantity, price_per_unit, returned_quantity) VALUES ('pkg-bad-1', ?1, 'jt-1', 'Cardboard Boxes', 10, 5.0, 15)",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        // Insert move_inventory item with negative quantity
        conn.execute(
            "INSERT INTO move_inventory (id, workspace_id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3) VALUES ('inv-neg-1', ?1, 'jt-1', 'Furniture', 'Pallets', -3, 1.2)",
            crate::params![&ws_id],
        )
        .await
        .unwrap();

        let flagged = trigger_post_sync_guardrails(&ws_id).await.unwrap();
        assert_eq!(flagged, 2, "Expected 2 inventory overallocation conflicts to be flagged");

        let conflicts = get_semantic_conflicts("u-inv-admin".to_string(), ws_id, None)
            .await
            .unwrap_or_default();
        assert_eq!(conflicts.len(), 2);
        assert_eq!(conflicts[0].conflict_type, "INVENTORY_OVERALLOCATION");
        assert_eq!(conflicts[0].domain, "inventory");
    }
}

