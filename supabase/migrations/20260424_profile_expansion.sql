-- Migration: Profile Expansion & Privacy Settings
ALTER TABLE public.users 
ADD COLUMN IF NOT EXISTS location TEXT,
ADD COLUMN IF NOT EXISTS privacy_settings JSONB DEFAULT '{"phone": "organization", "location": "organization"}'::jsonb;

-- Comment for clarity
COMMENT ON COLUMN public.users.privacy_settings IS 'Privacy visibility for profile fields. Options: everyone, organization, none';
