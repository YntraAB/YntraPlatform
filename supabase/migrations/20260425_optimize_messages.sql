-- Optimization for Messages table
-- Run this in your Supabase SQL Editor

-- 1. Indexes for faster filtering and joins
CREATE INDEX IF NOT EXISTS idx_messages_workspace_id ON messages(workspace_id);
CREATE INDEX IF NOT EXISTS idx_messages_sender_id ON messages(sender_id);
CREATE INDEX IF NOT EXISTS idx_messages_receiver_id ON messages(receiver_id);
CREATE INDEX IF NOT EXISTS idx_messages_target_team_id ON messages(target_team_id);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_is_read ON messages(is_read) WHERE is_read = false;

-- 2. Full Text Search Optimization
-- Add a generated column for search
ALTER TABLE messages ADD COLUMN IF NOT EXISTS fts tsvector 
GENERATED ALWAYS AS (to_tsvector('swedish', coalesce(subject, '') || ' ' || coalesce(body, ''))) STORED;

-- Index the search column
CREATE INDEX IF NOT EXISTS idx_messages_fts ON messages USING GIN(fts);

-- 3. Optimization for Users table (used in joins)
CREATE INDEX IF NOT EXISTS idx_users_workspace_id ON users(workspace_id);
