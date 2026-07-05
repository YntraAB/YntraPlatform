use crate::YntraError;
use super::super::DbConnection;

pub async fn run_schema_migrations(conn: &DbConnection) -> Result<(), YntraError> {
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

    Ok(())
}
