use super::super::DbConnection;
use crate::YntraError;

const MOCK_DATA_JSON: &str = include_str!("seeds.json");

pub async fn seed_mock_data(conn: &DbConnection) -> Result<(), YntraError> {
    conn.begin_transaction().await?;

    let res = seed_mock_data_impl(conn).await;
    if res.is_ok() {
        conn.commit().await?;
    } else {
        let _ = conn.rollback().await;
    }

    res
}

#[cfg(not(target_arch = "wasm32"))]
fn json_to_libsql_value(v: &serde_json::Value) -> libsql::Value {
    match v {
        serde_json::Value::Null => libsql::Value::Null,
        serde_json::Value::Bool(b) => libsql::Value::Integer(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                libsql::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                libsql::Value::Real(f)
            } else {
                libsql::Value::Null
            }
        }
        serde_json::Value::String(s) => libsql::Value::Text(s.clone()),
        serde_json::Value::Array(a) => libsql::Value::Text(serde_json::to_string(a).unwrap_or_default()),
        serde_json::Value::Object(o) => libsql::Value::Text(serde_json::to_string(o).unwrap_or_default()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn map_to_libsql_params(map: serde_json::Map<String, serde_json::Value>) -> libsql::params::Params {
    let list: Vec<(String, libsql::Value)> = map
        .into_iter()
        .map(|(k, v)| (k, json_to_libsql_value(&v)))
        .collect();
    libsql::params::Params::Named(list)
}

async fn seed_table(
    conn: &DbConnection,
    insert_op: &str,
    table: &str,
    columns: &[&str],
    records: &[serde_json::Value],
) -> Result<(), YntraError> {
    if records.is_empty() {
        return Ok(());
    }

    let cols_str = columns.join(", ");
    let vals_str = columns.iter().map(|col| format!(":{}", col)).collect::<Vec<_>>().join(", ");
    let sql = format!("{} INTO {} ({}) VALUES ({})", insert_op, table, cols_str, vals_str);

    for record in records {
        let mut map = serde_json::Map::new();
        for col in columns {
            let val = match record.get(*col) {
                Some(serde_json::Value::Object(o)) => serde_json::Value::String(serde_json::to_string(o).unwrap_or_default()),
                Some(serde_json::Value::Array(a)) => serde_json::Value::String(serde_json::to_string(a).unwrap_or_default()),
                Some(v) => v.clone(),
                None => serde_json::Value::Null,
            };
            map.insert(format!(":{}", col), val);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let params = map_to_libsql_params(map);
            conn.execute(&sql, params).await?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            conn.execute(&sql, serde_json::Value::Object(map)).await?;
        }
    }
    Ok(())
}

async fn seed_mock_data_impl(conn: &DbConnection) -> Result<(), YntraError> {
    let data: serde_json::Value = serde_json::from_str(MOCK_DATA_JSON)
        .map_err(|e| YntraError::SerializationError(e.to_string()))?;

    // 1. Seed Blocks (Static Metadata)
    conn.execute("DELETE FROM blocks WHERE id = 'school'", ()).await?;
    if let Some(blocks) = data["blocks"].as_array() {
        seed_table(
            conn,
            "INSERT OR IGNORE",
            "blocks",
            &[
                "id",
                "name",
                "description",
                "icon",
                "category",
                "dependencies",
                "created_at",
                "fields_schema",
                "navigation_items",
                "ui_config",
            ],
            blocks,
        ).await?;
    }

    // 2. Check if workspaces already exist
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM workspaces", (), |row| row.get(0))
        .await
        .unwrap_or(0);

    if count == 0 {
        // Seed workspaces
        if let Some(workspaces) = data["workspaces"].as_array() {
            seed_table(
                conn,
                "INSERT",
                "workspaces",
                &["id", "name", "brand_color", "logo_url", "block_settings", "modules_active", "settings"],
                workspaces,
            ).await?;
        }

        // Seed Users
        if let Some(users) = data["users"].as_array() {
            seed_table(
                conn,
                "INSERT OR IGNORE",
                "users",
                &["id", "workspace_id", "email", "full_name", "role", "preferences", "updated_at", "sync_status"],
                users,
            ).await?;

            // Generate role signatures for all seeded users so they pass cryptographic validation
            let mut cached_pk = None;
            let mut cached_sk = None;
            for u in users {
                if let (Some(u_id), Some(u_role)) = (u["id"].as_str(), u["role"].as_str()) {
                    let u_ws = u["workspace_id"].as_str().unwrap_or("workspace-1");
                    let _ = crate::services::users::ensure_user_role_signature_impl(
                        conn,
                        u_id,
                        u_role,
                        u_ws,
                        &mut cached_pk,
                        &mut cached_sk,
                    )
                    .await;
                }
            }
        }

        // Seed cryptographic credentials for users
        let alice_pub = const_hex::encode(ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]).verifying_key().to_bytes());
        let bob_pub = const_hex::encode(ed25519_dalek::SigningKey::from_bytes(&[2u8; 32]).verifying_key().to_bytes());

        conn.execute(
            "UPDATE users SET siths_card_id = 'SITHS-ALICE-123', siths_public_key = :pub_key, nfc_badge_uid = 'NFC-ALICE-999' WHERE id = 'user-1'",
            crate::named_params![":pub_key" => alice_pub],
        ).await?;

        conn.execute(
            "UPDATE users SET siths_card_id = 'SITHS-BOB-456', siths_public_key = :pub_key, nfc_badge_uid = 'NFC-BOB-888' WHERE id = 'user-2'",
            crate::named_params![":pub_key" => bob_pub],
        ).await?;

        // Seed Teams
        if let Some(teams) = data["teams"].as_array() {
            seed_table(
                conn,
                "INSERT OR IGNORE",
                "teams",
                &["id", "workspace_id", "name", "updated_at", "sync_status"],
                teams,
            ).await?;
        }

        // Seed Team Members
        if let Some(team_members) = data["team_members"].as_array() {
            seed_table(
                conn,
                "INSERT OR IGNORE",
                "team_members",
                &["team_id", "user_id", "workspace_id", "updated_at", "sync_status"],
                team_members,
            ).await?;
        }

        // Seed Invitations
        if let Some(invitations) = data["invitations"].as_array() {
            seed_table(
                conn,
                "INSERT OR IGNORE",
                "invitations",
                &["code", "workspace_id", "email", "full_name", "role", "activated", "siths_card_id", "nfc_badge_uid"],
                invitations,
            ).await?;
        }

        // Seed Jobs (Move / Logistics Module)
        let jobs_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM job_tickets", (), |row| row.get(0))
            .await
            .unwrap_or(0);

        if jobs_count == 0 {
            if let Some(job_tickets) = data["job_tickets"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "job_tickets",
                    &[
                        "id", "workspace_id", "title", "description", "location_address", "priority", "status",
                        "assigned_user_id", "scheduled_date", "checklist_json", "completion_report",
                        "created_at", "updated_at", "sync_status",
                        "origin_address", "destination_address", "origin_floor", "destination_floor",
                        "origin_has_elevator", "destination_has_elevator",
                        "origin_parking_permit_needed", "destination_parking_permit_needed"
                    ],
                    job_tickets,
                ).await?;
            }

            // Seed inventory
            if let Some(move_inventory) = data["move_inventory"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "move_inventory",
                    &["id", "job_ticket_id", "item_category", "item_name", "quantity", "estimated_volume_m3", "handling_notes"],
                    move_inventory,
                ).await?;
            }

            // Seed quotes
            if let Some(move_quotes) = data["move_quotes"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "move_quotes",
                    &["id", "job_ticket_id", "base_price", "distance_fee", "stairs_surcharge", "packing_supplies_fee", "total_price", "status", "accepted_at"],
                    move_quotes,
                ).await?;
            }
        }

        // Seed Students (Education Module)
        let students_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM student_profiles", (), |row| row.get(0))
            .await
            .unwrap_or(0);

        if students_count == 0 {
            // Seed student profiles
            if let Some(student_profiles) = data["student_profiles"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "student_profiles",
                    &["id", "workspace_id", "first_name", "last_name", "grade_level", "parent_contact", "updated_at", "sync_status"],
                    student_profiles,
                ).await?;
            }

            // Seed courses
            if let Some(courses) = data["courses"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "courses",
                    &["id", "workspace_id", "name", "subject", "teacher_id", "classroom", "updated_at", "sync_status"],
                    courses,
                ).await?;
            }

            // Seed assignments
            if let Some(assignments) = data["assignments"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "assignments",
                    &["id", "workspace_id", "course_id", "title", "description", "due_date", "max_points", "updated_at", "sync_status"],
                    assignments,
                ).await?;
            }

            // Seed submissions
            if let Some(submissions) = data["submissions"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "submissions",
                    &["id", "workspace_id", "assignment_id", "student_id", "content", "grade", "feedback", "submitted_at", "updated_at", "sync_status"],
                    submissions,
                ).await?;
            }

            // Seed attendance
            if let Some(attendance_records) = data["attendance_records"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "attendance_records",
                    &["id", "workspace_id", "student_id", "course_id", "date", "status", "notes", "updated_at", "sync_status"],
                    attendance_records,
                ).await?;
            }

            // Seed term grades
            if let Some(term_grades) = data["term_grades"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "term_grades",
                    &["id", "workspace_id", "student_id", "course_id", "term_name", "final_grade", "final_points", "teacher_comments", "updated_at", "sync_status"],
                    term_grades,
                ).await?;
            }

            // Seed report cards
            if let Some(report_cards) = data["report_cards"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "report_cards",
                    &["id", "workspace_id", "student_id", "term_name", "gpa", "principal_comments", "status", "updated_at", "sync_status"],
                    report_cards,
                ).await?;
            }

            // Seed health records
            if let Some(health_records) = data["health_records"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "health_records",
                    &["id", "workspace_id", "student_id", "vaccine_name", "status", "administered_at", "updated_at", "sync_status"],
                    health_records,
                ).await?;
            }

            // Seed health incidents
            if let Some(health_incidents) = data["health_incidents"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "health_incidents",
                    &["id", "workspace_id", "student_id", "visit_reason", "treatment", "checked_in_at", "checked_out_at", "notes", "updated_at", "sync_status"],
                    health_incidents,
                ).await?;
            }

            // Seed school invoices
            if let Some(school_invoices) = data["school_invoices"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "school_invoices",
                    &["id", "workspace_id", "student_id", "title", "amount", "due_date", "status", "paid_at", "updated_at", "sync_status"],
                    school_invoices,
                ).await?;
            }

            // Seed school payments
            if let Some(school_payments) = data["school_payments"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "school_payments",
                    &["id", "workspace_id", "invoice_id", "amount", "payment_method", "paid_at", "updated_at", "sync_status"],
                    school_payments,
                ).await?;
            }

            // Seed library books
            if let Some(library_books) = data["library_books"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "library_books",
                    &["id", "workspace_id", "title", "author", "isbn", "copies_available", "total_copies", "updated_at", "sync_status"],
                    library_books,
                ).await?;
            }

            // Seed library lending logs
            if let Some(library_lending_logs) = data["library_lending_logs"].as_array() {
                seed_table(
                    conn,
                    "INSERT",
                    "library_lending_logs",
                    &["id", "workspace_id", "book_id", "student_id", "checked_out_at", "due_date", "returned_at", "status", "updated_at", "sync_status"],
                    library_lending_logs,
                ).await?;
            }
        }
    }

    // 3. Env-based Platform Admin setup
    let admin_email = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut email = std::env::var("ADMIN_EMAIL").unwrap_or_default();
            if email.is_empty() {
                if let Ok(content) = std::fs::read_to_string("admin_email.txt") {
                    email = content.trim().to_string();
                } else if let Ok(content) = std::fs::read_to_string("../admin_email.txt") {
                    email = content.trim().to_string();
                }
            }
            if email.is_empty() {
                for path in &[".env", "../.env"] {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        for line in content.lines() {
                            if let Some(stripped) = line.strip_prefix("ADMIN_EMAIL=") {
                                email = stripped.trim().trim_matches('"').trim_matches('\'').to_string();
                                break;
                            }
                        }
                        if !email.is_empty() {
                            break;
                        }
                    }
                }
            }
            email
        }
        #[cfg(target_arch = "wasm32")]
        {
            "".to_string()
        }
    };

    if !admin_email.is_empty() {
        conn.execute(
            "INSERT OR IGNORE INTO users (id, workspace_id, email, full_name, role, preferences) VALUES ('user-env-admin', 'workspace-1', :email, 'Admin User', 'platform_admin', '{}')",
            crate::named_params![":email" => &admin_email],
        ).await?;
        conn.execute(
            "UPDATE users SET role = 'platform_admin' WHERE email = :email",
            crate::named_params![":email" => &admin_email],
        ).await?;
    }

    Ok(())
}
