-- Promote selected email addresses to platform_admin during Supabase Auth signup/login sync.
-- Run this in the Supabase SQL Editor or via your migration workflow.

create schema if not exists private;

alter table if exists public.users
  drop constraint if exists only_one_platform_admin;

drop index if exists public.only_one_platform_admin;

create table if not exists public.platform_admin_allowlist (
  email text primary key,
  created_at timestamp with time zone not null default now(),
  constraint platform_admin_allowlist_email_lowercase check (email = lower(email))
);

alter table public.platform_admin_allowlist enable row level security;

create or replace function private.is_platform_admin_email(candidate_email text)
returns boolean
language sql
security definer
set search_path = ''
stable
as $$
  select exists (
    select 1
    from public.platform_admin_allowlist pal
    where pal.email = lower(candidate_email)
  );
$$;

create or replace function private.resolve_user_full_name(raw_meta jsonb, fallback_email text)
returns text
language sql
security definer
set search_path = ''
stable
as $$
  select coalesce(
    nullif(trim(raw_meta ->> 'full_name'), ''),
    nullif(trim(raw_meta ->> 'name'), ''),
    nullif(trim(raw_meta ->> 'user_name'), ''),
    fallback_email
  );
$$;

create or replace function private.sync_public_user_from_auth()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  resolved_email text;
  resolved_role text;
  resolved_full_name text;
begin
  resolved_email := lower(new.email);
  resolved_role := case
    when private.is_platform_admin_email(resolved_email) then 'platform_admin'
    else 'user'
  end;
  resolved_full_name := private.resolve_user_full_name(new.raw_user_meta_data, resolved_email);

  insert into public.users (id, email, full_name, role)
  values (new.id, resolved_email, resolved_full_name, resolved_role)
  on conflict (id) do update
  set email = excluded.email,
      full_name = coalesce(excluded.full_name, public.users.full_name),
      role = case
        when private.is_platform_admin_email(excluded.email) then 'platform_admin'
        when public.users.role = 'platform_admin' then public.users.role
        else coalesce(public.users.role, excluded.role)
      end;

  return new;
end;
$$;

drop trigger if exists on_auth_user_synced on auth.users;

create trigger on_auth_user_synced
  after insert or update of email, raw_user_meta_data
  on auth.users
  for each row execute procedure private.sync_public_user_from_auth();

create or replace function private.sync_platform_admin_allowlist()
returns integer
language plpgsql
security definer
set search_path = ''
as $$
declare
  affected integer;
begin
  update public.users
  set role = 'platform_admin',
      email = lower(public.users.email)
  where lower(public.users.email) in (
    select email from public.platform_admin_allowlist
  );

  get diagnostics affected = row_count;
  return affected;
end;
$$;

insert into public.platform_admin_allowlist (email)
values
  ('reeeeboi2.0@gmail.com'),
  ('nicolaj.andersson05@gmail.com')
on conflict (email) do nothing;

select private.sync_platform_admin_allowlist();
