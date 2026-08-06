use super::super::DbConnection;
use crate::YntraError;

async fn execute_migration_sql(conn: &DbConnection, sql: &str) -> Result<(), YntraError> {
    match conn.execute(sql, ()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("duplicate column name")
                || err_str.contains("already exists")
                || err_str.contains("duplicate column")
            {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

async fn execute_migration_batch(conn: &DbConnection, sql: &str) -> Result<(), YntraError> {
    match conn.execute_batch(sql).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("duplicate column name")
                || err_str.contains("already exists")
                || err_str.contains("duplicate column")
                || err_str.contains("duplicate table")
            {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

pub async fn run_schema_migrations(
    conn: &DbConnection,
    current_version: i32,
) -> Result<i32, YntraError> {
    let mut version = current_version;
    if version < 2 {
        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN siths_card_id TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN nfc_badge_uid TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN personal_number TEXT").await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE invitations ADD COLUMN siths_card_id TEXT",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE invitations ADD COLUMN nfc_badge_uid TEXT",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE todos ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE todos ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE todos ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE users ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE users ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE workspaces ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE workspaces ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE teams ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE teams ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE team_members ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE team_members ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE team_members ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE events ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE events ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE messages ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE messages ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE notes ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE notes ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE time_reports ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE time_reports ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE clients ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE clients ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(conn, "ALTER TABLE client_medications ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'").await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE client_medications ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE client_medications ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(conn, "ALTER TABLE client_journals ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'").await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE client_journals ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE client_journals ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE reports ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE reports ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE invitations ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE invitations ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;

        execute_migration_sql(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN origin_address TEXT",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN destination_address TEXT",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN origin_floor INTEGER DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN destination_floor INTEGER DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN origin_has_elevator INTEGER DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN destination_has_elevator INTEGER DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN origin_parking_permit_needed INTEGER DEFAULT 0",
        )
        .await?;
        execute_migration_sql(conn, "ALTER TABLE job_tickets ADD COLUMN destination_parking_permit_needed INTEGER DEFAULT 0").await?;

        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS move_inventory (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
                job_ticket_id TEXT NOT NULL,
                item_category TEXT NOT NULL,
                item_name TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                estimated_volume_m3 REAL NOT NULL,
                handling_notes TEXT,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id)
            );
            CREATE TABLE IF NOT EXISTS move_quotes (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
                job_ticket_id TEXT NOT NULL UNIQUE,
                base_price REAL NOT NULL,
                distance_fee REAL NOT NULL,
                stairs_surcharge REAL NOT NULL,
                packing_supplies_fee REAL NOT NULL,
                total_price REAL NOT NULL,
                status TEXT NOT NULL,
                accepted_at INTEGER,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id)
            );
            CREATE INDEX IF NOT EXISTS idx_move_inventory_job ON move_inventory(job_ticket_id);
            CREATE INDEX IF NOT EXISTS idx_move_quotes_job ON move_quotes(job_ticket_id);",
        )
        .await?;

        execute_migration_sql(conn, "ALTER TABLE move_quotes ADD COLUMN use_rut INTEGER DEFAULT 0").await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE move_quotes ADD COLUMN rut_deduction_amount REAL DEFAULT 0.0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE move_quotes ADD COLUMN deposit_amount REAL DEFAULT 0.0",
        )
        .await?;
        execute_migration_sql(conn, "ALTER TABLE move_inventory ADD COLUMN preset_id TEXT").await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE move_inventory ADD COLUMN barcode_tag TEXT",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE move_inventory ADD COLUMN scan_status TEXT DEFAULT 'unscanned'",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE move_inventory ADD COLUMN last_scanned_at INTEGER",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE move_inventory ADD COLUMN last_scanned_by TEXT",
        )
        .await?;
        let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_move_inventory_barcode ON move_inventory(barcode_tag)", ()).await;

        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS student_profiles (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                user_id TEXT,
                first_name TEXT NOT NULL,
                last_name TEXT NOT NULL,
                grade_level TEXT NOT NULL,
                parent_contact TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(user_id) REFERENCES users(id)
            );
            CREATE TABLE IF NOT EXISTS student_parents (
                student_id TEXT NOT NULL,
                parent_user_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                PRIMARY KEY(student_id, parent_user_id),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id),
                FOREIGN KEY(parent_user_id) REFERENCES users(id)
            );
            CREATE TABLE IF NOT EXISTS courses (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                subject TEXT NOT NULL,
                teacher_id TEXT,
                classroom TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
            );
            CREATE TABLE IF NOT EXISTS assignments (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                course_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                due_date TEXT NOT NULL,
                max_points INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(course_id) REFERENCES courses(id)
            );
            CREATE TABLE IF NOT EXISTS submissions (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                assignment_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                content TEXT NOT NULL,
                grade TEXT,
                feedback TEXT,
                submitted_at TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(assignment_id) REFERENCES assignments(id),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS attendance_records (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                course_id TEXT NOT NULL,
                date TEXT NOT NULL,
                status TEXT NOT NULL,
                notes TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id),
                FOREIGN KEY(course_id) REFERENCES courses(id)
            );
            CREATE TABLE IF NOT EXISTS term_grades (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                course_id TEXT NOT NULL,
                term_name TEXT NOT NULL,
                final_grade TEXT,
                final_points INTEGER,
                teacher_comments TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id),
                FOREIGN KEY(course_id) REFERENCES courses(id)
            );
            CREATE TABLE IF NOT EXISTS report_cards (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                term_name TEXT NOT NULL,
                gpa REAL NOT NULL,
                principal_comments TEXT,
                status TEXT NOT NULL DEFAULT 'draft',
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS timetable_slots (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                course_id TEXT NOT NULL,
                day_of_week INTEGER NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                classroom TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(course_id) REFERENCES courses(id)
            );
            CREATE TABLE IF NOT EXISTS health_records (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                vaccine_name TEXT NOT NULL,
                status TEXT NOT NULL,
                administered_at TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS health_incidents (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                visit_reason TEXT NOT NULL,
                treatment TEXT NOT NULL,
                checked_in_at TEXT NOT NULL,
                checked_out_at TEXT,
                notes TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS school_invoices (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                title TEXT NOT NULL,
                amount REAL NOT NULL,
                due_date TEXT NOT NULL,
                status TEXT NOT NULL,
                paid_at TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS school_payments (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                invoice_id TEXT NOT NULL,
                amount REAL NOT NULL,
                payment_method TEXT NOT NULL,
                paid_at TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(invoice_id) REFERENCES school_invoices(id)
            );
            CREATE TABLE IF NOT EXISTS library_books (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                title TEXT NOT NULL,
                author TEXT NOT NULL,
                isbn TEXT NOT NULL,
                copies_available INTEGER NOT NULL,
                total_copies INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
            );
            CREATE TABLE IF NOT EXISTS library_lending_logs (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                book_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                checked_out_at TEXT NOT NULL,
                due_date TEXT NOT NULL,
                returned_at TEXT,
                status TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(book_id) REFERENCES library_books(id),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE INDEX IF NOT EXISTS idx_courses_workspace ON courses(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_assignments_course ON assignments(course_id);
            CREATE INDEX IF NOT EXISTS idx_submissions_assignment ON submissions(assignment_id);
            CREATE INDEX IF NOT EXISTS idx_attendance_records_course_date ON attendance_records(course_id, date);
            CREATE INDEX IF NOT EXISTS idx_attendance_records_student ON attendance_records(student_id);"
        ).await?;

        execute_migration_sql(conn, "ALTER TABLE student_profiles ADD COLUMN user_id TEXT").await?;

        version = 2;
    }
    if version < 3 {
        execute_migration_sql(
            conn,
            "ALTER TABLE audit_logs ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'",
        )
        .await?;
        execute_migration_sql(
            conn,
            "CREATE INDEX IF NOT EXISTS idx_audit_logs_workspace ON audit_logs(workspace_id)",
        )
        .await?;
        version = 3;
    }
    if version < 4 {
        execute_migration_sql(
            conn,
            "ALTER TABLE audit_logs ADD COLUMN seq INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        version = 4;
    }
    if version < 5 {
        execute_migration_sql(
            conn,
            "ALTER TABLE workspaces ADD COLUMN creator_public_key TEXT",
        )
        .await?;
        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN role_signature TEXT").await?;
        version = 5;
    }
    if version < 6 {
        execute_migration_batch(
            conn,
            "CREATE INDEX IF NOT EXISTS idx_todos_workspace ON todos(workspace_id);
             CREATE INDEX IF NOT EXISTS idx_note_updates_note_seq ON note_updates(note_id, seq);
             CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender_id);
             CREATE INDEX IF NOT EXISTS idx_messages_target_team ON messages(target_team_id);
             CREATE INDEX IF NOT EXISTS idx_teams_workspace ON teams(workspace_id);
             CREATE INDEX IF NOT EXISTS idx_events_team ON events(team_id);
             CREATE INDEX IF NOT EXISTS idx_time_reports_user_date ON time_reports(user_id, date);
             CREATE INDEX IF NOT EXISTS idx_job_tickets_workspace ON job_tickets(workspace_id);
             CREATE INDEX IF NOT EXISTS idx_job_tickets_assigned_user ON job_tickets(assigned_user_id);
             CREATE INDEX IF NOT EXISTS idx_move_inventory_job_ticket ON move_inventory(job_ticket_id);
             CREATE INDEX IF NOT EXISTS idx_move_quotes_job_ticket ON move_quotes(job_ticket_id);
             CREATE INDEX IF NOT EXISTS idx_student_profiles_workspace ON student_profiles(workspace_id);
             CREATE INDEX IF NOT EXISTS idx_student_profiles_user ON student_profiles(user_id);
             CREATE INDEX IF NOT EXISTS idx_courses_workspace ON courses(workspace_id);
             CREATE INDEX IF NOT EXISTS idx_assignments_course ON assignments(course_id);
             CREATE INDEX IF NOT EXISTS idx_submissions_assignment ON submissions(assignment_id);
             CREATE INDEX IF NOT EXISTS idx_submissions_student ON submissions(student_id);
             CREATE INDEX IF NOT EXISTS idx_attendance_records_student ON attendance_records(student_id);
             CREATE INDEX IF NOT EXISTS idx_attendance_records_course ON attendance_records(course_id);
             CREATE INDEX IF NOT EXISTS idx_term_grades_student ON term_grades(student_id);
             CREATE INDEX IF NOT EXISTS idx_term_grades_course ON term_grades(course_id);
             CREATE INDEX IF NOT EXISTS idx_report_cards_student ON report_cards(student_id);
             CREATE INDEX IF NOT EXISTS idx_timetable_slots_course ON timetable_slots(course_id);
             CREATE INDEX IF NOT EXISTS idx_health_records_student ON health_records(student_id);
             CREATE INDEX IF NOT EXISTS idx_health_incidents_student ON health_incidents(student_id);
             CREATE INDEX IF NOT EXISTS idx_school_invoices_student ON school_invoices(student_id);
             CREATE INDEX IF NOT EXISTS idx_school_payments_invoice ON school_payments(invoice_id);
             CREATE INDEX IF NOT EXISTS idx_library_lending_logs_book ON library_lending_logs(book_id);
             CREATE INDEX IF NOT EXISTS idx_library_lending_logs_student ON library_lending_logs(student_id);"
        ).await?;
        version = 6;
    }
    if version < 7 {
        execute_migration_sql(conn, "ALTER TABLE blocks ADD COLUMN fields_schema TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE blocks ADD COLUMN navigation_items TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE blocks ADD COLUMN ui_config TEXT").await?;
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                block_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
                FOREIGN KEY(block_id) REFERENCES blocks(id)
            );
            CREATE INDEX IF NOT EXISTS idx_entities_block ON entities(workspace_id, block_id);",
        )
        .await?;
        version = 7;
    }
    if version < 8 {
        execute_migration_sql(
            conn,
            "ALTER TABLE workspaces ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE workspaces ADD COLUMN sync_status TEXT DEFAULT 'pending'",
        )
        .await?;
        version = 8;
    }
    if version < 9 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS oauth_auth_sessions (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                token TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                authenticated_user_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )
        .await?;
        version = 9;
    }
    if version < 10 {
        execute_migration_sql(conn, "ALTER TABLE audit_logs ADD COLUMN signature TEXT").await?;
        version = 10;
    }
    if version < 11 {
        execute_migration_batch(
            conn,
            "CREATE INDEX IF NOT EXISTS idx_time_reports_workspace_date ON time_reports(workspace_id, date DESC);
             CREATE INDEX IF NOT EXISTS idx_notes_team_created ON notes(team_id, created_at DESC);
             CREATE INDEX IF NOT EXISTS idx_notes_workspace_created ON notes(workspace_id, created_at DESC);
             CREATE INDEX IF NOT EXISTS idx_messages_workspace_created ON messages(workspace_id, created_at ASC);"
        ).await?;
        version = 11;
    }
    if version < 12 {
        execute_migration_sql(
            conn,
            "ALTER TABLE bankid_auth_sessions ADD COLUMN token TEXT",
        )
        .await?;
        version = 12;
    }
    if version < 13 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS move_invoices (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
                quote_id TEXT NOT NULL,
                customer_id TEXT NOT NULL,
                invoice_date TEXT NOT NULL,
                due_date TEXT NOT NULL,
                subtotal REAL NOT NULL,
                rut_deduction REAL NOT NULL,
                customer_amount REAL NOT NULL,
                tax_authority_amount REAL NOT NULL,
                status TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(quote_id) REFERENCES move_quotes(id)
            );
            CREATE INDEX IF NOT EXISTS idx_move_invoices_quote ON move_invoices(quote_id);",
        )
        .await?;
        version = 13;
    }
    if version < 14 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS vehicles (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
                name TEXT NOT NULL,
                license_plate TEXT NOT NULL,
                capacity_m3 REAL NOT NULL,
                status TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
            );
            ALTER TABLE job_tickets ADD COLUMN assigned_vehicle_id TEXT;",
        )
        .await?;
        version = 14;
    }
    if version < 15 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS job_crew (
                job_ticket_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'mover',
                PRIMARY KEY(job_ticket_id, user_id),
                FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id) ON DELETE CASCADE,
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
            );",
        )
        .await?;
        version = 15;
    }
    if version < 16 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS move_signatures (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
                job_ticket_id TEXT NOT NULL,
                signer_name TEXT NOT NULL,
                signature_data_base64 TEXT NOT NULL,
                signed_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id) ON DELETE CASCADE
            );",
        )
        .await?;
        version = 16;
    }
    if version < 17 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS student_profiles (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                user_id TEXT,
                first_name TEXT NOT NULL,
                last_name TEXT NOT NULL,
                grade_level TEXT NOT NULL,
                parent_contact TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(user_id) REFERENCES users(id)
            );
            CREATE TABLE IF NOT EXISTS student_parents (
                student_id TEXT NOT NULL,
                parent_user_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                PRIMARY KEY(student_id, parent_user_id),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id),
                FOREIGN KEY(parent_user_id) REFERENCES users(id)
            );
            CREATE TABLE IF NOT EXISTS courses (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                subject TEXT NOT NULL,
                teacher_id TEXT,
                classroom TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
            );
            CREATE TABLE IF NOT EXISTS assignments (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                course_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                due_date TEXT NOT NULL,
                max_points INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(course_id) REFERENCES courses(id)
            );
            CREATE TABLE IF NOT EXISTS submissions (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                assignment_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                content TEXT NOT NULL,
                grade TEXT,
                feedback TEXT,
                submitted_at TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(assignment_id) REFERENCES assignments(id),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS attendance_records (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                course_id TEXT NOT NULL,
                date TEXT NOT NULL,
                status TEXT NOT NULL,
                notes TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id),
                FOREIGN KEY(course_id) REFERENCES courses(id)
            );
            CREATE TABLE IF NOT EXISTS term_grades (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                course_id TEXT NOT NULL,
                term_name TEXT NOT NULL,
                final_grade TEXT,
                final_points INTEGER,
                teacher_comments TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id),
                FOREIGN KEY(course_id) REFERENCES courses(id)
            );
            CREATE TABLE IF NOT EXISTS report_cards (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                term_name TEXT NOT NULL,
                gpa REAL NOT NULL,
                principal_comments TEXT,
                status TEXT NOT NULL DEFAULT 'draft',
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS timetable_slots (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                course_id TEXT NOT NULL,
                day_of_week INTEGER NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                classroom TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(course_id) REFERENCES courses(id)
            );
            CREATE TABLE IF NOT EXISTS health_records (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                vaccine_name TEXT NOT NULL,
                status TEXT NOT NULL,
                administered_at TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS health_incidents (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                visit_reason TEXT NOT NULL,
                treatment TEXT NOT NULL,
                checked_in_at TEXT NOT NULL,
                checked_out_at TEXT,
                notes TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS school_invoices (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                title TEXT NOT NULL,
                amount REAL NOT NULL,
                due_date TEXT NOT NULL,
                status TEXT NOT NULL,
                paid_at TEXT,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE TABLE IF NOT EXISTS school_payments (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                invoice_id TEXT NOT NULL,
                amount REAL NOT NULL,
                payment_method TEXT NOT NULL,
                paid_at TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(invoice_id) REFERENCES school_invoices(id)
            );
            CREATE TABLE IF NOT EXISTS library_books (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                title TEXT NOT NULL,
                author TEXT NOT NULL,
                isbn TEXT NOT NULL,
                copies_available INTEGER NOT NULL,
                total_copies INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
            );
            CREATE TABLE IF NOT EXISTS library_lending_logs (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                book_id TEXT NOT NULL,
                student_id TEXT NOT NULL,
                checked_out_at TEXT NOT NULL,
                due_date TEXT NOT NULL,
                returned_at TEXT,
                status TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(book_id) REFERENCES library_books(id),
                FOREIGN KEY(student_id) REFERENCES student_profiles(id)
            );
            CREATE INDEX IF NOT EXISTS idx_courses_workspace ON courses(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_assignments_course ON assignments(course_id);
            CREATE INDEX IF NOT EXISTS idx_submissions_assignment ON submissions(assignment_id);
            CREATE INDEX IF NOT EXISTS idx_attendance_records_course_date ON attendance_records(course_id, date);
            CREATE INDEX IF NOT EXISTS idx_attendance_records_student ON attendance_records(student_id);
            CREATE INDEX IF NOT EXISTS idx_student_profiles_workspace ON student_profiles(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_student_profiles_user ON student_profiles(user_id);
            CREATE INDEX IF NOT EXISTS idx_submissions_student ON submissions(student_id);
            CREATE INDEX IF NOT EXISTS idx_attendance_records_course ON attendance_records(course_id);
            CREATE INDEX IF NOT EXISTS idx_term_grades_student ON term_grades(student_id);
            CREATE INDEX IF NOT EXISTS idx_term_grades_course ON term_grades(course_id);
            CREATE INDEX IF NOT EXISTS idx_report_cards_student ON report_cards(student_id);
            CREATE INDEX IF NOT EXISTS idx_timetable_slots_course ON timetable_slots(course_id);
            CREATE INDEX IF NOT EXISTS idx_health_records_student ON health_records(student_id);
            CREATE INDEX IF NOT EXISTS idx_health_incidents_student ON health_incidents(student_id);
            CREATE INDEX IF NOT EXISTS idx_school_invoices_student ON school_invoices(student_id);",
        )
        .await?;
        version = 17;
    }
    if version < 18 {
        execute_migration_sql(
            conn,
            "ALTER TABLE client_journals ADD COLUMN author_id TEXT",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE client_journals ADD COLUMN created_at TEXT NOT NULL DEFAULT ''",
        )
        .await?;
        version = 18;
    }
    if version < 19 {
        execute_migration_sql(
            conn,
            "CREATE TABLE IF NOT EXISTS school_conflicts (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                entity_table TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                conflict_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        )
        .await?;
        execute_migration_sql(
            conn,
            "CREATE INDEX IF NOT EXISTS idx_school_conflicts_entity ON school_conflicts(entity_table, entity_id)",
        )
        .await?;
        execute_migration_sql(
            conn,
            "CREATE TABLE IF NOT EXISTS local_blobs (
                sha256 TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
        )
        .await?;
        execute_migration_sql(
            conn,
            "CREATE INDEX IF NOT EXISTS idx_local_blobs_workspace ON local_blobs(workspace_id)",
        )
        .await?;
        version = 19;
    }
    if version < 20 {
        execute_migration_sql(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN route_stops_json TEXT",
        )
        .await?;
        version = 20;
    }
    if version < 21 {
        execute_migration_batch(
            conn,
            "ALTER TABLE vehicles ADD COLUMN latitude REAL;
             ALTER TABLE vehicles ADD COLUMN longitude REAL;
             ALTER TABLE vehicles ADD COLUMN last_ping INTEGER;
             ALTER TABLE vehicles ADD COLUMN gps_device_id TEXT;
             ALTER TABLE vehicles ADD COLUMN max_payload_kg REAL;",
        )
        .await?;
        version = 21;
    }
    if version < 22 {
        execute_migration_batch(
            conn,
            "ALTER TABLE job_tickets ADD COLUMN long_carry_meters INTEGER DEFAULT 0;
             ALTER TABLE job_tickets ADD COLUMN toll_fees REAL DEFAULT 0.0;",
        )
        .await?;
        version = 22;
    }
    if version < 23 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS job_packaging_items (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
                job_ticket_id TEXT NOT NULL,
                item_name TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                price_per_unit REAL NOT NULL,
                is_leased INTEGER DEFAULT 0,
                returned_quantity INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id)
            );",
        )
        .await?;
        version = 23;
    }
    if version < 24 {
        execute_migration_batch(
            conn,
            "ALTER TABLE move_invoices ADD COLUMN actual_hours REAL;
             ALTER TABLE move_invoices ADD COLUMN additional_charges REAL;
             ALTER TABLE move_invoices ADD COLUMN adjustment_notes TEXT;",
        )
        .await?;
        version = 24;
    }
    if version < 25 {
        execute_migration_batch(
            conn,
            "ALTER TABLE move_quotes ADD COLUMN manual_price_override REAL;
             ALTER TABLE move_quotes ADD COLUMN price_discount REAL DEFAULT 0.0;",
        )
        .await?;
        version = 25;
    }
    if version < 26 {
        execute_migration_batch(
            conn,
            "ALTER TABLE hvac_diagnostics ADD COLUMN system_type TEXT DEFAULT 'REFRIGERANT_HVAC';
             ALTER TABLE hvac_diagnostics ADD COLUMN water_pressure_bar REAL DEFAULT 0.0;
             ALTER TABLE hvac_diagnostics ADD COLUMN asset_id TEXT;
             CREATE TABLE IF NOT EXISTS job_parts_used (
                 id TEXT PRIMARY KEY,
                 workspace_id TEXT NOT NULL,
                 job_ticket_id TEXT NOT NULL,
                 part_name TEXT NOT NULL,
                 quantity REAL NOT NULL,
                 unit_cost_sek REAL NOT NULL,
                 rot_eligible INTEGER NOT NULL DEFAULT 0,
                 created_at INTEGER NOT NULL
             );
              CREATE INDEX IF NOT EXISTS idx_job_parts_used_ticket ON job_parts_used(job_ticket_id, workspace_id);",
        )
        .await?;
        version = 26;
    }
    if version < 27 {
        execute_migration_batch(
            conn,
            "ALTER TABLE hvac_diagnostics ADD COLUMN operating_mode TEXT DEFAULT 'COOLING_MODE';
             ALTER TABLE hvac_diagnostics ADD COLUMN ambient_temp_c REAL;",
        )
        .await?;
        version = 27;
    }
    if version < 28 {
        execute_migration_batch(
            conn,
            "ALTER TABLE hvac_diagnostics ADD COLUMN static_flow_pressure_bar REAL;
             ALTER TABLE hvac_diagnostics ADD COLUMN dynamic_flow_pressure_bar REAL;
             ALTER TABLE hvac_diagnostics ADD COLUMN pipe_material TEXT;
             ALTER TABLE hvac_diagnostics ADD COLUMN backflow_preventer_status TEXT;
             ALTER TABLE hvac_diagnostics ADD COLUMN water_heater_temp_c REAL;
             ALTER TABLE hvac_diagnostics ADD COLUMN leak_test_duration_min REAL;
             ALTER TABLE hvac_diagnostics ADD COLUMN leak_test_pressure_drop_bar REAL;
             CREATE INDEX IF NOT EXISTS idx_hvac_diagnostics_ticket ON hvac_diagnostics(job_ticket_id, workspace_id);",
        )
        .await?;
        version = 28;
    }
    if version < 29 {
        execute_migration_batch(
            conn,
            "DROP INDEX IF EXISTS idx_job_parts_used_ticket;
             CREATE INDEX IF NOT EXISTS idx_job_parts_used_ticket ON job_parts_used(job_ticket_id, workspace_id);",
        )
        .await?;
        version = 29;
    }
    if version < 30 {
        execute_migration_batch(
            conn,
            "ALTER TABLE hvac_diagnostics ADD COLUMN refrigerant_added_kg REAL;
             ALTER TABLE hvac_diagnostics ADD COLUMN refrigerant_recovered_kg REAL;
             ALTER TABLE hvac_diagnostics ADD COLUMN reclaim_cylinder_id TEXT;",
        )
        .await?;
        version = 30;
    }
    if version < 31 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS ai_action_triggers (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                created_by TEXT NOT NULL,
                natural_language_prompt TEXT NOT NULL,
                condition_type TEXT NOT NULL,
                condition_params TEXT NOT NULL,
                action_type TEXT NOT NULL,
                action_params TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
            );
            CREATE INDEX IF NOT EXISTS idx_ai_action_triggers_ws ON ai_action_triggers(workspace_id, is_active);

            CREATE TABLE IF NOT EXISTS ai_daily_digests (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                date TEXT NOT NULL,
                summary_text TEXT NOT NULL,
                total_hours_logged REAL NOT NULL,
                total_reports_count INTEGER NOT NULL,
                flagged_reports_count INTEGER NOT NULL,
                audit_events_count INTEGER NOT NULL,
                telemetry_summary_json TEXT NOT NULL,
                digest_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
            );
            CREATE INDEX IF NOT EXISTS idx_ai_daily_digests_ws_date ON ai_daily_digests(workspace_id, date DESC);",
        )
        .await?;
        version = 31;
    }
    if version < 32 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS data_imports (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                file_name TEXT NOT NULL,
                file_format TEXT NOT NULL,
                records_total INTEGER NOT NULL DEFAULT 0,
                records_imported INTEGER NOT NULL DEFAULT 0,
                records_failed INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'completed',
                summary_json TEXT DEFAULT '{}',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
            );
            CREATE INDEX IF NOT EXISTS idx_data_imports_ws ON data_imports(workspace_id, created_at DESC);

            CREATE TABLE IF NOT EXISTS calendar_integrations (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                account_email TEXT NOT NULL,
                access_token TEXT,
                refresh_token TEXT,
                token_expires_at INTEGER DEFAULT 0,
                sync_direction TEXT NOT NULL DEFAULT 'two_way',
                auto_sync_enabled INTEGER NOT NULL DEFAULT 1,
                last_synced_at INTEGER DEFAULT 0,
                sync_status TEXT DEFAULT 'idle',
                error_message TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
            );
            CREATE INDEX IF NOT EXISTS idx_calendar_integrations_ws ON calendar_integrations(workspace_id);

            CREATE TABLE IF NOT EXISTS calendar_sync_mappings (
                id TEXT PRIMARY KEY,
                integration_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                local_event_id TEXT NOT NULL,
                external_event_id TEXT NOT NULL,
                external_etag TEXT,
                last_synced_at INTEGER NOT NULL,
                FOREIGN KEY(integration_id) REFERENCES calendar_integrations(id)
            );
            CREATE INDEX IF NOT EXISTS idx_calendar_sync_map ON calendar_sync_mappings(integration_id, local_event_id);

            CREATE TABLE IF NOT EXISTS webhook_endpoints (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                target_url TEXT NOT NULL,
                secret TEXT NOT NULL,
                events TEXT NOT NULL DEFAULT '[]',
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
            );
            CREATE INDEX IF NOT EXISTS idx_webhook_endpoints_ws ON webhook_endpoints(workspace_id);

            CREATE TABLE IF NOT EXISTS webhook_delivery_logs (
                id TEXT PRIMARY KEY,
                endpoint_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                status TEXT NOT NULL,
                response_code INTEGER DEFAULT 0,
                response_body TEXT,
                attempt_count INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(endpoint_id) REFERENCES webhook_endpoints(id)
            );
            CREATE INDEX IF NOT EXISTS idx_webhook_logs_endpoint ON webhook_delivery_logs(endpoint_id, created_at DESC);",
        )
        .await?;
        version = 32;
    }
    if version < 33 {
        let has_sync_tok: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM pragma_table_info('calendar_integrations') WHERE name = 'sync_token'",
                (),
                |r| r.get(0),
            )
            .await
            .ok();

        if has_sync_tok.is_none() {
            conn.execute_batch(
                "ALTER TABLE calendar_integrations ADD COLUMN sync_token TEXT;
                 ALTER TABLE webhook_endpoints ADD COLUMN consecutive_failures INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE webhook_endpoints ADD COLUMN circuit_state TEXT NOT NULL DEFAULT 'closed';
                 ALTER TABLE webhook_delivery_logs ADD COLUMN idempotency_key TEXT;
                 ALTER TABLE webhook_delivery_logs ADD COLUMN next_retry_at INTEGER DEFAULT 0;",
            )
            .await?;
        }
        version = 33;
    }
    if version < 34 {
        execute_migration_sql(
            conn,
            "ALTER TABLE client_medications ADD COLUMN fhir_payload TEXT DEFAULT '{}'",
        )
        .await?;
        execute_migration_sql(
            conn,
            "ALTER TABLE report_cards ADD COLUMN edfi_payload TEXT DEFAULT '{}'",
        )
        .await?;
        version = 34;
    }
    if version < 35 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS wasm_plugins (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                domain_scope TEXT NOT NULL,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                bytecode_base64 TEXT NOT NULL,
                manifest_json TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
            );
            CREATE INDEX IF NOT EXISTS idx_wasm_plugins_workspace ON wasm_plugins(workspace_id, domain_scope);",
        )
        .await?;
        version = 35;
    }
    if version < 36 {
        execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS ehr_integrations (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                fhir_endpoint_url TEXT NOT NULL,
                account_id TEXT,
                api_token TEXT,
                sync_direction TEXT NOT NULL DEFAULT 'two_way',
                auto_sync_enabled INTEGER NOT NULL DEFAULT 1,
                last_synced_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT NOT NULL DEFAULT 'idle',
                error_message TEXT,
                sync_token TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
            );
            CREATE TABLE IF NOT EXISTS ehr_sync_mappings (
                id TEXT PRIMARY KEY,
                integration_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                local_entity_id TEXT NOT NULL,
                external_fhir_id TEXT NOT NULL,
                external_etag TEXT,
                last_synced_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(integration_id) REFERENCES ehr_integrations(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS sis_integrations (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                edfi_endpoint_url TEXT NOT NULL,
                client_key TEXT,
                client_secret TEXT,
                sync_direction TEXT NOT NULL DEFAULT 'two_way',
                auto_sync_enabled INTEGER NOT NULL DEFAULT 1,
                last_synced_at INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT NOT NULL DEFAULT 'idle',
                error_message TEXT,
                sync_token TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
            );
            CREATE TABLE IF NOT EXISTS sis_sync_mappings (
                id TEXT PRIMARY KEY,
                integration_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                local_entity_id TEXT NOT NULL,
                external_edfi_id TEXT NOT NULL,
                external_etag TEXT,
                last_synced_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(integration_id) REFERENCES sis_integrations(id) ON DELETE CASCADE
            );",
        )
        .await?;
        version = 36;
    }
    if version < 37 {
        let _ = execute_migration_batch(
            conn,
            "ALTER TABLE ehr_integrations ADD COLUMN refresh_token TEXT;
             ALTER TABLE ehr_integrations ADD COLUMN token_expires_at INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE ehr_integrations ADD COLUMN mtls_client_cert_pem TEXT;
             ALTER TABLE ehr_integrations ADD COLUMN mtls_client_key_pem TEXT;",
        )
        .await;
        version = 37;
    }
    if version < 38 {
        let _ = execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS audit_merkle_nodes (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                tree_level INTEGER NOT NULL,
                node_index INTEGER NOT NULL,
                hash TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_audit_merkle_level_index ON audit_merkle_nodes(workspace_id, tree_level, node_index);",
        )
        .await;
        version = 38;
    }
    if version < 39 {
        let _ = execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS mllp_listeners (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 2575,
                bind_address TEXT NOT NULL DEFAULT '0.0.0.0',
                tls_enabled INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'stopped',
                last_active_at INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS hl7_messages (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                listener_id TEXT,
                message_type TEXT NOT NULL,
                trigger_event TEXT NOT NULL,
                sending_app TEXT,
                sending_facility TEXT,
                message_control_id TEXT NOT NULL,
                patient_mrn TEXT,
                patient_name TEXT,
                encounter_id TEXT,
                raw_payload TEXT NOT NULL,
                parsed_json TEXT NOT NULL,
                ack_status TEXT NOT NULL DEFAULT 'AA',
                ack_payload TEXT,
                status TEXT NOT NULL DEFAULT 'processed',
                received_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_hl7_messages_ws ON hl7_messages(workspace_id, received_at DESC);",
        )
        .await;
        version = 39;
    }
    if version < 40 {
        let _ = execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS fhir_resource_mappings (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                fhir_id TEXT NOT NULL,
                internal_entity_type TEXT NOT NULL,
                internal_entity_id TEXT NOT NULL,
                raw_fhir_json TEXT NOT NULL,
                last_synced_at INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_fhir_mappings_ws ON fhir_resource_mappings(workspace_id, resource_type, fhir_id);",
        )
        .await;
        version = 40;
    }
    if version < 41 {
        let _ = execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS ncpdp_prescriptions (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                client_id TEXT NOT NULL,
                prescriber_id TEXT NOT NULL,
                prescriber_npi TEXT NOT NULL,
                pharmacy_npi TEXT NOT NULL,
                pharmacy_name TEXT,
                drug_name TEXT NOT NULL,
                rxnorm_code TEXT,
                ndc_code TEXT,
                quantity REAL NOT NULL,
                days_supply INTEGER NOT NULL,
                refills INTEGER NOT NULL DEFAULT 0,
                sig_instructions TEXT NOT NULL,
                transaction_type TEXT NOT NULL DEFAULT 'NewRx',
                status TEXT NOT NULL DEFAULT 'draft',
                surescripts_tx_id TEXT,
                raw_xml_payload TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
                FOREIGN KEY(client_id) REFERENCES clients(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_ncpdp_prescriptions_ws ON ncpdp_prescriptions(workspace_id, client_id);",
        )
        .await;
        version = 41;
    }
    if version < 42 {
        let _ = execute_migration_batch(
            conn,
            "CREATE TABLE IF NOT EXISTS fda_part11_signatures (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                target_record_type TEXT NOT NULL,
                target_record_id TEXT NOT NULL,
                primary_signer_id TEXT NOT NULL,
                primary_signer_name TEXT NOT NULL,
                primary_intent TEXT NOT NULL,
                primary_ed25519_sig TEXT NOT NULL,
                primary_pubkey TEXT NOT NULL,
                secondary_signer_id TEXT,
                secondary_signer_name TEXT,
                secondary_intent TEXT,
                secondary_ed25519_sig TEXT,
                secondary_pubkey TEXT,
                dual_sign_completed INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_fda_part11_target ON fda_part11_signatures(workspace_id, target_record_type, target_record_id);",
        )
        .await;
        version = 42;
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_sanity() {
        let _lock = crate::database::DB_TEST_LOCK.lock().unwrap();

        let db = libsql::Builder::new_local(":memory:")
            .build()
            .await
            .unwrap();
        let raw_conn = db.connect().unwrap();
        let mut conn = crate::database::DbConnection {
            inner: Some(raw_conn),
            in_transaction: std::sync::atomic::AtomicBool::new(false),
            _permit: None,
        };

        crate::database::schema::tables::create_initial_tables(&conn)
            .await
            .unwrap();

        conn.execute("PRAGMA user_version = 0", ()).await.unwrap();

        let migrated_version = run_schema_migrations(&conn, 0).await.unwrap();
        assert_eq!(migrated_version, 42);

        let has_oauth_sessions = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='oauth_auth_sessions'",
            (),
            |r| r.get::<i64>(0),
        ).await.unwrap_or(0) > 0;
        assert!(has_oauth_sessions);

        // Take the inner connection out to prevent it from being recycled into the global pool
        let _ = conn.inner.take();
    }
}
