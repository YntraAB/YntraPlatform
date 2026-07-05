use crate::YntraError;
use super::DbConnection;

pub async fn setup_schema(conn: &DbConnection) -> Result<(), YntraError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS todos (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            text TEXT NOT NULL,
            completed INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
        )",
        (),
    )
    .await
    .map_err(|e| YntraError::DbError(e.to_string()))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS workspaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            modules_active TEXT NOT NULL,
            settings TEXT NOT NULL,
            brand_color TEXT DEFAULT '#3b82f6',
            logo_url TEXT,
            block_settings TEXT DEFAULT '{}'
        );

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            workspace_id TEXT,
            email TEXT NOT NULL,
            full_name TEXT,
            phone TEXT,
            role TEXT NOT NULL DEFAULT 'user',
            preferences TEXT NOT NULL DEFAULT '{}',
            password_hash TEXT,
            siths_card_id TEXT,
            nfc_badge_uid TEXT,
            personal_number TEXT,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );

        CREATE TABLE IF NOT EXISTS teams (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            name TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );

        CREATE TABLE IF NOT EXISTS team_members (
            team_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            role_id TEXT,
            notes_last_read_at TEXT,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            PRIMARY KEY(team_id, user_id),
            FOREIGN KEY(team_id) REFERENCES teams(id),
            FOREIGN KEY(user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            user_id TEXT,
            team_id TEXT,
            assignee_id TEXT,
            title TEXT NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            metadata TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
            FOREIGN KEY(user_id) REFERENCES users(id),
            FOREIGN KEY(team_id) REFERENCES teams(id),
            FOREIGN KEY(assignee_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            sender_id TEXT,
            receiver_id TEXT,
            target_team_id TEXT,
            subject TEXT,
            body TEXT,
            is_read INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
            FOREIGN KEY(sender_id) REFERENCES users(id),
            FOREIGN KEY(receiver_id) REFERENCES users(id),
            FOREIGN KEY(target_team_id) REFERENCES teams(id)
        );

        CREATE TABLE IF NOT EXISTS notes (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            team_id TEXT NOT NULL,
            author_id TEXT,
            subject TEXT NOT NULL,
            content TEXT NOT NULL,
            edit_history TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
            FOREIGN KEY(team_id) REFERENCES teams(id),
            FOREIGN KEY(author_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS time_reports (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            team_id TEXT,
            date TEXT NOT NULL,
            start_time TEXT,
            end_time TEXT,
            hours REAL NOT NULL,
            note TEXT,
            status TEXT NOT NULL DEFAULT 'pending_attest',
            created_at TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
            FOREIGN KEY(user_id) REFERENCES users(id),
            FOREIGN KEY(team_id) REFERENCES teams(id)
        );

        CREATE TABLE IF NOT EXISTS clients (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            team_id TEXT,
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL,
            personal_number TEXT,
            care_level TEXT,
            message_settings TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
            FOREIGN KEY(team_id) REFERENCES teams(id)
        );

        CREATE TABLE IF NOT EXISTS client_medications (
            id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL,
            name TEXT NOT NULL,
            dosage TEXT,
            frequency TEXT,
            instructions TEXT,
            created_at TEXT NOT NULL,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(client_id) REFERENCES clients(id)
        );

        CREATE TABLE IF NOT EXISTS client_journals (
            id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL,
            author_id TEXT,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(client_id) REFERENCES clients(id),
            FOREIGN KEY(author_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS blocks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            icon TEXT NOT NULL,
            category TEXT NOT NULL,
            dependencies TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS reports (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            type TEXT NOT NULL,
            is_anonymous INTEGER NOT NULL DEFAULT 0,
            content TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
            FOREIGN KEY(user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS bankid_auth_sessions (
            id TEXT PRIMARY KEY,
            target_role TEXT NOT NULL,
            provider TEXT NOT NULL,
            status TEXT NOT NULL,
            pin TEXT NOT NULL,
            qr_data TEXT NOT NULL,
            progress REAL NOT NULL DEFAULT 0.0,
            authenticated_user_id TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_tickets (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            location_address TEXT NOT NULL,
            priority TEXT NOT NULL,
            status TEXT NOT NULL,
            assigned_user_id TEXT,
            scheduled_date TEXT NOT NULL,
            checklist_json TEXT NOT NULL,
            completion_report TEXT,
            created_at TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            origin_address TEXT,
            destination_address TEXT,
            origin_floor INTEGER DEFAULT 0,
            destination_floor INTEGER DEFAULT 0,
            origin_has_elevator INTEGER DEFAULT 0,
            destination_has_elevator INTEGER DEFAULT 0,
            origin_parking_permit_needed INTEGER DEFAULT 0,
            destination_parking_permit_needed INTEGER DEFAULT 0,
            FOREIGN KEY(assigned_user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS invitations (
            code TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            email TEXT NOT NULL,
            full_name TEXT NOT NULL,
            role TEXT NOT NULL,
            activated INTEGER DEFAULT 0,
            siths_card_id TEXT,
            nfc_badge_uid TEXT,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
        );

        CREATE TABLE IF NOT EXISTS audit_logs (
            id TEXT PRIMARY KEY,
            actor_id TEXT NOT NULL,
            target_client_id TEXT,
            action_type TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            prev_hash TEXT NOT NULL,
            curr_hash TEXT NOT NULL
        );"
    )
    .await
    .map(|_| ())
    .map_err(|e| YntraError::DbError(e.to_string()))?;

    let _ = conn.execute("ALTER TABLE users ADD COLUMN siths_card_id TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE users ADD COLUMN nfc_badge_uid TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE users ADD COLUMN personal_number TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE invitations ADD COLUMN siths_card_id TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE invitations ADD COLUMN nfc_badge_uid TEXT", ()).await;

    let _ = conn.execute("ALTER TABLE todos ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'", ()).await;
    let _ = conn.execute("ALTER TABLE todos ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE todos ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE users ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE users ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE teams ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE teams ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE team_members ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'", ()).await;
    let _ = conn.execute("ALTER TABLE team_members ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE team_members ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE events ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE events ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE messages ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE messages ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE notes ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE notes ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE time_reports ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE time_reports ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE clients ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE clients ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE client_medications ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'", ()).await;
    let _ = conn.execute("ALTER TABLE client_medications ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE client_medications ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE client_journals ADD COLUMN workspace_id TEXT NOT NULL DEFAULT 'workspace-1'", ()).await;
    let _ = conn.execute("ALTER TABLE client_journals ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE client_journals ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE reports ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE reports ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE invitations ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE invitations ADD COLUMN sync_status TEXT DEFAULT 'pending'", ()).await;

    let _ = conn.execute("ALTER TABLE job_tickets ADD COLUMN origin_address TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE job_tickets ADD COLUMN destination_address TEXT", ()).await;
    let _ = conn.execute("ALTER TABLE job_tickets ADD COLUMN origin_floor INTEGER DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE job_tickets ADD COLUMN destination_floor INTEGER DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE job_tickets ADD COLUMN origin_has_elevator INTEGER DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE job_tickets ADD COLUMN destination_has_elevator INTEGER DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE job_tickets ADD COLUMN origin_parking_permit_needed INTEGER DEFAULT 0", ()).await;
    let _ = conn.execute("ALTER TABLE job_tickets ADD COLUMN destination_parking_permit_needed INTEGER DEFAULT 0", ()).await;

    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS move_inventory (
            id TEXT PRIMARY KEY,
            job_ticket_id TEXT NOT NULL,
            item_category TEXT NOT NULL,
            item_name TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            estimated_volume_m3 REAL NOT NULL,
            handling_notes TEXT,
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id)
        );
        CREATE TABLE IF NOT EXISTS move_quotes (
            id TEXT PRIMARY KEY,
            job_ticket_id TEXT NOT NULL,
            base_price REAL NOT NULL,
            distance_fee REAL NOT NULL,
            stairs_surcharge REAL NOT NULL,
            packing_supplies_fee REAL NOT NULL,
            total_price REAL NOT NULL,
            status TEXT NOT NULL,
            accepted_at INTEGER,
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id)
        );"
    ).await;

    let _ = conn.execute_batch(
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
        );"
    ).await;

    let _ = conn.execute("ALTER TABLE student_profiles ADD COLUMN user_id TEXT", ()).await;
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS student_parents (
            student_id TEXT NOT NULL,
            parent_user_id TEXT NOT NULL,
            PRIMARY KEY(student_id, parent_user_id),
            FOREIGN KEY(student_id) REFERENCES student_profiles(id),
            FOREIGN KEY(parent_user_id) REFERENCES users(id)
        );"
    ).await;

    seed_mock_data(conn).await;
    Ok(())
}

async fn seed_mock_data(conn: &DbConnection) {
    let _ = conn.execute_batch(
        "INSERT OR IGNORE INTO users (id, workspace_id, email, full_name, role, preferences, updated_at, sync_status) VALUES
        ('user-1', 'workspace-1', 'marie.andersson@yntra.se', 'Marie Andersson', 'assistant', '{}', 1719830400000, 'synced'),
        ('user-2', 'workspace-1', 'dev.user@yntra.se', 'Dev User', 'platform_admin', '{}', 1719830400000, 'synced');
        
        INSERT OR IGNORE INTO teams (id, workspace_id, name, updated_at, sync_status) VALUES
        ('team-1', 'workspace-1', 'Default Team', 1719830400000, 'synced');
        
        INSERT OR IGNORE INTO team_members (team_id, user_id, workspace_id, updated_at, sync_status) VALUES
        ('team-1', 'user-1', 'workspace-1', 1719830400000, 'synced'),
        ('team-1', 'user-2', 'workspace-1', 1719830400000, 'synced');"
    ).await;

    let _ = conn.execute(
        "UPDATE users SET siths_card_id = 'SITHS-ALICE-123', nfc_badge_uid = 'NFC-ALICE-999' WHERE id = 'user-1'",
        (),
    ).await;
    let _ = conn.execute(
        "UPDATE users SET siths_card_id = 'SITHS-BOB-456', nfc_badge_uid = 'NFC-BOB-888' WHERE id = 'user-2'",
        (),
    ).await;

    #[allow(unused_mut)]
    let mut admin_email = "".to_string();
    #[cfg(not(target_arch = "wasm32"))]
    {
        admin_email = std::env::var("ADMIN_EMAIL").unwrap_or_default();
        if admin_email.is_empty() {
            if let Ok(content) = std::fs::read_to_string("admin_email.txt") {
                admin_email = content.trim().to_string();
            } else if let Ok(content) = std::fs::read_to_string("../admin_email.txt") {
                admin_email = content.trim().to_string();
            }
        }
        if admin_email.is_empty() {
            for path in &[".env", "../.env"] {
                if let Ok(content) = std::fs::read_to_string(path) {
                    for line in content.lines() {
                        if let Some(stripped) = line.strip_prefix("ADMIN_EMAIL=") {
                            admin_email = stripped.trim().trim_matches('"').trim_matches('\'').to_string();
                            break;
                        }
                    }
                    if !admin_email.is_empty() {
                        break;
                    }
                }
            }
        }
    }

    if !admin_email.is_empty() {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO users (id, workspace_id, email, full_name, role, preferences) VALUES ('user-env-admin', 'workspace-1', ?1, 'Admin User', 'platform_admin', '{}')",
            crate::params![&admin_email],
        ).await;
        let _ = conn.execute(
            "UPDATE users SET role = 'platform_admin' WHERE email = ?1",
            crate::params![&admin_email],
        ).await;
    }

    let _ = conn.execute_batch(
        "
        DELETE FROM blocks WHERE id = 'school';
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'dashboard', 'Dashboard', 'Central overview and command center', 'LayoutGrid', 'Core', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'messaging', 'Messaging', 'Internal messaging system', 'MessageSquare', 'Communication', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'scheduling', 'Scheduling', 'Calendar and scheduling management', 'Calendar', 'Operations', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'notes', 'Notes', 'Team notes and documentation', 'FileText', 'Communication', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'directory', 'Directory', 'Team and user directory', 'Users', 'Core', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'assistance', 'Assistance', 'Client assistance and overview', 'Heart', 'Care', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'journals', 'Care Journals', 'Document client care diaries, daily reports, and support logs', 'BookOpen', 'Care', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'medications', 'Medications', 'Log active medication plans, instructions, and dosages', 'Activity', 'Care', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'time', 'Time', 'Time management and reporting', 'Clock', 'Operations', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'reporting', 'Reporting', 'Incident and deviation reporting', 'AlertTriangle', 'Operations', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'jobs', 'Jobs', 'Job Tickets & Work Orders', 'Wrench', 'Operations', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'todos', 'Todos', 'Task list and personal todo manager', 'CheckSquare', 'Core', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'academics', 'Academics', 'Academics, courses and grading management', 'BookOpen', 'Education', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'attendance', 'Attendance', 'Student attendance tracking', 'UserCheck', 'Education', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'finance', 'Finance', 'Finance, invoices and fee management', 'CreditCard', 'Operations', '[]', '2026-06-30 00:00:00'
        );
        INSERT OR IGNORE INTO blocks (id, name, description, icon, category, dependencies, created_at) VALUES (
            'library', 'Library', 'Library catalog and lending log', 'BookOpen', 'Education', '[]', '2026-06-30 00:00:00'
        );
        "
    ).await;

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM workspaces", (), |row| row.get(0))
        .await
        .unwrap_or(0);

    if count == 0 {
        let _ = conn.execute_batch(
            "INSERT INTO workspaces (id, name, modules_active, settings, brand_color, logo_url, block_settings) VALUES (
                'workspace-1',
                'Yntra Care & Operations Ltd',
                '{\"messaging\":true,\"scheduling\":true,\"notes\":true,\"time\":true,\"assistance\":true,\"directory\":true,\"reporting\":true}',
                '{\"roles\":[{\"id\":\"role-samordnare\",\"name\":\"Verksamhetssamordnare\",\"permissions\":{\"can_manage_schedule\":true,\"can_manage_notes\":true,\"can_approve_time_reports\":true,\"can_manage_clients\":true,\"can_view_journals\":true,\"can_write_journals\":true,\"can_view_medications\":true,\"can_manage_medications\":true,\"can_manage_workspace\":true,\"can_manage_users\":true,\"can_view_audit_logs\":true,\"can_submit_reports\":true,\"can_manage_reports\":true}},{\"id\":\"role-sjukskoterska\",\"name\":\"Leg. Sjuksköterska\",\"permissions\":{\"can_manage_schedule\":false,\"can_manage_notes\":true,\"can_approve_time_reports\":false,\"can_manage_clients\":true,\"can_view_journals\":true,\"can_write_journals\":true,\"can_view_medications\":true,\"can_manage_medications\":true,\"can_manage_workspace\":false,\"can_manage_users\":false,\"can_view_audit_logs\":false,\"can_submit_reports\":true,\"can_manage_reports\":true}},{\"id\":\"role-underskoterska\",\"name\":\"Undersköterska\",\"permissions\":{\"can_manage_schedule\":false,\"can_manage_notes\":true,\"can_approve_time_reports\":false,\"can_manage_clients\":false,\"can_view_journals\":true,\"can_write_journals\":true,\"can_view_medications\":true,\"can_manage_medications\":false,\"can_manage_workspace\":false,\"can_manage_users\":false,\"can_view_audit_logs\":false,\"can_submit_reports\":true,\"can_manage_reports\":false}}]}',
                '#3b82f6',
                NULL,
                '{}'
            );"
        ).await;

        let _ = conn.execute_batch(
            "
            INSERT OR IGNORE INTO invitations (code, workspace_id, email, full_name, role, activated, siths_card_id, nfc_badge_uid) VALUES (
                'YNTRA-CARE-2026',
                'workspace-1',
                'marie.andersson@yntra.se',
                'Marie Andersson',
                'assistant',
                0,
                'SITHS-BOB-456',
                'NFC-BOB-888'
            );
            INSERT OR IGNORE INTO invitations (code, workspace_id, email, full_name, role, activated, siths_card_id, nfc_badge_uid) VALUES (
                'WELCOME-OFFLINE-FIRST',
                'workspace-1',
                'dev.user@yntra.se',
                'Dev User',
                'platform_admin',
                0,
                'SITHS-ALICE-123',
                'NFC-ALICE-999'
            );
            "
        ).await;

        let jobs_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM job_tickets", (), |row| row.get(0))
            .await
            .unwrap_or(0);

        if jobs_count == 0 {
            let _ = conn.execute_batch(
                "INSERT INTO job_tickets (
                    id, workspace_id, title, description, location_address, priority, status,
                    assigned_user_id, scheduled_date, checklist_json, completion_report,
                    created_at, updated_at, sync_status,
                    origin_address, destination_address, origin_floor, destination_floor,
                    origin_has_elevator, destination_has_elevator,
                    origin_parking_permit_needed, destination_parking_permit_needed
                ) VALUES (
                    'job-1',
                    'workspace-1',
                    'Bohagsflytt - Familjen Andersson',
                    'Komplett bohagsflytt av 4:a till radhus inklusive packning av porslin.',
                    'Vasagatan 12, Stockholm',
                    'high',
                    'assigned',
                    'user-2',
                    '2026-07-10',
                    '[{\"text\":\"Packa porslin och glas i köket\",\"done\":false},{\"text\":\"Demontera dubbelsäng\",\"done\":false},{\"text\":\"Lasta lastbil\",\"done\":false},{\"text\":\"Transportera till nya adressen\",\"done\":false},{\"text\":\"Montera dubbelsäng på nya adressen\",\"done\":false}]',
                    NULL,
                    '2026-07-04 12:00:00',
                    0,
                    'synced',
                    'Vasagatan 12, Stockholm',
                    'Storgatan 45, Solna',
                    3,
                    1,
                    1,
                    1,
                    1,
                    0
                );

                INSERT INTO job_tickets (
                    id, workspace_id, title, description, location_address, priority, status,
                    assigned_user_id, scheduled_date, checklist_json, completion_report,
                    created_at, updated_at, sync_status,
                    origin_address, destination_address, origin_floor, destination_floor,
                    origin_has_elevator, destination_has_elevator,
                    origin_parking_permit_needed, destination_parking_permit_needed
                ) VALUES (
                    'job-2',
                    'workspace-1',
                    'Kontorsflytt - Tech Corp AB',
                    'Flytt av 15 arbetsplatser, skrivare och konferensrumsmöbler.',
                    'Kungsgatan 3, Stockholm',
                    'critical',
                    'in_progress',
                    'user-2',
                    '2026-07-12',
                    '[{\"text\":\"Märk alla kartonger och skärmar\",\"done\":true},{\"text\":\"Montera ner konferensbord\",\"done\":false},{\"text\":\"Transportera IT-utrustning i specialburar\",\"done\":false}]',
                    NULL,
                    '2026-07-04 12:05:00',
                    0,
                    'synced',
                    'Kungsgatan 3, Stockholm',
                    'Sveavägen 100, Stockholm',
                    5,
                    0,
                    1,
                    1,
                    0,
                    1
                );

                INSERT INTO move_inventory (id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes) VALUES
                ('inv-1-1', 'job-1', 'Möbler', 'Soffa 3-sits', 1, 1.5, 'Svep med plastfolie'),
                ('inv-1-2', 'job-1', 'Möbler', 'Matbord', 1, 0.8, 'Montera ner ben'),
                ('inv-1-3', 'job-1', 'Möbler', 'Stolar', 6, 1.2, NULL),
                ('inv-1-4', 'job-1', 'Kartonger', 'Standardkartonger', 30, 3.0, 'Märk med Kök/Vardagsrum');

                INSERT INTO move_quotes (id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at) VALUES
                ('quote-1', 'job-1', 4500.0, 800.0, 1200.0, 500.0, 7000.0, 'sent', NULL);

                INSERT INTO move_inventory (id, job_ticket_id, item_category, item_name, quantity, estimated_volume_m3, handling_notes) VALUES
                ('inv-2-1', 'job-2', 'Kontor', 'Skrivbord', 15, 6.0, 'Kräver demontering'),
                ('inv-2-2', 'job-2', 'Kontor', 'Kontorsstolar', 15, 4.5, NULL),
                ('inv-2-3', 'job-2', 'Möbler', 'Konferensbord', 1, 2.0, 'Tung glasskiva'),
                ('inv-2-4', 'job-2', 'IT', 'Datorburar', 5, 2.5, 'Ömtåligt, transporteras i burar');

                INSERT INTO move_quotes (id, job_ticket_id, base_price, distance_fee, stairs_surcharge, packing_supplies_fee, total_price, status, accepted_at) VALUES
                ('quote-2', 'job-2', 15000.0, 1200.0, 3000.0, 1800.0, 21000.0, 'accepted', 1719830400000);
                "
            ).await;
        }

        let students_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM student_profiles", (), |row| row.get(0))
            .await
            .unwrap_or(0);

        if students_count == 0 {
            let _ = conn.execute_batch(
                "INSERT INTO users (id, workspace_id, email, full_name, role, preferences, updated_at, sync_status) VALUES
                ('user-parent-1', 'workspace-1', 'parent.smith@example.com', 'Sarah Smith', 'parent', '{}', 1719830400000, 'synced'),
                ('user-parent-2', 'workspace-1', 'parent.johnson@example.com', 'Michael Johnson', 'parent', '{}', 1719830400000, 'synced'),
                ('user-parent-3', 'workspace-1', 'parent.brown@example.com', 'Eleanor Brown', 'parent', '{}', 1719830400000, 'synced');

                INSERT INTO student_profiles (id, workspace_id, first_name, last_name, grade_level, parent_contact, updated_at, sync_status) VALUES
                ('student-1', 'workspace-1', 'Alice', 'Smith', 'Grade 10', 'parent.smith@example.com', 1719830400000, 'synced'),
                ('student-2', 'workspace-1', 'Bob', 'Johnson', 'Grade 10', 'parent.johnson@example.com', 1719830400000, 'synced'),
                ('student-3', 'workspace-1', 'Charlie', 'Brown', 'Grade 11', 'parent.brown@example.com', 1719830400000, 'synced');

                INSERT INTO courses (id, workspace_id, name, subject, teacher_id, classroom, updated_at, sync_status) VALUES
                ('course-1', 'workspace-1', 'Mathematics 101', 'Math', 'user-1', 'Room 204', 1719830400000, 'synced'),
                ('course-2', 'workspace-1', 'Science & Physics', 'Science', 'user-2', 'Lab 3', 1719830400000, 'synced');

                INSERT INTO assignments (id, workspace_id, course_id, title, description, due_date, max_points, updated_at, sync_status) VALUES
                ('assign-1', 'workspace-1', 'course-1', 'Algebra Quiz 1', 'Basic equations and quadratic formulas.', '2026-07-15', 100, 1719830400000, 'synced'),
                ('assign-2', 'workspace-1', 'course-2', 'Physics Lab Report', 'Friction and acceleration lab.', '2026-07-20', 50, 1719830400000, 'synced');

                INSERT INTO submissions (id, workspace_id, assignment_id, student_id, content, grade, feedback, submitted_at, updated_at, sync_status) VALUES
                ('sub-1', 'workspace-1', 'assign-1', 'student-1', 'Solved equations 1-10 with full showing work.', 'A', 'Perfect work!', '2026-07-04 10:00:00', 1719830400000, 'synced'),
                ('sub-2', 'workspace-1', 'assign-1', 'student-2', 'Incomplete work for equations 8-10.', 'C', 'Needs more explanations.', '2026-07-04 10:15:00', 1719830400000, 'synced');

                INSERT INTO attendance_records (id, workspace_id, student_id, course_id, date, status, notes, updated_at, sync_status) VALUES
                ('att-1', 'workspace-1', 'student-1', 'course-1', '2026-07-04', 'present', NULL, 1719830400000, 'synced'),
                ('att-2', 'workspace-1', 'student-2', 'course-1', '2026-07-04', 'late', '15 mins late due to bus delay', 1719830400000, 'synced'),
                ('att-3', 'workspace-1', 'student-3', 'course-1', '2026-07-04', 'absent', 'Sickness', 1719830400000, 'synced');

                INSERT INTO term_grades (id, workspace_id, student_id, course_id, term_name, final_grade, final_points, teacher_comments, updated_at, sync_status) VALUES
                ('tg-1', 'workspace-1', 'student-1', 'course-1', 'Fall 2026', 'A', 95, 'Outstanding analytical skills.', 1719830400000, 'synced'),
                ('tg-2', 'workspace-1', 'student-1', 'course-2', 'Fall 2026', 'A', 98, 'Brilliant lab investigations.', 1719830400000, 'synced'),
                ('tg-3', 'workspace-1', 'student-2', 'course-1', 'Fall 2026', 'B', 85, 'Good work, but should focus more on algebraic proofs.', 1719830400000, 'synced'),
                ('tg-4', 'workspace-1', 'student-2', 'course-2', 'Fall 2026', 'C', 72, 'Requires additional study on thermodynamics.', 1719830400000, 'synced');

                INSERT INTO report_cards (id, workspace_id, student_id, term_name, gpa, principal_comments, status, updated_at, sync_status) VALUES
                ('rc-1', 'workspace-1', 'student-1', 'Fall 2026', 4.0, 'Sarah, Alice is an exceptional student. Her dedication to excellence is commendable.', 'published', 1719830400000, 'synced'),
                ('rc-2', 'workspace-1', 'student-2', 'Fall 2026', 2.5, 'Bob is showing solid progress, but needs to focus on science.', 'draft', 1719830400000, 'synced');

                INSERT INTO health_records (id, workspace_id, student_id, vaccine_name, status, administered_at, updated_at, sync_status) VALUES
                ('hr-1', 'workspace-1', 'student-1', 'Covid-19', 'completed', '2025-09-15', 1719830400000, 'synced'),
                ('hr-2', 'workspace-1', 'student-1', 'MMR', 'completed', '2024-05-10', 1719830400000, 'synced'),
                ('hr-3', 'workspace-1', 'student-2', 'Covid-19', 'completed', '2025-10-01', 1719830400000, 'synced'),
                ('hr-4', 'workspace-1', 'student-2', 'MMR', 'pending', NULL, 1719830400000, 'synced');

                INSERT INTO health_incidents (id, workspace_id, student_id, visit_reason, treatment, checked_in_at, checked_out_at, notes, updated_at, sync_status) VALUES
                ('hi-1', 'workspace-1', 'student-1', 'Headache', 'Rest & Ice pack', '2026-07-04 09:30', '2026-07-04 10:00', 'Returned to class after resting.', 1719830400000, 'synced'),
                ('hi-2', 'workspace-1', 'student-2', 'Scraped Knee', 'Cleaned & applied bandage', '2026-07-04 11:45', '2026-07-04 11:55', 'Slight scrape from soccer during recess.', 1719830400000, 'synced');

                INSERT INTO school_invoices (id, workspace_id, student_id, title, amount, due_date, status, paid_at, updated_at, sync_status) VALUES
                ('inv-1', 'workspace-1', 'student-1', 'Fall 2026 Tuition', 1200.00, '2026-07-01', 'paid', '2026-06-30', 1719830400000, 'synced'),
                ('inv-2', 'workspace-1', 'student-1', 'Chemistry Lab Fee', 75.00, '2026-07-20', 'unpaid', NULL, 1719830400000, 'synced'),
                ('inv-3', 'workspace-1', 'student-2', 'Fall 2026 Tuition', 1200.00, '2026-07-01', 'unpaid', NULL, 1719830400000, 'synced');

                INSERT INTO school_payments (id, workspace_id, invoice_id, amount, payment_method, paid_at, updated_at, sync_status) VALUES
                ('pay-1', 'workspace-1', 'inv-1', 1200.00, 'bank_transfer', '2026-06-30 09:00:00', 1719830400000, 'synced');

                INSERT INTO library_books (id, workspace_id, title, author, isbn, copies_available, total_copies, updated_at, sync_status) VALUES
                ('book-1', 'workspace-1', 'Introduction to Algorithms', 'Thomas H. Cormen', '978-0262033848', 2, 3, 1719830400000, 'synced'),
                ('book-2', 'workspace-1', 'The Catcher in the Rye', 'J.D. Salinger', '978-0316769174', 4, 5, 1719830400000, 'synced'),
                ('book-3', 'workspace-1', 'Brief Answers to the Big Questions', 'Stephen Hawking', '978-1473560246', 1, 1, 1719830400000, 'synced');

                INSERT INTO library_lending_logs (id, workspace_id, book_id, student_id, checked_out_at, due_date, returned_at, status, updated_at, sync_status) VALUES
                ('loan-1', 'workspace-1', 'book-1', 'student-1', '2026-06-25', '2026-07-09', NULL, 'active', 1719830400000, 'synced'),
                ('loan-2', 'workspace-1', 'book-2', 'student-2', '2026-06-10', '2026-06-24', NULL, 'overdue', 1719830400000, 'synced');
                "
            ).await;
        }
    }
}
