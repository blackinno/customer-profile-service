# Customer Profile Service — Rust Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite `cx-customer-profile-service` (Go/Fiber) to Rust/Axum at `~/Projects/customer-profiles/`, achieving full feature parity with the same HTTP paths, auth header, and JSON field names.

**Architecture:** Hexagonal/ports-and-adapters across four Cargo workspace crates (`domain`, `application`, `infrastructure`, `api`). Each domain is implemented in full-vertical-slice order (entity → repo trait → sqlx impl → use case → handler → route) before the next. Per-domain gate: use-case unit tests 100% + handler integration tests green.

**Tech Stack:** Rust/Axum 0.8, sqlx 0.8 (QueryBuilder only — no `query!` macros), PostgreSQL, reqwest 0.12, aws-sdk-s3, rsa 0.9 (CloudFront), jsonwebtoken 9, rand 0.8, wiremock (integration stub), cargo-llvm-cov.

## Global Constraints

- Axum 0.8.6, sqlx 0.8.6 (workspace versions from template — do not change)
- All DB queries use `sqlx::QueryBuilder` — never `query!`, `query_as!`, or offline snapshots
- Migration created via `sqlx migrate add initial_schema` (use the sqlx-generated filename)
- All Go env var names preserved exactly (see Settings task)
- HTTP paths/methods/`user_uuid` header/JSON field names identical to Go service
- Response envelope is template's `ApiResponse<T>` `{ success, data, message }` — acceptable divergence from Go's raw struct
- HTTP status codes preserved exactly; error message strings may change
- `ApplicationError` extended with `BadRequest(String)` → 400 and `External(String)` → 502
- Coverage: application layer 100%, overall ≥ 85%; measured with `cargo-llvm-cov`
- Mock repositories are hand-written (no `mockall`) — follow template's `Arc<Mutex<HashMap>>` pattern
- Integration tests use `tower::ServiceExt::oneshot` — no real server started
- External services in integration tests stubbed with `wiremock`
- SNS is feature-gated behind `sns` feature (already in template)

---

## File Structure

```
customer-profiles/
├── Cargo.toml                          # workspace; binary: customer-profile-service
├── cmd/main.rs                         # entrypoint (from template, unchanged)
├── migrations/
│   └── {sqlx-timestamp}_initial_schema.sql
├── tests/
│   ├── integration/
│   │   ├── helpers.rs                  # create_test_app(), send_request()
│   │   ├── customers/
│   │   │   └── customer_controller_test.rs
│   │   ├── identities/
│   │   │   └── identity_controller_test.rs
│   │   ├── profile_changes/
│   │   │   └── profile_change_controller_test.rs
│   │   ├── profile_images/
│   │   │   └── profile_image_controller_test.rs
│   │   ├── segments/
│   │   │   └── segment_controller_test.rs
│   │   └── the1/
│   │       └── the1_controller_test.rs
│   └── unit_tests/
│       ├── applications/
│       │   ├── customer_use_cases_test.rs
│       │   ├── identity_use_cases_test.rs
│       │   ├── profile_change_use_cases_test.rs
│       │   ├── profile_image_use_cases_test.rs
│       │   ├── segment_use_cases_test.rs
│       │   └── the1_use_cases_test.rs
│       └── domain/
├── crates/
│   ├── domain/src/
│   │   ├── entities/
│   │   │   ├── customer.rs             # Customer, CreateCustomer, UpdateCustomer
│   │   │   ├── identity.rs             # Identity, CreateIdentity, InvokeTokenRequest
│   │   │   ├── profile_change.rs       # ProfileChange, CreateProfileChangeRequest, etc.
│   │   │   ├── the1_user.rs            # The1User, Tier, The1Profile
│   │   │   └── segment.rs              # Segment
│   │   ├── repositories/
│   │   │   ├── customer_repository.rs
│   │   │   ├── identity_repository.rs
│   │   │   ├── profile_change_repository.rs
│   │   │   └── the1_user_repository.rs
│   │   └── errors.rs                   # keep as-is from template
│   ├── application/src/
│   │   ├── errors.rs                   # add BadRequest + External variants
│   │   ├── repositories.rs             # extend Repositories struct
│   │   ├── use_cases.rs                # extend UseCases struct
│   │   ├── customers/
│   │   │   ├── mod.rs
│   │   │   ├── dtos.rs
│   │   │   └── use_cases.rs
│   │   ├── identities/
│   │   ├── profile_changes/
│   │   ├── profile_images/
│   │   ├── segments/
│   │   └── the1/
│   ├── infrastructure/src/
│   │   ├── persistence/
│   │   │   ├── pg_customer_repository.rs
│   │   │   ├── pg_identity_repository.rs
│   │   │   ├── pg_profile_change_repository.rs
│   │   │   └── pg_the1_user_repository.rs
│   │   ├── external/
│   │   │   ├── the1_client.rs          # reqwest: get_profile, invoke_token, get_partner_member
│   │   │   └── sms_client.rs           # reqwest: send OTP SMS
│   │   ├── storage/
│   │   │   ├── s3.rs                   # aws-sdk-s3 upload/delete
│   │   │   └── cloudfront_signer.rs    # RSA PKCS#1 v1.5 signed URL
│   │   ├── messaging/sns.rs            # keep from template
│   │   ├── utils/
│   │   │   ├── otp.rs                  # 6-digit OTP + ref code (rand)
│   │   │   └── jwt.rs                  # JWT gen/validate (jsonwebtoken)
│   │   ├── configuration/settings.rs   # extend with all project env vars
│   │   └── stages/factory.rs           # AppFactoryState wiring (all domains)
│   └── api/src/
│       ├── handlers/
│       │   ├── customers.rs
│       │   ├── identities.rs
│       │   ├── profile_changes.rs
│       │   ├── profile_images.rs
│       │   ├── segments.rs
│       │   └── the1.rs
│       ├── middleware/
│       │   └── user_uuid.rs            # UserUuid Axum extractor
│       ├── routers.rs                  # extend with all domain routes
│       └── docs.rs                     # utoipa OpenAPI
```

---

## Phase 0 — Bootstrap

### Task 1: Copy template, rename, clean, add dependencies

**Files:**
- Create: `~/Projects/customer-profiles/` (full copy of template, minus `.git`)
- Modify: `Cargo.toml` (root)
- Delete: all `users` example code from all crates

- [ ] **Step 1: Copy template into project directory**

```bash
rsync -av --exclude='.git' \
  ~/cpn/core/templates/axum-rust-template/ \
  ~/Projects/customer-profiles/
cd ~/Projects/customer-profiles
```

