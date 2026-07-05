use super::super::DbConnection;

pub async fn seed_mock_data(conn: &DbConnection) {
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

    let alice_pub = const_hex::encode(ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]).verifying_key().to_bytes());
    let bob_pub = const_hex::encode(ed25519_dalek::SigningKey::from_bytes(&[2u8; 32]).verifying_key().to_bytes());

    let _ = conn.execute(
        "UPDATE users SET siths_card_id = 'SITHS-ALICE-123', siths_public_key = ?1, nfc_badge_uid = 'NFC-ALICE-999' WHERE id = 'user-1'",
        crate::params![alice_pub],
    ).await;
    let _ = conn.execute(
        "UPDATE users SET siths_card_id = 'SITHS-BOB-456', siths_public_key = ?1, nfc_badge_uid = 'NFC-BOB-888' WHERE id = 'user-2'",
        crate::params![bob_pub],
    ).await;

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
