use crate::YntraError;
use super::super::DbConnection;

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

pub async fn run_schema_migrations(conn: &DbConnection, current_version: i32) -> Result<i32, YntraError> {
    let mut version = current_version;
    if version < 2 {
        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN siths_card_id TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN nfc_badge_uid TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN personal_number TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE invitations ADD COLUMN siths_card_id TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE invitations ADD COLUMN nfc_badge_uid TEXT").await?;

        execute_migration_sql(conn, "ALTER TABLE todos ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'").await?;
        execute_migration_sql(conn, "ALTER TABLE todos ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE todos ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE users ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE workspaces ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE workspaces ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE teams ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE teams ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE team_members ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'").await?;
        execute_migration_sql(conn, "ALTER TABLE team_members ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE team_members ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE events ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE events ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE messages ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE messages ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE notes ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE notes ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE time_reports ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE time_reports ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE clients ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE clients ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE client_medications ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'").await?;
        execute_migration_sql(conn, "ALTER TABLE client_medications ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE client_medications ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE client_journals ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'").await?;
        execute_migration_sql(conn, "ALTER TABLE client_journals ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE client_journals ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE reports ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE reports ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE invitations ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE invitations ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;

        execute_migration_sql(conn, "ALTER TABLE job_tickets ADD COLUMN origin_address TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE job_tickets ADD COLUMN destination_address TEXT").await?;
        execute_migration_sql(conn, "ALTER TABLE job_tickets ADD COLUMN origin_floor INTEGER DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE job_tickets ADD COLUMN destination_floor INTEGER DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE job_tickets ADD COLUMN origin_has_elevator INTEGER DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE job_tickets ADD COLUMN destination_has_elevator INTEGER DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE job_tickets ADD COLUMN origin_parking_permit_needed INTEGER DEFAULT 0").await?;
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
                job_ticket_id TEXT NOT NULL,
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
            CREATE INDEX IF NOT EXISTS idx_move_quotes_job ON move_quotes(job_ticket_id);"
        ).await?;

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
        execute_migration_sql(conn, "ALTER TABLE audit_logs ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'").await?;
        execute_migration_sql(conn, "CREATE INDEX IF NOT EXISTS idx_audit_logs_workspace ON audit_logs(workspace_id)").await?;
        version = 3;
    }
    if version < 4 {
        execute_migration_sql(conn, "ALTER TABLE audit_logs ADD COLUMN seq INTEGER NOT NULL DEFAULT 0").await?;
        version = 4;
    }
    if version < 5 {
        execute_migration_sql(conn, "ALTER TABLE workspaces ADD COLUMN creator_public_key TEXT").await?;
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
            CREATE INDEX IF NOT EXISTS idx_entities_block ON entities(workspace_id, block_id);"
        ).await?;
        version = 7;
    }
    if version < 8 {
        execute_migration_sql(conn, "ALTER TABLE workspaces ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0").await?;
        execute_migration_sql(conn, "ALTER TABLE workspaces ADD COLUMN sync_status TEXT DEFAULT 'pending'").await?;
        version = 8;
    }
    Ok(version)
}