- [ ] **Step 2: Rename binary in root Cargo.toml**

In `Cargo.toml`, change:
```toml
[package]
name = "axum-rust-template"
# ...
[[bin]]
name = "axum-rust-template"
```
To:
```toml
[package]
name = "customer-profile-service"
# ...
[[bin]]
name = "customer-profile-service"
path = "cmd/main.rs"
```

- [ ] **Step 3: Add new workspace dependencies**

In `[workspace.dependencies]` section of root `Cargo.toml`, add:
```toml
reqwest = { version = "0.12", features = ["json", "multipart"] }
aws-sdk-s3 = "1"
rsa = { version = "0.9", features = ["pkcs1"] }
jsonwebtoken = "9"
rand = "0.8"
wiremock = "0.6"
```

In `[dev-dependencies]` of root `Cargo.toml`, add:
```toml
wiremock = { workspace = true }
```

Add to `crates/infrastructure/Cargo.toml` under `[dependencies]`:
```toml
reqwest = { workspace = true }
aws-sdk-s3 = { workspace = true }
rsa = { workspace = true }
jsonwebtoken = { workspace = true }
rand = { workspace = true }
```

- [ ] **Step 4: Remove the template's users example**

Delete files:
```
crates/domain/src/entities/user.rs
crates/domain/src/repositories/user_repository.rs
crates/application/src/users/        (entire directory)
crates/infrastructure/src/users/      (entire directory)
crates/api/src/users/                (entire directory, or handlers/users.rs)
```

Update `mod.rs` files in each crate to remove the `users` module references. Update `crates/application/src/repositories.rs` to remove `users` field. Update `crates/application/src/use_cases.rs` to remove `users` field. Update `crates/infrastructure/src/stages/factory.rs` to remove user wiring.

- [ ] **Step 5: Verify it compiles**

```bash
cargo check --no-default-features
```
Expected: no errors (warnings OK)

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: bootstrap from axum-rust-template, remove example users module"
```

---

### Task 2: Extend Settings with all environment variables

**Files:**
- Modify: `crates/infrastructure/src/configuration/settings.rs`

- [ ] **Step 1: Replace Settings struct with full set of env vars**

```rust
#[derive(Clone, Deserialize)]
pub struct Settings {
    // From template (keep these)
    pub database_url: String,
    pub qml_database_url: String,
    pub qml_worker_count: usize,
    pub qml_batch_size: usize,
    pub qml_retry_max_attempts: u32,
    pub qml_retry_base_seconds: u32,
    pub qml_retry_multiplier: f64,
    pub qml_retry_max_seconds: u32,
    pub aws_region: String,
    pub server_host: String,
    pub server_port: u16,
    // AWS S3 / CloudFront
    pub s3_profile_bucket: String,
    pub cloudfront_base_endpoint: String,
    pub cloudfront_private_key: String,   // PEM string
    pub cloudfront_key_id: String,
    pub image_expired_in_sec: u32,
    // SNS topic ARNs
    pub sns_user_profile_changed: String,
    pub sns_email_sent_requested: String,
    pub sns_user_identity_linked_changed: String,
    pub sns_user_the1_get_profile_updated: String,
    // The1
    pub the1_proxy_service_url: String,
    // SMS
    pub sms_proxy_service_url: String,
    pub phone_number_format: String,
    pub otp_text: String,
    pub otp_expired_time: u32,           // minutes
    // Auth / misc
    pub jwt_secret_key: String,
    pub country_code: String,
    pub profile_change_expired_time: u32, // minutes
    pub token_expired_time: u32,          // minutes
    pub allow_image_types: Vec<String>,   // e.g. ["image/jpeg","image/png"]
    pub max_image_size_mb: u32,
    pub image_prefix: String,
}
```

Update `from_env()` to read all new vars using `required_var()` / `var()` / `parse_var()`. For `allow_image_types`, parse from a comma-separated env var `ALLOW_IMAGE_TYPES`.

- [ ] **Step 2: Verify**

```bash
cargo check --no-default-features
```
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/infrastructure/src/configuration/settings.rs
git commit -m "feat: extend Settings with all service env vars"
```

---

### Task 3: Write idempotent migration

**Files:**
- Create: `migrations/{sqlx-timestamp}_initial_schema.sql`

- [ ] **Step 1: Init sqlx migration**

```bash
sqlx migrate add initial_schema
```
Expected: creates `migrations/{timestamp}_initial_schema.sql`

- [ ] **Step 2: Write the consolidated idempotent SQL**

Paste this content into the generated file:

```sql
-- Enums (idempotent via DO block because PG has no CREATE TYPE IF NOT EXISTS)
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

-- Add missing enum values (idempotent — PG ignores duplicate value errors)
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
```

- [ ] **Step 3: Run against dev database to verify**

```bash
export DATABASE_URL="postgres://..."   # your local dev URL
sqlx migrate run
```
Expected: `Applied {timestamp}/initial_schema` (or `No pending migrations` if already applied to Go DB)

- [ ] **Step 4: Commit**

```bash
git add migrations/
git commit -m "feat: add idempotent initial_schema migration consolidating all Go migrations"
```

---

## Phase 1 — Customers

### Task 4: Customer domain entities + CustomerRepository trait

**Files:**
- Create: `crates/domain/src/entities/customer.rs`
- Create: `crates/domain/src/repositories/customer_repository.rs`
- Modify: `crates/domain/src/lib.rs` (add mod declarations)

**Interfaces — Produces:**
- `Customer`, `CreateCustomer`, `UpdateCustomer`, `CustomerProfile` structs
- `CustomerRepository` async trait

- [ ] **Step 1: Write customer.rs**

```rust
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum Locale { Th, En }

#[derive(Debug, Clone)]
pub enum Gender { Male, Female, Other, Unspecified, NotToSay }

#[derive(Debug, Clone)]
pub struct Customer {
    pub id: Uuid,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub email_verified: bool,
    pub phone_verified: bool,
    pub locale: Locale,
    pub has_consent: bool,
    pub is_deleted: bool,
    pub client_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile: Option<CustomerProfile>,
}

#[derive(Debug, Clone)]
pub struct CustomerProfile {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub gender: Option<Gender>,
    pub profile_image: Option<String>,
    pub nationality: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateCustomer {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub locale: Option<Locale>,
    pub has_consent: Option<bool>,
    pub client_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub gender: Option<Gender>,
    pub nationality: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateCustomer {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub locale: Option<Locale>,
    pub has_consent: Option<bool>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub gender: Option<Gender>,
    pub nationality: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SearchField {
    Id(Uuid),
    Phone(String),
    The1MemberId(String),
    The1CardNumber(String),
}
```

