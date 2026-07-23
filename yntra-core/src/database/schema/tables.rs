use super::super::DbConnection;
use crate::YntraError;

pub async fn create_initial_tables(conn: &DbConnection) -> Result<(), YntraError> {
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
            block_settings TEXT DEFAULT '{}',
            creator_public_key TEXT,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced'))
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
            metadata TEXT DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            role_signature TEXT,
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
            content_plain TEXT,
            edit_history TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
            FOREIGN KEY(team_id) REFERENCES teams(id),
            FOREIGN KEY(author_id) REFERENCES users(id)
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
            id UNINDEXED,
            subject,
            content_plain,
            tokenize='unicode61'
        );

        CREATE TABLE IF NOT EXISTS note_updates (
            id TEXT PRIMARY KEY,
            note_id TEXT NOT NULL,
            client_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            update_data TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(note_id) REFERENCES notes(id)
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
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            name TEXT NOT NULL,
            dosage TEXT,
            frequency TEXT,
            instructions TEXT,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(client_id) REFERENCES clients(id)
        );

        CREATE TABLE IF NOT EXISTS client_journals (
            id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            content TEXT NOT NULL,
            author_id TEXT,
            created_at TEXT NOT NULL DEFAULT '',
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(client_id) REFERENCES clients(id)
        );



        CREATE TABLE IF NOT EXISTS blocks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            icon TEXT NOT NULL,
            category TEXT NOT NULL,
            dependencies TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            fields_schema TEXT,
            navigation_items TEXT,
            ui_config TEXT
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
            error_message TEXT,
            qr_data TEXT NOT NULL,
            progress REAL NOT NULL DEFAULT 0.0,
            authenticated_user_id TEXT,
            created_at TEXT NOT NULL,
            challenge TEXT,
            token TEXT
        );

        CREATE TABLE IF NOT EXISTS oauth_auth_sessions (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            token TEXT NOT NULL,
            status TEXT NOT NULL,
            error_message TEXT,
            authenticated_user_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS vehicles (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            name TEXT NOT NULL,
            license_plate TEXT NOT NULL,
            capacity_m3 REAL NOT NULL,
            max_payload_kg REAL,
            status TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            latitude REAL,
            longitude REAL,
            last_ping INTEGER,
            gps_device_id TEXT
        );

        CREATE TABLE IF NOT EXISTS eld_hos_logs (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            driver_id TEXT NOT NULL,
            driver_name TEXT NOT NULL,
            vehicle_id TEXT NOT NULL,
            status TEXT NOT NULL,
            driving_hours_today REAL NOT NULL,
            on_duty_hours_today REAL NOT NULL,
            cycle_hours_7day REAL NOT NULL,
            rest_break_required INTEGER NOT NULL,
            violation_flag INTEGER NOT NULL,
            violation_reason TEXT,
            timestamp_ms INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS dvir_inspections (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            vehicle_id TEXT NOT NULL,
            inspector_driver_id TEXT NOT NULL,
            inspection_type TEXT NOT NULL,
            brakes_ok INTEGER NOT NULL,
            tires_ok INTEGER NOT NULL,
            lights_ok INTEGER NOT NULL,
            steering_ok INTEGER NOT NULL,
            coupling_devices_ok INTEGER NOT NULL,
            defects_found INTEGER NOT NULL,
            defect_details TEXT,
            safety_status TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS ifta_fuel_logs (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            vehicle_id TEXT NOT NULL,
            driver_id TEXT NOT NULL,
            from_jurisdiction TEXT NOT NULL,
            to_jurisdiction TEXT NOT NULL,
            odometer_km REAL NOT NULL,
            fuel_purchased_liters REAL NOT NULL,
            timestamp_ms INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS hvac_diagnostics (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            job_ticket_id TEXT NOT NULL,
            technician_id TEXT NOT NULL,
            system_type TEXT DEFAULT 'REFRIGERANT_HVAC',
            refrigerant_type TEXT NOT NULL,
            refrigerant_charge_level TEXT NOT NULL,
            high_side_psi REAL NOT NULL,
            low_side_psi REAL NOT NULL,
            water_pressure_bar REAL DEFAULT 0.0,
            temp_differential_c REAL NOT NULL,
            voltage_v REAL NOT NULL,
            amp_draw_a REAL NOT NULL,
            diagnostic_status TEXT NOT NULL,
            asset_id TEXT,
            notes TEXT,
            operating_mode TEXT DEFAULT 'COOLING_MODE',
            ambient_temp_c REAL,
            created_at INTEGER NOT NULL
        );

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
        CREATE INDEX IF NOT EXISTS idx_job_parts_used_ticket ON job_parts_used(job_ticket_id);

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
            assigned_vehicle_id TEXT,
            route_stops_json TEXT,
            long_carry_meters INTEGER DEFAULT 0,
            toll_fees REAL DEFAULT 0.0,
            FOREIGN KEY(assigned_user_id) REFERENCES users(id),
            FOREIGN KEY(assigned_vehicle_id) REFERENCES vehicles(id)
        );

        CREATE TABLE IF NOT EXISTS job_crew (
            job_ticket_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'mover',
            PRIMARY KEY(job_ticket_id, user_id),
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id) ON DELETE CASCADE,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS move_signatures (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            job_ticket_id TEXT NOT NULL,
            signer_name TEXT NOT NULL,
            signature_data_base64 TEXT NOT NULL,
            signed_at INTEGER NOT NULL,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS warehouse_vaults (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            job_ticket_id TEXT NOT NULL,
            vault_number TEXT NOT NULL,
            warehouse_name TEXT NOT NULL,
            allocated_volume_m3 REAL NOT NULL,
            monthly_rate_sek REAL NOT NULL,
            move_in_date TEXT NOT NULL,
            estimated_move_out_date TEXT,
            status TEXT NOT NULL DEFAULT 'stored',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS job_tips (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            job_ticket_id TEXT NOT NULL,
            total_tip_amount REAL NOT NULL,
            crew_count INTEGER NOT NULL,
            tip_per_member REAL NOT NULL,
            status TEXT NOT NULL DEFAULT 'distributed',
            created_at INTEGER NOT NULL,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS fuel_receipts (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            vehicle_id TEXT NOT NULL,
            driver_user_id TEXT NOT NULL,
            liters REAL NOT NULL,
            cost_sek REAL NOT NULL,
            fuel_type TEXT NOT NULL,
            odometer_km INTEGER NOT NULL,
            receipt_image_url TEXT,
            station_name TEXT,
            purchase_date TEXT NOT NULL,
            erp_sync_status TEXT NOT NULL DEFAULT 'pending',
            erp_reference TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS move_inventory (
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
            room_name TEXT,
            estimated_weight_kg REAL DEFAULT 0.0,
            preset_id TEXT,
            barcode_tag TEXT,
            scan_status TEXT DEFAULT 'unscanned',
            last_scanned_at INTEGER,
            last_scanned_by TEXT,
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
            manual_price_override REAL,
            price_discount REAL DEFAULT 0.0,
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id)
        );

        CREATE TABLE IF NOT EXISTS move_quote_revisions (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            quote_id TEXT NOT NULL,
            job_ticket_id TEXT NOT NULL,
            actor_user_id TEXT NOT NULL,
            previous_total REAL NOT NULL,
            new_total REAL NOT NULL,
            revision_reason TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_move_quote_revisions_quote ON move_quote_revisions(quote_id);

        CREATE TABLE IF NOT EXISTS move_invoices (
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
            actual_hours REAL,
            additional_charges REAL,
            adjustment_notes TEXT,
            FOREIGN KEY(quote_id) REFERENCES move_quotes(id)
        );

        CREATE TABLE IF NOT EXISTS job_packaging_items (
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
        );

        CREATE TABLE IF NOT EXISTS damage_inspections (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            job_ticket_id TEXT NOT NULL,
            item_inventory_id TEXT,
            item_name TEXT NOT NULL,
            damage_type TEXT NOT NULL,
            severity TEXT NOT NULL,
            annotations TEXT,
            photo_url TEXT,
            timestamp_ms INTEGER NOT NULL,
            inspector_user_id TEXT NOT NULL,
            client_acknowledged INTEGER DEFAULT 0,
            client_signature_svg TEXT,
            created_at INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(job_ticket_id) REFERENCES job_tickets(id)
        );

        CREATE TABLE IF NOT EXISTS offline_media_blobs (
            hash TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            job_ticket_id TEXT NOT NULL,
            media_type TEXT NOT NULL,
            compressed_blob BLOB NOT NULL,
            original_size_bytes INTEGER NOT NULL,
            compressed_size_bytes INTEGER NOT NULL,
            mime_type TEXT NOT NULL,
            upload_status TEXT DEFAULT 'queued_offline',
            created_at INTEGER NOT NULL DEFAULT 0,
            synced_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS invitations (
            code TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            email TEXT NOT NULL,
            full_name TEXT NOT NULL,
            role TEXT NOT NULL,
            activated INTEGER DEFAULT 0,
            metadata TEXT DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            encrypted_workspace_key TEXT
        );

        CREATE TABLE IF NOT EXISTS audit_logs (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL DEFAULT 'workspace-1',
            actor_id TEXT NOT NULL,
            target_client_id TEXT,
            action_type TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            prev_hash TEXT NOT NULL,
            curr_hash TEXT NOT NULL,
            seq INTEGER NOT NULL DEFAULT 0,
            signature TEXT,
            UNIQUE(workspace_id, seq)
        );
        
        CREATE TABLE IF NOT EXISTS entities (
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

        CREATE TABLE IF NOT EXISTS student_profiles (
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


        CREATE TABLE IF NOT EXISTS school_conflicts (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            entity_table TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            conflict_json TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS local_blobs (
            sha256 TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            data TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        -- Indices
        CREATE INDEX IF NOT EXISTS idx_school_conflicts_entity ON school_conflicts(entity_table, entity_id);
        CREATE INDEX IF NOT EXISTS idx_local_blobs_workspace ON local_blobs(workspace_id);
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
        CREATE INDEX IF NOT EXISTS idx_library_lending_logs_student ON library_lending_logs(student_id);

        CREATE INDEX IF NOT EXISTS idx_users_workspace ON users(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
        CREATE INDEX IF NOT EXISTS idx_users_email_lower ON users(LOWER(email));
        CREATE INDEX IF NOT EXISTS idx_notes_team ON notes(team_id);
        CREATE INDEX IF NOT EXISTS idx_notes_workspace ON notes(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_events_workspace ON events(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_time_reports_user ON time_reports(user_id);
        CREATE INDEX IF NOT EXISTS idx_team_members_user ON team_members(user_id);
        CREATE INDEX IF NOT EXISTS idx_clients_workspace ON clients(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_messages_receiver ON messages(receiver_id);
        CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
        CREATE INDEX IF NOT EXISTS idx_audit_logs_workspace ON audit_logs(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_todos_workspace ON todos(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_note_updates_note_seq ON note_updates(note_id, seq);
        CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender_id);
        CREATE INDEX IF NOT EXISTS idx_messages_target_team ON messages(target_team_id);
        CREATE INDEX IF NOT EXISTS idx_teams_workspace ON teams(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_events_team ON events(team_id);
        CREATE INDEX IF NOT EXISTS idx_time_reports_user_date ON time_reports(user_id, date);
        CREATE INDEX IF NOT EXISTS idx_job_tickets_workspace ON job_tickets(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_job_tickets_assigned_user ON job_tickets(assigned_user_id);
        CREATE INDEX IF NOT EXISTS idx_entities_block ON entities(workspace_id, block_id);
        CREATE INDEX IF NOT EXISTS idx_time_reports_workspace_date ON time_reports(workspace_id, date DESC);
        CREATE INDEX IF NOT EXISTS idx_notes_team_created ON notes(team_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_notes_workspace_created ON notes(workspace_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_messages_workspace_created ON messages(workspace_id, created_at ASC);
        CREATE INDEX IF NOT EXISTS idx_move_inventory_job ON move_inventory(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_move_quotes_job ON move_quotes(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_move_inventory_job_ticket ON move_inventory(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_move_quotes_job_ticket ON move_quotes(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_move_invoices_quote ON move_invoices(quote_id);"
    )
    .await
    .map_err(|e| YntraError::DbError(e.to_string()))?;

    // Defensive column assertions for job_tickets
    let job_ticket_cols = [
        "ALTER TABLE job_tickets ADD COLUMN origin_address TEXT",
        "ALTER TABLE job_tickets ADD COLUMN destination_address TEXT",
        "ALTER TABLE job_tickets ADD COLUMN origin_floor INTEGER DEFAULT 0",
        "ALTER TABLE job_tickets ADD COLUMN destination_floor INTEGER DEFAULT 0",
        "ALTER TABLE job_tickets ADD COLUMN origin_has_elevator INTEGER DEFAULT 0",
        "ALTER TABLE job_tickets ADD COLUMN destination_has_elevator INTEGER DEFAULT 0",
        "ALTER TABLE job_tickets ADD COLUMN origin_parking_permit_needed INTEGER DEFAULT 0",
        "ALTER TABLE job_tickets ADD COLUMN destination_parking_permit_needed INTEGER DEFAULT 0",
        "ALTER TABLE job_tickets ADD COLUMN assigned_vehicle_id TEXT",
        "ALTER TABLE job_tickets ADD COLUMN route_stops_json TEXT",
        "ALTER TABLE job_tickets ADD COLUMN long_carry_meters INTEGER DEFAULT 0",
        "ALTER TABLE job_tickets ADD COLUMN toll_fees REAL DEFAULT 0.0",
    ];

    for col_sql in job_ticket_cols {
        let _ = conn.execute(col_sql, ()).await;
    }

    Ok(())
}
