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

        CREATE TABLE IF NOT EXISTS passkey_credentials (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            credential_id_hex TEXT NOT NULL UNIQUE,
            public_key_hex TEXT NOT NULL,
            counter INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT 0,
            last_used_at INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
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
            fhir_payload TEXT DEFAULT '{}',
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
            high_side_psi REAL,
            low_side_psi REAL,
            water_pressure_bar REAL DEFAULT 0.0,
            temp_differential_c REAL,
            voltage_v REAL,
            amp_draw_a REAL,
            diagnostic_status TEXT NOT NULL,
            asset_id TEXT,
            notes TEXT,
            operating_mode TEXT DEFAULT 'COOLING_MODE',
            ambient_temp_c REAL,
            static_flow_pressure_bar REAL,
            dynamic_flow_pressure_bar REAL,
            pipe_material TEXT,
            backflow_preventer_status TEXT,
            water_heater_temp_c REAL,
            leak_test_duration_min REAL,
            leak_test_pressure_drop_bar REAL,
            refrigerant_added_kg REAL,
            refrigerant_recovered_kg REAL,
            reclaim_cylinder_id TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_hvac_diagnostics_ticket ON hvac_diagnostics(job_ticket_id, workspace_id);

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
        CREATE INDEX IF NOT EXISTS idx_job_parts_used_ticket ON job_parts_used(job_ticket_id, workspace_id);

        CREATE TABLE IF NOT EXISTS location_assets (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            job_ticket_id TEXT,
            customer_id TEXT,
            asset_tag TEXT NOT NULL,
            model_name TEXT NOT NULL,
            serial_number TEXT NOT NULL,
            equipment_category TEXT NOT NULL,
            location_address TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_location_assets_ticket ON location_assets(job_ticket_id, workspace_id);
        CREATE INDEX IF NOT EXISTS idx_location_assets_customer ON location_assets(customer_id, workspace_id);

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

        CREATE TABLE IF NOT EXISTS wasm_plugins (
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

        CREATE TABLE IF NOT EXISTS ehr_integrations (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            fhir_endpoint_url TEXT NOT NULL,
            account_id TEXT,
            api_token TEXT,
            refresh_token TEXT,
            token_expires_at INTEGER NOT NULL DEFAULT 0,
            mtls_client_cert_pem TEXT,
            mtls_client_key_pem TEXT,
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
        );

        CREATE TABLE IF NOT EXISTS audit_merkle_nodes (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            tree_level INTEGER NOT NULL,
            node_index INTEGER NOT NULL,
            hash TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_audit_merkle_level_index ON audit_merkle_nodes(workspace_id, tree_level, node_index);

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
            edfi_payload TEXT DEFAULT '{}',
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

        CREATE TABLE IF NOT EXISTS workspace_subscriptions (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            tier TEXT NOT NULL DEFAULT 'starter',
            payment_platform TEXT NOT NULL DEFAULT 'stripe',
            external_subscription_id TEXT,
            external_customer_id TEXT,
            seats_allocated INTEGER NOT NULL DEFAULT 1,
            seats_used INTEGER NOT NULL DEFAULT 1,
            price_per_seat_monthly REAL NOT NULL DEFAULT 15.0,
            currency TEXT NOT NULL DEFAULT 'USD',
            billing_cycle TEXT NOT NULL DEFAULT 'monthly',
            status TEXT NOT NULL DEFAULT 'active',
            current_period_start INTEGER NOT NULL DEFAULT 0,
            current_period_end INTEGER NOT NULL DEFAULT 0,
            cancel_at_period_end INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );

        CREATE TABLE IF NOT EXISTS platform_invoices (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            subscription_id TEXT NOT NULL,
            invoice_number TEXT NOT NULL,
            payment_platform TEXT NOT NULL,
            amount_due REAL NOT NULL,
            amount_paid REAL NOT NULL,
            currency TEXT NOT NULL DEFAULT 'USD',
            seat_count INTEGER NOT NULL,
            period_start INTEGER NOT NULL,
            period_end INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'paid',
            pdf_download_url TEXT,
            created_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );

        CREATE INDEX IF NOT EXISTS idx_workspace_subscriptions_ws ON workspace_subscriptions(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_platform_invoices_ws ON platform_invoices(workspace_id);

        CREATE TABLE IF NOT EXISTS support_tickets (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            user_name TEXT NOT NULL,
            user_email TEXT NOT NULL,
            subject TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'technical',
            priority TEXT NOT NULL DEFAULT 'medium',
            status TEXT NOT NULL DEFAULT 'open',
            messages_json TEXT NOT NULL DEFAULT '[]',
            created_at INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending' CHECK(sync_status IN ('pending', 'synced')),
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );

        CREATE TABLE IF NOT EXISTS helpdesk_articles (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            category TEXT NOT NULL,
            summary TEXT NOT NULL,
            content_markdown TEXT NOT NULL,
            tags_json TEXT NOT NULL DEFAULT '[]',
            views_count INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS user_tour_progress (
            workspace_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            tour_name TEXT NOT NULL,
            current_step INTEGER NOT NULL DEFAULT 0,
            total_steps INTEGER NOT NULL DEFAULT 5,
            completed INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(workspace_id, user_id, tour_name)
        );

        CREATE INDEX IF NOT EXISTS idx_support_tickets_user ON support_tickets(user_id);
        CREATE INDEX IF NOT EXISTS idx_support_tickets_ws ON support_tickets(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_helpdesk_articles_category ON helpdesk_articles(category);

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
        CREATE INDEX IF NOT EXISTS idx_move_invoices_quote ON move_invoices(quote_id);

        CREATE INDEX IF NOT EXISTS idx_client_medications_client ON client_medications(client_id);
        CREATE INDEX IF NOT EXISTS idx_client_journals_client ON client_journals(client_id);
        CREATE INDEX IF NOT EXISTS idx_move_signatures_job ON move_signatures(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_warehouse_vaults_job ON warehouse_vaults(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_job_tips_job ON job_tips(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_fuel_receipts_vehicle ON fuel_receipts(vehicle_id);
        CREATE INDEX IF NOT EXISTS idx_fuel_receipts_driver ON fuel_receipts(driver_user_id);
        CREATE INDEX IF NOT EXISTS idx_job_packaging_items_job ON job_packaging_items(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_damage_inspections_job ON damage_inspections(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_offline_media_blobs_job ON offline_media_blobs(job_ticket_id);
        CREATE INDEX IF NOT EXISTS idx_student_parents_parent ON student_parents(parent_user_id);

        CREATE TABLE IF NOT EXISTS ai_action_triggers (
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
        CREATE INDEX IF NOT EXISTS idx_ai_daily_digests_ws_date ON ai_daily_digests(workspace_id, date DESC);

        CREATE TABLE IF NOT EXISTS data_imports (
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
            sync_token TEXT,
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
            consecutive_failures INTEGER NOT NULL DEFAULT 0,
            circuit_state TEXT NOT NULL DEFAULT 'closed',
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
            idempotency_key TEXT,
            next_retry_at INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(endpoint_id) REFERENCES webhook_endpoints(id)
        );
        CREATE INDEX IF NOT EXISTS idx_webhook_logs_endpoint ON webhook_delivery_logs(endpoint_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS crdt_semantic_conflicts (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            domain TEXT NOT NULL,
            entity_table TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            colliding_entity_id TEXT,
            conflict_type TEXT NOT NULL,
            severity TEXT NOT NULL DEFAULT 'medium',
            conflict_details_json TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'flagged_for_review',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending',
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );
        CREATE INDEX IF NOT EXISTS idx_crdt_conflicts_ws ON crdt_semantic_conflicts(workspace_id, status);
        CREATE INDEX IF NOT EXISTS idx_events_semantic_user ON events(workspace_id, user_id, start_time, end_time);
        CREATE INDEX IF NOT EXISTS idx_events_semantic_assignee ON events(workspace_id, assignee_id, start_time, end_time);
        CREATE INDEX IF NOT EXISTS idx_job_tickets_vehicle ON job_tickets(workspace_id, assigned_vehicle_id, scheduled_date);
        CREATE INDEX IF NOT EXISTS idx_timetable_classroom ON timetable_slots(workspace_id, day_of_week, classroom, start_time, end_time);

        CREATE TABLE IF NOT EXISTS event_rules (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            source_block_id TEXT NOT NULL,
            target_block_id TEXT NOT NULL,
            trigger_event TEXT NOT NULL,
            action_type TEXT NOT NULL,
            config_json TEXT DEFAULT '{}',
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending',
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );

        CREATE TABLE IF NOT EXISTS event_logs (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            block_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            status TEXT NOT NULL DEFAULT 'processed',
            created_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending',
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );

        CREATE TABLE IF NOT EXISTS outbox_events (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            status TEXT NOT NULL DEFAULT 'pending',
            retry_count INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT 0,
            sync_status TEXT DEFAULT 'pending',
            FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
        );"
    )
    .await
    .map_err(|e| YntraError::DbError(e.to_string()))?;

    // Conditional migration for job_tickets columns if they do not exist
    let has_col: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM pragma_table_info('job_tickets') WHERE name = 'origin_address'",
            (),
            |r| r.get(0),
        )
        .await
        .ok();

    if has_col.is_none() {
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
    }

    Ok(())
}