- [ ] **Step 2: Write customer_repository.rs**

```rust
use async_trait::async_trait;
use uuid::Uuid;
use crate::entities::customer::{Customer, CreateCustomer, UpdateCustomer, SearchField};
use crate::errors::RepositoryError;

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn create(&self, data: CreateCustomer) -> Result<Customer, RepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Customer>, RepositoryError>;
    async fn find_by_phone(&self, phone: &str) -> Result<Option<Customer>, RepositoryError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<Customer>, RepositoryError>;
    async fn search(&self, field: SearchField) -> Result<Vec<Customer>, RepositoryError>;
    async fn update(&self, id: Uuid, data: UpdateCustomer) -> Result<Customer, RepositoryError>;
    async fn soft_delete(&self, id: Uuid) -> Result<Customer, RepositoryError>;
    async fn update_profile_image(&self, user_uuid: Uuid, image_key: Option<String>) -> Result<(), RepositoryError>;
}
```

- [ ] **Step 3: Update domain lib.rs**

Add `pub mod customers;` (or inline `mod` entries) in `crates/domain/src/entities/mod.rs` and `crates/domain/src/repositories/mod.rs`.

- [ ] **Step 4: Verify**

```bash
cargo check --no-default-features -p domain
```
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/entities/customer.rs crates/domain/src/repositories/customer_repository.rs
git commit -m "feat(domain): add Customer entities and CustomerRepository trait"
```

---

### Task 5: PgCustomerRepository (sqlx QueryBuilder)

**Files:**
- Create: `crates/infrastructure/src/persistence/pg_customer_repository.rs`

**Interfaces — Consumes:** `CustomerRepository` trait, `Customer`, `CreateCustomer`, `UpdateCustomer`, `SearchField` from domain.

- [ ] **Step 1: Write pg_customer_repository.rs**

Key patterns (full impl not shown — follow template's `pg_repository.rs`):

```rust
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;
// ...

#[derive(FromRow)]
struct CustomerRow {
    // users table columns
    id: Uuid,
    email: Option<String>,
    phone: Option<String>,
    email_verified: bool,
    phone_verified: bool,
    locale: String,      // "th" | "en"
    has_consent: bool,
    is_deleted: bool,
    client_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // user_profiles columns (LEFT JOIN, all nullable)
    profile_id: Option<Uuid>,
    first_name: Option<String>,
    last_name: Option<String>,
    birthdate: Option<NaiveDate>,
    gender: Option<String>,
    profile_image: Option<String>,
    nationality: Option<String>,
    profile_created_at: Option<DateTime<Utc>>,
    profile_updated_at: Option<DateTime<Utc>>,
}

// SELECT with LEFT JOIN user_profiles is the standard query for all find methods.
// create() inserts into users AND user_profiles in a transaction.
// update() updates both tables.
// soft_delete() sets is_deleted=true and appends "-deleted-{uuid}" suffix to phone/email
//   (matching Go behaviour: prevents uniqueness constraint violation on re-registration).
```

Implement `CustomerRepository` for `PgCustomerRepository`. Use `QueryBuilder::new(...)`, `push_bind(...)`, `build_query_as()`, `fetch_one/fetch_optional/fetch_all`.

The `search()` method switches on `SearchField` variant:
- `Id(id)` → `WHERE u.id = ?`
- `Phone(p)` → `WHERE u.phone = ?`
- `The1MemberId(m)` → `JOIN the1_users t1 ON t1.user_uuid = u.id WHERE t1.member_id = ?`
- `The1CardNumber(c)` → `JOIN the1_users t1 ON t1.user_uuid = u.id WHERE t1.card_number = ?`

- [ ] **Step 2: Verify**

```bash
cargo check --no-default-features -p infrastructure
```

- [ ] **Step 3: Commit**

```bash
git add crates/infrastructure/src/persistence/pg_customer_repository.rs
git commit -m "feat(infra): add PgCustomerRepository with QueryBuilder"
```

---

### Task 6: CustomerUseCases + unit tests (100% coverage)

**Files:**
- Create: `crates/application/src/customers/dtos.rs`
- Create: `crates/application/src/customers/use_cases.rs`
- Modify: `crates/application/src/errors.rs` (add `BadRequest` and `External`)
- Modify: `crates/application/src/repositories.rs` (add `customers` field)
- Create: `tests/unit_tests/applications/customer_use_cases_test.rs`

**Interfaces — Produces:** `CustomerUseCases { create, get_by_id, search, get_me, update_me, delete }`

- [ ] **Step 1: Extend ApplicationError**

In `crates/application/src/errors.rs`, add:
```rust
#[error("Bad request: {0}")]
BadRequest(String),

