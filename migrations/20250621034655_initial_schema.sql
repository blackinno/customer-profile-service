-- Enums (idempotent via DO block — PG has no CREATE TYPE IF NOT EXISTS)
DO $$ BEGIN
    CREATE TYPE locale_enum AS ENUM ('th', 'en');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE gender_enum AS ENUM ('male', 'female', 'other', 'unspecified', 'not_to_say');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE change_type_enum AS ENUM ('telephone', 'email');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE change_type_status_enum AS ENUM (
        'pending_verify_otp',
        'verify_change_completed',
        'pending_change_top_confirmation',
        'completed'
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- Add missing enum values idempotently
DO $$ BEGIN
    ALTER TYPE gender_enum ADD VALUE IF NOT EXISTS 'unspecified';
EXCEPTION WHEN others THEN NULL; END $$;

DO $$ BEGIN
    ALTER TYPE gender_enum ADD VALUE IF NOT EXISTS 'not_to_say';
EXCEPTION WHEN others THEN NULL; END $$;

-- Tables
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email VARCHAR(100) UNIQUE,
    phone VARCHAR(100) UNIQUE,
    email_verified BOOLEAN DEFAULT FALSE,
    phone_verified BOOLEAN DEFAULT FALSE,
    locale locale_enum DEFAULT 'th',
    has_consent BOOLEAN DEFAULT FALSE,
    is_deleted BOOLEAN DEFAULT FALSE,
    client_id VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uuid UUID NOT NULL,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    birthdate DATE,
    gender gender_enum,
    profile_image VARCHAR(255),
    nationality VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_user_profiles_user FOREIGN KEY (user_uuid) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT uq_user_profiles_user_uuid UNIQUE (user_uuid)
);

CREATE TABLE IF NOT EXISTS identity_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uuid UUID NOT NULL,
    provider_name VARCHAR(100) NOT NULL,
    external_id VARCHAR(100) NOT NULL,
    provider_id_token TEXT,
    provider_access_token TEXT,
    provider_refresh_token TEXT,
    is_deleted BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_identity_providers_user FOREIGN KEY (user_uuid) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS identity_provider_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uuid UUID NOT NULL,
    action_type VARCHAR(100) NOT NULL,
    provider_name VARCHAR(100) NOT NULL,
    external_id VARCHAR(100) NOT NULL,
    deleted_date TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS profile_changes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uuid UUID NOT NULL,
    change_type change_type_enum NOT NULL,
    identifier VARCHAR(100),
    old_value VARCHAR(100),
    new_value VARCHAR(100),
    status change_type_status_enum NOT NULL,
    token TEXT,
    token_expired_at TIMESTAMPTZ NOT NULL,
    otp VARCHAR(100),
    ref_code VARCHAR(100),
    next_otp_request_at TIMESTAMPTZ NOT NULL,
    otp_expired_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_profile_changes_user FOREIGN KEY (user_uuid) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS the1_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_uuid UUID NOT NULL,
    member_id VARCHAR(100) NOT NULL,
    account_id VARCHAR(255) NOT NULL,
    profile_id VARCHAR(255) NOT NULL,
    card_number VARCHAR(32),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_the1_users_user FOREIGN KEY (user_uuid) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(100) NOT NULL,
    name VARCHAR(100),
    expired_date TIMESTAMPTZ,
    the1_users_id UUID NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_tiers_the1_users FOREIGN KEY (the1_users_id) REFERENCES the1_users(id) ON DELETE CASCADE
);

-- Columns from later Go migrations (idempotent)
ALTER TABLE identity_providers ADD COLUMN IF NOT EXISTS provider_id_token TEXT;
ALTER TABLE identity_providers ADD COLUMN IF NOT EXISTS provider_access_token TEXT;
ALTER TABLE identity_providers ADD COLUMN IF NOT EXISTS provider_refresh_token TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS client_id VARCHAR(255);
ALTER TABLE the1_users ADD COLUMN IF NOT EXISTS card_number VARCHAR(32);
ALTER TABLE user_profiles ADD COLUMN IF NOT EXISTS nationality VARCHAR(50);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_users_id ON users(id);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_phone ON users(phone);
CREATE INDEX IF NOT EXISTS idx_user_profiles_user_uuid ON user_profiles(user_uuid);
CREATE INDEX IF NOT EXISTS idx_identity_providers_user_uuid ON identity_providers(user_uuid);
CREATE INDEX IF NOT EXISTS idx_profile_changes_user_uuid ON profile_changes(user_uuid);
CREATE INDEX IF NOT EXISTS idx_profile_changes_otp ON profile_changes(otp);
CREATE INDEX IF NOT EXISTS idx_the1_users_user_uuid ON the1_users(user_uuid);
CREATE INDEX IF NOT EXISTS idx_tiers_id ON tiers(id);
CREATE INDEX IF NOT EXISTS idx_tiers_the1_users_id ON tiers(the1_users_id);
