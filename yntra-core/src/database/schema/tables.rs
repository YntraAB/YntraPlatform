use crate::YntraError;
use super::super::DbConnection;

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
            challenge TEXT
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
            FOREIGN KEY(assigned_user_id) REFERENCES users(id)
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



        -- Indices
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
        CREATE INDEX IF NOT EXISTS idx_entities_block ON entities(workspace_id, block_id);"
    )
    .await
    .map_err(|e| YntraError::DbError(e.to_string()))?;

    Ok(())
}