#[error("External service error: {0}")]
External(String),
```

Also add `IntoResponse` impl for `ApplicationError` in `crates/api/src/` (or in `application/src/errors.rs` if you place it there) mapping:
- `NotFound` → 404
- `ValidationError` → 422
- `BadRequest | BusinessRuleViolation` → 400
- `Repository(Conflict)` → 409
- `Repository(NotFound)` → 404
- `External` → 502
- everything else → 500

- [ ] **Step 2: Write DTOs**

```rust
// crates/application/src/customers/dtos.rs
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub locale: Option<String>,
    pub has_consent: Option<bool>,
    pub client_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<String>,   // "YYYY-MM-DD"
    pub gender: Option<String>,
    pub nationality: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCustomerRequest {
    pub email: Option<String>,
    pub locale: Option<String>,
    pub has_consent: Option<bool>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<String>,
    pub gender: Option<String>,
    pub nationality: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CustomerResponse {
    pub id: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub email_verified: bool,
    pub phone_verified: bool,
    pub locale: String,
    pub has_consent: bool,
    pub is_deleted: bool,
    pub client_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<String>,
    pub gender: Option<String>,
    pub profile_image: Option<String>,
    pub nationality: Option<String>,
}
```

- [ ] **Step 3: Write CustomerUseCases**

Key business logic (matching Go):
- `create()`: normalize phone (prepend country code, strip leading 0), check email/phone uniqueness (→ `BadRequest`), insert
- `get_by_id()`: returns `NotFound` if absent or `is_deleted`
- `search()`: delegates to repo; returns empty vec if nothing found
- `get_me(user_uuid)`: `find_by_id`, return `NotFound` if deleted
- `update_me(user_uuid, req)`: check email uniqueness if email changes (→ `BadRequest`), update
- `delete(id)`: soft_delete via repo

```rust
pub struct CustomerUseCases {
    customers: Arc<dyn CustomerRepository>,
    settings: Arc<Settings>,
}

impl CustomerUseCases {
    pub fn new(customers: Arc<dyn CustomerRepository>, settings: Arc<Settings>) -> Self {
        Self { customers, settings }
    }
    // ...methods
}
```

- [ ] **Step 4: Write unit tests**

Mock pattern (follow template exactly):
```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct MockCustomerRepository {
    store: Arc<Mutex<HashMap<Uuid, Customer>>>,
    should_fail: bool,
}

impl MockCustomerRepository {
    fn new() -> Self { Self { store: Default::default(), should_fail: false } }
    fn with_failure() -> Self { Self { should_fail: true, ..Self::new() } }
    fn seed(&self, c: Customer) { self.store.lock().unwrap().insert(c.id, c); }
}

#[async_trait]
impl CustomerRepository for MockCustomerRepository {
    async fn create(&self, data: CreateCustomer) -> Result<Customer, RepositoryError> {
        if self.should_fail { return Err(RepositoryError::Backend("mock failure".into())); }
        // build Customer from data, insert into store
        // ...
    }
    // ... all other methods
}
```

Test cases must reach 100% branch coverage of `CustomerUseCases`:
- `create` happy path, duplicate email → `BadRequest`, repo error → propagated
- `get_by_id` found, not found, deleted → `NotFound`
- `search` results, empty
- `get_me` found, not found
- `update_me` happy path, email conflict → `BadRequest`
- `delete` happy path, not found

- [ ] **Step 5: Run unit tests (must be 100% for CustomerUseCases)**

```bash
cargo test -p customer-profile-service tests::unit_tests::applications::customer -- --nocapture
```
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add crates/application/src/ tests/unit_tests/applications/customer_use_cases_test.rs
git commit -m "feat(app): CustomerUseCases with 100% unit test coverage"
```

---

### Task 7: Customer handlers, routes, integration tests

**Files:**
- Create: `crates/api/src/handlers/customers.rs`
- Create: `crates/api/src/middleware/user_uuid.rs`
- Modify: `crates/api/src/routers.rs`
- Create: `tests/integration/helpers.rs`
- Create: `tests/integration/customers/customer_controller_test.rs`

**Interfaces — Consumes:** `CustomerUseCases`, `AppState`, `ApiResponse`

- [ ] **Step 1: Write UserUuid extractor**

```rust
// crates/api/src/middleware/user_uuid.rs
use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;
use application::errors::ApplicationError;

pub struct UserUuid(pub Uuid);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for UserUuid {
    type Rejection = ApplicationError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let raw = parts
            .headers
            .get("user_uuid")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApplicationError::BadRequest("missing user_uuid header".into()))?;
        let id = Uuid::parse_str(raw)
            .map_err(|_| ApplicationError::BadRequest("invalid user_uuid header".into()))?;
        Ok(UserUuid(id))
    }
}
```

- [ ] **Step 2: Write customer handlers**

```rust
// crates/api/src/handlers/customers.rs
pub async fn create_customer(
    State(state): State<AppState>,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<impl IntoResponse, ApplicationError> {
    let customer = state.use_cases.customers.create(body).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(CustomerResponse::from(customer)))))
}

pub async fn get_me(
    State(state): State<AppState>,
    UserUuid(user_uuid): UserUuid,
) -> Result<impl IntoResponse, ApplicationError> {
    let customer = state.use_cases.customers.get_me(user_uuid).await?;
    Ok(Json(ApiResponse::success(CustomerResponse::from(customer))))
}

// search, get_by_id, update_me, delete follow same pattern
```

- [ ] **Step 3: Wire routes in routers.rs**

```rust
// In Routers::init_routers(), add customer routes:
let me_routes = Router::new()
    .route("/me", get(get_me).patch(update_me))
    .route_layer(/* UserUuid is extracted per-handler, no layer needed */);

let customers = Router::new()
    .route("/v1/customers", post(create_customer).get(search_customers))
    .route("/v1/customers/profiles/:id", get(get_customer_by_id))
    .route("/v1/customers/:id", delete(delete_customer))
    .route("/v1/customers/me", get(get_me).patch(update_me))
    .with_state(state.clone());
```

- [ ] **Step 4: Write integration test helpers**

```rust
// tests/integration/helpers.rs
use axum::Router;
use tower::ServiceExt;
use axum::http::{Request, StatusCode};
use axum::body::Body;

pub fn create_test_app(state: AppState) -> Router {
    Routers::init_routers(state)
}

pub async fn send_request(app: Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
    (status, json)
}
```

- [ ] **Step 5: Write customer integration tests**

```rust
// tests/integration/customers/customer_controller_test.rs
#[tokio::test]
async fn test_create_customer_returns_201() {
    let state = build_test_state_with_mock_customers();
    let app = create_test_app(state);
    let req = Request::builder()
        .method("POST")
        .uri("/v1/customers")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"phone":"0812345678"}"#))
        .unwrap();
    let (status, body) = send_request(app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_get_me_missing_header_returns_400() { /* ... */ }

#[tokio::test]
async fn test_get_me_not_found_returns_404() { /* ... */ }
```

- [ ] **Step 6: Run integration tests**

```bash
cargo test -p customer-profile-service tests::integration::customers -- --nocapture
```
Expected: all pass

- [ ] **Step 7: Commit**

```bash
git add crates/api/src/ tests/integration/
git commit -m "feat(api): customer handlers, routes, integration tests"
```

---

## Phase 2 — Identities

### Task 8: Identity domain entities + IdentityRepository trait

**Files:**
- Create: `crates/domain/src/entities/identity.rs`
- Create: `crates/domain/src/repositories/identity_repository.rs`

**Interfaces — Produces:**
```rust
pub struct Identity {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub provider_name: String,
    pub external_id: String,
    pub provider_id_token: Option<String>,
    pub provider_access_token: Option<String>,
    pub provider_refresh_token: Option<String>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateIdentity {
    pub user_uuid: Uuid,
    pub provider_name: String,
    pub external_id: String,
    pub provider_id_token: Option<String>,
    pub provider_access_token: Option<String>,
    pub provider_refresh_token: Option<String>,
}

#[async_trait]
pub trait IdentityRepository: Send + Sync {
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Vec<Identity>, RepositoryError>;
    async fn find_active(&self, user_uuid: Uuid, provider: &str, external_id: &str) -> Result<Option<Identity>, RepositoryError>;
    async fn find_deleted(&self, provider: &str, external_id: &str) -> Result<Option<Identity>, RepositoryError>;
    async fn create(&self, data: CreateIdentity) -> Result<Identity, RepositoryError>;
    async fn restore(&self, id: Uuid, user_uuid: Uuid, tokens: CreateIdentity) -> Result<Identity, RepositoryError>;
    async fn soft_delete(&self, id: Uuid, user_uuid: Uuid) -> Result<Identity, RepositoryError>;
    async fn update_tokens(&self, id: Uuid, access_token: Option<String>, refresh_token: Option<String>) -> Result<Identity, RepositoryError>;
    async fn log_transaction(&self, user_uuid: Uuid, action: &str, provider: &str, external_id: &str) -> Result<(), RepositoryError>;
}
```

- [ ] **Write entities + trait → verify → commit**

```bash
cargo check --no-default-features -p domain
git add crates/domain/src/entities/identity.rs crates/domain/src/repositories/identity_repository.rs
git commit -m "feat(domain): add Identity entities and IdentityRepository trait"
```

---

### Task 9: PgIdentityRepository

**Files:**
- Create: `crates/infrastructure/src/persistence/pg_identity_repository.rs`

Key: `find_deleted()` searches including soft-deleted rows. `restore()` relinks a deleted identity to a new user with new tokens (matching Go's reassignment logic). `log_transaction()` inserts into `identity_provider_transactions`.

- [ ] **Write → verify → commit**

```bash
cargo check --no-default-features -p infrastructure
git add crates/infrastructure/src/persistence/pg_identity_repository.rs
git commit -m "feat(infra): PgIdentityRepository"
```

---

### Task 10: IdentityUseCases + unit tests (100% coverage)

**Files:**
- Create: `crates/application/src/identities/dtos.rs`
- Create: `crates/application/src/identities/use_cases.rs`
- Create: `tests/unit_tests/applications/identity_use_cases_test.rs`

Key business logic (matching Go `identity.usecase.go`):
- `create_identity(user_uuid, req)`: check for active identity → `BadRequest("already linked")`; check for deleted identity under different user → reassign (restore); check for deleted identity under same user → restore; otherwise create new. Then publish `SNS_USER_IDENTITY_LINKED_CHANGED`.
- `delete_identity(user_uuid, provider, external_id)`: soft_delete + log_transaction + publish SNS.
- `invoke_token(user_uuid, provider)`: call The1 HTTP client `invoke_token()`, update tokens in repo, call `sync_user_identity()` which fetches The1 profile and publishes `SNS_USER_IDENTITY_LINKED_CHANGED`.
- `get_identities(user_uuid)`: list active identities.
- `get_identities_internal(user_uuid)`: same, for internal route.

```rust
pub struct IdentityUseCases {
    identities: Arc<dyn IdentityRepository>,
    customers: Arc<dyn CustomerRepository>,
    #[cfg(feature = "sns")]
    sns: Arc<AwsSns>,
    sns_identity_linked_topic: String,
    the1_client: Arc<The1Client>,
}
```

- [ ] **Write use cases + 100% unit tests → run → commit**

```bash
cargo test -p customer-profile-service tests::unit_tests::applications::identity -- --nocapture
git add crates/application/src/identities/ tests/unit_tests/applications/identity_use_cases_test.rs
git commit -m "feat(app): IdentityUseCases with 100% unit test coverage"
```

---

### Task 11: Identity handlers, routes, integration tests

Routes:
- `GET /v1/customers/me/identities` — `get_my_identities` (requires `UserUuid`)
- `POST /v1/customers/me/identities` — `create_identity`
- `DELETE /v1/customers/me/identities/:provider/:identity_id` — `delete_identity`
- `POST /v1/customers/me/identities/:provider_name/invoke` — `invoke_token`
- `GET /v1/customers/:user_uuid/identities` — `get_identities_internal` (no auth)

- [ ] **Write handlers → wire routes → write integration tests → run → commit**

```bash
cargo test -p customer-profile-service tests::integration::identities -- --nocapture
git add crates/api/src/handlers/identities.rs tests/integration/identities/
git commit -m "feat(api): identity handlers, routes, integration tests"
```

---

## Phase 3 — Profile Changes

### Task 12: OTP/JWT utilities + SMS client

**Files:**
- Create: `crates/infrastructure/src/utils/otp.rs`
- Create: `crates/infrastructure/src/utils/jwt.rs`
- Create: `crates/infrastructure/src/external/sms_client.rs`

- [ ] **Step 1: otp.rs**

```rust
use rand::Rng;

pub fn generate_otp() -> String {
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..=999999))
}

pub fn generate_ref_code() -> String {
    let mut rng = rand::thread_rng();
    let chars: String = (0..6)
        .map(|_| char::from(rng.gen_range(b'A'..=b'Z')))
        .collect();
    chars
}
```

- [ ] **Step 2: jwt.rs**

```rust
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ProfileChangeClaims {
    pub sub: String,       // profile_change_id
    pub user_uuid: String,
    pub exp: usize,
}

pub fn generate_token(claims: &ProfileChangeClaims, secret: &str) -> anyhow::Result<String> {
    encode(&Header::default(), claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(Into::into)
}

pub fn validate_token(token: &str, secret: &str) -> anyhow::Result<ProfileChangeClaims> {
    decode::<ProfileChangeClaims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|d| d.claims)
        .map_err(Into::into)
}
```

- [ ] **Step 3: sms_client.rs**

```rust
pub struct SmsClient {
    http: reqwest::Client,
    proxy_url: String,
}

impl SmsClient {
    pub fn new(proxy_url: String) -> Self {
        Self { http: reqwest::Client::new(), proxy_url }
    }

    pub async fn send(&self, phone: &str, message: &str) -> Result<(), String> {
        self.http
            .post(&self.proxy_url)
            .json(&serde_json::json!({ "phone": phone, "message": message }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
```

- [ ] **Verify → commit**

```bash
cargo check --no-default-features -p infrastructure
git add crates/infrastructure/src/utils/ crates/infrastructure/src/external/sms_client.rs
git commit -m "feat(infra): OTP utils, JWT utils, SMS client"
```

---

### Task 13: ProfileChange domain entities + trait

**Files:**
- Create: `crates/domain/src/entities/profile_change.rs`
- Create: `crates/domain/src/repositories/profile_change_repository.rs`

**Interfaces — Produces:**
```rust
pub enum ChangeType { Telephone, Email }
pub enum ChangeStatus {
    PendingVerifyOtp,
    VerifyChangeCompleted,
    PendingChangeTopConfirmation,
    Completed,
}

pub struct ProfileChange {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub change_type: ChangeType,
    pub identifier: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub status: ChangeStatus,
    pub token: Option<String>,
    pub token_expired_at: DateTime<Utc>,
    pub otp: Option<String>,
    pub ref_code: Option<String>,
    pub next_otp_request_at: DateTime<Utc>,
    pub otp_expired_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait ProfileChangeRepository: Send + Sync {
    async fn create(&self, data: CreateProfileChange) -> Result<ProfileChange, RepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ProfileChange>, RepositoryError>;
    async fn find_active_by_user_and_type(&self, user_uuid: Uuid, change_type: ChangeType) -> Result<Option<ProfileChange>, RepositoryError>;
    async fn update_otp(&self, id: Uuid, otp: String, ref_code: String, expires: DateTime<Utc>, next_request: DateTime<Utc>) -> Result<ProfileChange, RepositoryError>;
    async fn update_status_and_token(&self, id: Uuid, status: ChangeStatus, token: Option<String>, token_expires: Option<DateTime<Utc>>) -> Result<ProfileChange, RepositoryError>;
}
```

- [ ] **Write → verify → commit**

```bash
git add crates/domain/src/entities/profile_change.rs crates/domain/src/repositories/profile_change_repository.rs
git commit -m "feat(domain): ProfileChange entities and trait"
```

---

### Task 14: PgProfileChangeRepository

**Files:**
- Create: `crates/infrastructure/src/persistence/pg_profile_change_repository.rs`

Key: Map Postgres enum strings ↔ `ChangeType`/`ChangeStatus` enums via `impl From<String>`.

- [ ] **Write → verify → commit**

```bash
git add crates/infrastructure/src/persistence/pg_profile_change_repository.rs
git commit -m "feat(infra): PgProfileChangeRepository"
```

---

### Task 15: ProfileChangeUseCases + unit tests (100% coverage)

**Files:**
- Create: `crates/application/src/profile_changes/use_cases.rs`
- Create: `tests/unit_tests/applications/profile_change_use_cases_test.rs`

Key business logic (matching Go `profile_change.usecase.go`):
- `create_profile_change(user_uuid, req)`: check for existing active change (→ `BadRequest`); generate OTP + ref_code; for telephone: normalize phone, send SMS; for email: publish `SNS_EMAIL_SENT_REQUESTED`; create record.
- `update_profile_change(user_uuid, profile_id, token)`: validate JWT token (→ `BadRequest` if expired/invalid); generate new OTP; send SMS/email again; update otp in repo.
- `verify_profile_change(user_uuid, profile_id, otp)`: check OTP match + expiry (→ `BadRequest`); generate JWT token; update status to `VerifyChangeCompleted` + store token.
- `confirm_profile_change(user_uuid, profile_id)`: find record with status `VerifyChangeCompleted`; apply new phone/email to `users` table; update status to `Completed`; publish `SNS_USER_PROFILE_CHANGED`.

```rust
pub struct ProfileChangeUseCases {
    profile_changes: Arc<dyn ProfileChangeRepository>,
    customers: Arc<dyn CustomerRepository>,
    sms: Arc<SmsClient>,
    #[cfg(feature = "sns")]
    sns: Arc<AwsSns>,
    settings: Arc<Settings>,
}
```

- [ ] **Write use cases + 100% tests → run → commit**

```bash
cargo test -p customer-profile-service tests::unit_tests::applications::profile_change -- --nocapture
git commit -m "feat(app): ProfileChangeUseCases with 100% unit test coverage"
```

---

### Task 16: ProfileChange handlers, routes, integration tests

Routes:
- `POST /v1/customers/me/profile-changes`
- `PUT /v1/customers/me/profile-changes/:profile_id`
- `POST /v1/customers/me/profile-changes/:profile_id/verify`
- `POST /v1/customers/me/profile-changes/:profile_id/confirm`

All require `UserUuid`.

- [ ] **Write handlers → wire routes → integration tests → run → commit**

```bash
cargo test -p customer-profile-service tests::integration::profile_changes -- --nocapture
git commit -m "feat(api): profile_change handlers, routes, integration tests"
```

---

## Phase 4 — Profile Images

### Task 17: S3 client + CloudFront signer

**Files:**
- Create: `crates/infrastructure/src/storage/s3.rs`
- Create: `crates/infrastructure/src/storage/cloudfront_signer.rs`

- [ ] **Step 1: s3.rs**

```rust
use aws_sdk_s3::Client;
use bytes::Bytes;

pub struct S3Storage {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Storage {
    pub async fn upload(&self, key: &str, data: Bytes, content_type: &str) -> Result<String, String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(format!("{}/{}", self.prefix, key))
            .body(data.into())
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("{}/{}", self.prefix, key))
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
```

- [ ] **Step 2: cloudfront_signer.rs**

Uses `rsa` crate PKCS#1 v1.5 (matching Go). Signs a canned policy URL valid for `image_expired_in_sec` seconds.

```rust
use rsa::{pkcs1::DecodeRsaPrivateKey, RsaPrivateKey};
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use rsa::pkcs1v15::SigningKey;
use sha1::Sha1;
use base64::Engine;

pub struct CloudFrontSigner {
    private_key: RsaPrivateKey,
    key_id: String,
    base_url: String,
    expires_in_secs: u32,
}

impl CloudFrontSigner {
    pub fn new(pem: &str, key_id: String, base_url: String, expires_in_secs: u32) -> anyhow::Result<Self> {
        let key = RsaPrivateKey::from_pkcs1_pem(pem)?;
        Ok(Self { private_key: key, key_id, base_url, expires_in_secs })
    }

    pub fn sign_url(&self, object_key: &str) -> anyhow::Result<String> {
        let expires = chrono::Utc::now().timestamp() as u64 + self.expires_in_secs as u64;
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), object_key);
        let policy = serde_json::json!({
            "Statement": [{
                "Resource": url,
                "Condition": { "DateLessThan": { "AWS:EpochTime": expires } }
            }]
        }).to_string();
        let signing_key = SigningKey::<Sha1>::new(self.private_key.clone());
        let mut rng = rand::thread_rng();
        let sig = signing_key.sign_with_rng(&mut rng, policy.as_bytes());
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
        let sig_url_safe = sig_b64.replace('+', "-").replace('=', "_").replace('/', "~");
        Ok(format!("{}?Expires={}&Signature={}&Key-Pair-Id={}", url, expires, sig_url_safe, self.key_id))
    }
}
```

- [ ] **Verify → commit**

```bash
cargo check --no-default-features -p infrastructure
git commit -m "feat(infra): S3 storage and CloudFront URL signer"
```

---

### Task 18: ProfileImage use cases + unit tests (100% coverage)

**Files:**
- Create: `crates/application/src/profile_images/use_cases.rs`
- Create: `tests/unit_tests/applications/profile_image_use_cases_test.rs`

Key logic:
- `upload(user_uuid, bytes, content_type, file_size)`: validate content type against `allow_image_types` (→ `BadRequest`); validate size ≤ `max_image_size_mb` (→ `BadRequest`); generate key = `{image_prefix}/{user_uuid}`; upload to S3; update `user_profiles.profile_image`; return signed CloudFront URL.
- `get_image(user_uuid)`: get customer, return signed URL from CloudFront for stored key. `NotFound` if no image.
- `delete_image(user_uuid)`: delete from S3; set `profile_image = NULL` in repo.

Unit tests mock `S3Storage` and `CloudFrontSigner` via traits.

- [ ] **Write → 100% tests → commit**

```bash
git commit -m "feat(app): ProfileImageUseCases with 100% unit test coverage"
```

---

### Task 19: ProfileImage handlers, routes, integration tests

Routes:
- `POST /v1/customers/me/profile-images` — multipart/form-data upload
- `GET /v1/customers/me/profile-images`
- `DELETE /v1/customers/me/profile-images`

Handler for upload uses `axum::extract::Multipart` to stream file bytes.

- [ ] **Write handlers → wire routes → integration tests → run → commit**

```bash
cargo test -p customer-profile-service tests::integration::profile_images -- --nocapture
git commit -m "feat(api): profile_image handlers, routes, integration tests"
```

---

## Phase 5 — The1 & Segments

### Task 20: The1 HTTP client

**Files:**
- Create: `crates/infrastructure/src/external/the1_client.rs`

```rust
pub struct The1Client {
    http: reqwest::Client,
    base_url: String,
}

impl The1Client {
    pub fn new(base_url: String) -> Self {
        Self { http: reqwest::Client::new(), base_url }
    }

    pub async fn get_profile(&self, access_token: &str) -> Result<The1ProfileResponse, String> {
        self.http
            .get(format!("{}/customers/me", self.base_url))
            .bearer_auth(access_token)
            .header("x-api-channel", "central-x")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<The1ProfileResponse>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn invoke_token(&self, refresh_token: &str) -> Result<InvokeTokenResponse, String> {
        self.http
            .post(format!("{}/auth/invoke", self.base_url))
            .bearer_auth(refresh_token)
            .header("x-api-channel", "central-x")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<InvokeTokenResponse>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_partner_member(&self, card_number: &str) -> Result<The1PartnerMemberResponse, String> {
        self.http
            .get(format!("{}/partner-members/{}", self.base_url, card_number))
            .header("x-api-channel", "central-x")
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<The1PartnerMemberResponse>()
            .await
            .map_err(|e| e.to_string())
    }
}
```

Define `The1ProfileResponse`, `InvokeTokenResponse`, `The1PartnerMemberResponse` structs matching Go entities.

- [ ] **Write → verify → commit**

```bash
git commit -m "feat(infra): The1 HTTP client (get_profile, invoke_token, get_partner_member)"
```

---

### Task 21: The1User domain + PgThe1UserRepository

**Files:**
- Create: `crates/domain/src/entities/the1_user.rs`
- Create: `crates/domain/src/repositories/the1_user_repository.rs`
- Create: `crates/infrastructure/src/persistence/pg_the1_user_repository.rs`

**The1User entity** (matches actual DB schema from migrations 003 + 008 — `tiers` is a separate table):
```rust
pub struct The1User {
    pub id: Uuid,
    pub user_uuid: Uuid,
    pub member_id: String,
    pub account_id: String,
    pub profile_id: String,
    pub card_number: Option<String>,
    pub tiers: Vec<Tier>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Tier {
    pub id: Uuid,
    pub code: String,
    pub name: Option<String>,
    pub expired_date: Option<DateTime<Utc>>,
}
```

**Trait:**
```rust
#[async_trait]
pub trait The1UserRepository: Send + Sync {
    async fn find_by_user(&self, user_uuid: Uuid) -> Result<Option<The1User>, RepositoryError>;
    async fn find_by_card_number(&self, card_number: &str) -> Result<Option<The1User>, RepositoryError>;
    async fn find_by_member_id(&self, member_id: &str) -> Result<Option<The1User>, RepositoryError>;
    async fn upsert(&self, user_uuid: Uuid, profile: UpsertThe1User) -> Result<The1User, RepositoryError>;
}
```

`PgThe1UserRepository`: `find_*` queries JOIN `the1_users` with `tiers`. `upsert()` uses INSERT ... ON CONFLICT DO UPDATE with QueryBuilder, then replaces tiers (DELETE + INSERT).

- [ ] **Write → verify → commit**

```bash
git commit -m "feat: The1User domain entities, trait, PgThe1UserRepository"
```

---

### Task 22: Segment use case + tests + handler + integration

Route: `GET /v1/customers/segments` (no auth header required; takes `card_number` query param)

Use case `GetSegment(card_number)`:
1. Call `the1_client.get_partner_member(card_number)`
2. Find or create The1User in DB (via repo upsert)
3. Return first tier as `SegmentResponse { segment_slug, expired_time, user_uuid }`

```rust
pub struct SegmentResponse {
    pub segment_slug: String,
    pub expired_time: Option<DateTime<Utc>>,
    pub user_uuid: Uuid,
}
```

- [ ] **Write use case + 100% unit tests + handler + integration test → run → commit**

```bash
cargo test -p customer-profile-service tests::unit_tests::applications::segment -- --nocapture
cargo test -p customer-profile-service tests::integration::segments -- --nocapture
git commit -m "feat: segment use case, handler, integration tests"
```

---

### Task 23: The1 account/partner use cases + tests + handlers + integration

Routes:
- `GET /v1/customers/the1/account` — `GetThe1Account` (takes `user_uuid` query param OR header; internal route, no auth)

Use cases:
- `get_the1_account(user_uuid)`: lookup `the1_users` by user_uuid → `NotFound` if absent
- `get_by_card_number(card_number)`: lookup by card_number
- `get_by_member_id(member_id)`: lookup by member_id
- `create_or_update(user_uuid, profile)`: upsert The1User; publish `SNS_USER_THE1_GET_PROFILE_UPDATED`

- [ ] **Write use cases + 100% unit tests + handlers + integration tests → run → commit**

```bash
git commit -m "feat: the1 account use cases, handlers, integration tests"
```

---

### Task 24: Update Customer + Identity to enrich with The1 data

**Modify:** `crates/application/src/customers/use_cases.rs`
**Modify:** `crates/application/src/identities/use_cases.rs`

`CustomerUseCases::get_by_id` (internal, used by `GET /v1/customers/profiles/:id`): after fetching customer, call `the1_user_repo.find_by_user()` and attach tiers. (The inline The1 HTTP refresh only happens in the identity invoke_token flow via `sync_user_identity`.)

`IdentityUseCases::sync_user_identity` (called after invoke_token): fetch live profile from The1 HTTP client, call `the1_use_cases.create_or_update()`, publish `SNS_USER_IDENTITY_LINKED_CHANGED`.

- [ ] **Update → re-run all unit tests → commit**

```bash
cargo test -p customer-profile-service tests::unit_tests -- --nocapture
git commit -m "feat: enrich customer and identity with The1 account data"
```

---

## Phase 6 — Wire & Finalize

### Task 25: AppFactoryState — wire all domains

**Files:**
- Modify: `crates/infrastructure/src/stages/factory.rs`
- Modify: `crates/application/src/repositories.rs`
- Modify: `crates/application/src/use_cases.rs`

```rust
// crates/application/src/repositories.rs
pub struct Repositories {
    pub customers: Arc<dyn CustomerRepository>,
    pub identities: Arc<dyn IdentityRepository>,
    pub profile_changes: Arc<dyn ProfileChangeRepository>,
    pub the1_users: Arc<dyn The1UserRepository>,
}

// crates/application/src/use_cases.rs
pub struct UseCases {
    pub customers: Arc<CustomerUseCases>,
    pub identities: Arc<IdentityUseCases>,
    pub profile_changes: Arc<ProfileChangeUseCases>,
    pub profile_images: Arc<ProfileImageUseCases>,
    pub segments: Arc<SegmentUseCases>,
    pub the1: Arc<The1UseCases>,
}
```

`AppFactoryState::new()`:
```rust
let the1_client = Arc::new(The1Client::new(factory.settings.the1_proxy_service_url.clone()));
let sms_client = Arc::new(SmsClient::new(factory.settings.sms_proxy_service_url.clone()));
let s3 = Arc::new(S3Storage::new(...));
let cloudfront = Arc::new(CloudFrontSigner::new(...)?);

let repos = Repositories {
    customers: Arc::new(PgCustomerRepository::new(pool.clone())),
    identities: Arc::new(PgIdentityRepository::new(pool.clone())),
    profile_changes: Arc::new(PgProfileChangeRepository::new(pool.clone())),
    the1_users: Arc::new(PgThe1UserRepository::new(pool.clone())),
};

let settings_arc = Arc::new(factory.settings.clone());

let use_cases = UseCases {
    customers: Arc::new(CustomerUseCases::new(repos.customers.clone(), settings_arc.clone())),
    identities: Arc::new(IdentityUseCases::new(repos.identities.clone(), repos.customers.clone(), the1_client.clone(), sns.clone(), settings_arc.clone())),
    profile_changes: Arc::new(ProfileChangeUseCases::new(repos.profile_changes.clone(), repos.customers.clone(), sms_client, sns.clone(), settings_arc.clone())),
    profile_images: Arc::new(ProfileImageUseCases::new(repos.customers.clone(), s3, cloudfront, settings_arc.clone())),
    segments: Arc::new(SegmentUseCases::new(repos.the1_users.clone(), the1_client.clone())),
    the1: Arc::new(The1UseCases::new(repos.the1_users.clone(), sns.clone(), settings_arc.clone())),
};
```

Also wire `AppState` in `api/src/routers.rs` to include all `use_cases` fields.

- [ ] **Wire → `cargo check` → `cargo build` → commit**

```bash
cargo build --no-default-features
git commit -m "feat: wire all domains into AppFactoryState and AppState"
```

---

### Task 26: OpenAPI docs + final integration smoke test

**Files:**
- Create: `crates/api/src/docs.rs` (utoipa `OpenApi` derive covering all handlers)

```rust
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        handlers::customers::create_customer,
        handlers::customers::search_customers,
        // ... all handler paths
    ),
    components(schemas(CustomerResponse, ApiResponse<CustomerResponse>, /* ... */)),
    tags((name = "customers"), (name = "identities"), (name = "profile_changes"), (name = "profile_images"), (name = "segments"), (name = "the1")),
)]
pub struct ApiDoc;
```

Also run the full integration test suite end-to-end:
```bash
cargo test -p customer-profile-service tests::integration -- --nocapture
```
Expected: all integration tests pass.

- [ ] **Write docs → run all tests → commit**

```bash
git commit -m "feat(api): OpenAPI docs and final integration smoke test pass"
```

---

### Task 27: Coverage measurement + final commit

- [ ] **Step 1: Install coverage tool if needed**

```bash
cargo install cargo-llvm-cov
```

- [ ] **Step 2: Run coverage**

```bash
cargo llvm-cov --no-default-features \
  --workspace \
  --ignore-filename-regex 'tests/' \
  --html \
  --output-dir coverage/
```

Check report:
- `application/` (use cases) → must be **100%**
- Overall → must be **≥ 85%**

- [ ] **Step 3: Fix any gaps**

If any use-case branch is below 100%, add the missing test case to the relevant `*_use_cases_test.rs`.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: final coverage verification — application 100%, overall ≥85%"
git push origin main
```

---

## Verification

End-to-end smoke test sequence:
1. Start local Postgres, apply migration: `sqlx migrate run`
2. `cargo run --no-default-features` — service starts on `:8000`
3. `curl http://localhost:8000/livez` → `{ "status": "ok" }`
4. `curl http://localhost:8000/swagger` → SwaggerUI loads
5. `curl -X POST http://localhost:8000/v1/customers -H 'Content-Type: application/json' -d '{"phone":"0812345678"}' ` → 201 with `{ "success": true, "data": {...} }`
6. `curl http://localhost:8000/v1/customers/me -H 'user_uuid: <uuid from step 5>'` → 200
7. Run full test suite: `cargo test` → all green
8. Run coverage: `cargo llvm-cov` → application 100%, overall ≥85%
