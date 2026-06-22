# Rust Clean Architecture Template

This repository provides a template for building scalable and maintainable Rust applications using Clean Architecture principles. The structure is suitable for a wide range of backend services.

## Features

- **Clean Architecture** — Modular crate-based structure for clear separation of concerns
- **Axum 0.8 + Tokio** — Async HTTP framework on top of the Tokio runtime
- **PostgreSQL with SQLx** — Async database access via SQLx `QueryBuilder`
- **Background Jobs** — PostgreSQL-backed job queue via [`qml-rs`](https://crates.io/crates/qml-rs)
- **AWS SNS Messaging** — Event publishing via the AWS SDK
- **OpenAPI / Swagger UI** — Auto-generated API docs via `utoipa`, served at `/swagger`
- **Validation** — Input validation with the `validator` crate
- **Error Handling** — Three-layer error model (`DomainError` → `ApplicationError` → RFC 7807 responses)
- **Logging** — Structured logging with `tracing`

## Architecture Overview

This project follows Clean Architecture (hexagonal) — inner layers never depend on outer layers.

- **domain** — Entities, value objects, repository traits, domain errors, event dispatcher port
- **application** — Use cases, request/response DTOs, application errors
- **infrastructure** — PostgreSQL repos, AWS SNS, config, background tasks, factory (DI)
- **api** — Axum controllers, routes, middleware, OpenAPI docs

## Directory Layout

```
├── crates/                       # Source code, split by architectural layer
│   ├── domain/                   # Entities, repository traits, errors, event port
│   ├── application/              # Per-feature modules (users/), DI bag, use-case aggregate
│   │   ├── users/                # dtos.rs + use_cases.rs
│   │   ├── repositories.rs       # `Repositories` DI bag
│   │   └── use_cases.rs          # `UseCases` aggregate
│   ├── infrastructure/           # Per-feature persistence + AWS / QML / config
│   │   └── users/                # pg_repository.rs
│   └── api/                      # Per-feature controllers, middleware, OpenAPI
│       └── users/                # controller.rs (with `routes(state)` fn)
├── cmd/                          # Application entrypoint (main.rs)
├── migrations/               # SQLx database migrations
├── tests/                    # Unit and integration tests (see tests/README.md)
└── deployment/               # Dockerfile and docker-compose configs
```

## Getting Started

### Prerequisites

1. [Rust](https://www.rust-lang.org/tools/install) (latest stable)
2. [Docker](https://docs.docker.com/get-docker/) and Docker Compose
3. CLI tools:
   ```sh
   cargo install sqlx-cli --no-default-features --features postgres  # migrations
   cargo install cargo-watch                                         # for `make watch`
   ```

### Setup

1. **Copy environment file:**

   ```sh
   cp .env.sample .env
   ```

2. **Start PostgreSQL:**

   ```sh
   docker-compose -f deployment/docker-compose-dev.yml up -d
   ```

3. **Run migrations:**

   ```sh
   source ./.env && sqlx migrate run
   ```

4. **Run the service:**

   ```sh
   make run        # cargo run with .env loaded
   # or
   make watch      # hot reload via cargo-watch
   ```

The server listens on `http://0.0.0.0:8000` by default.

> **Note:** `make` targets `source ./.env` before invoking cargo. Running `cargo run` / `cargo test` directly requires manually exporting env vars first (or `source ./.env`).

## Make Commands

```sh
make run          # Build and run with .env
make build        # Clean build
make watch        # Hot reload (cargo-watch)
make test         # Run all tests
make check        # Cargo check (full build, includes AWS SDK)
make check-fast   # Cargo check --no-default-features (skip AWS SDK; fast dev loop)
make fmt          # Apply rustfmt across the workspace
make fmt-check    # Verify formatting without writing (CI-friendly)
make clean        # cargo clean
```

### Cargo features

- **`sns`** (default-on): pulls in `aws-config` + `aws-sdk-sns` and wires the
  `Message`/`AwsSns` scaffolding into `AppFactoryState`. Production builds
  always enable this.
- Disable with `--no-default-features` (or `make check-fast`) when iterating
  on code that doesn't touch SNS — the AWS SDK is the heaviest part of the
  dependency graph and dropping it shaves several minutes off a cold build
  and tens of seconds off an incremental check.

A CI job (`check (no sns)`) verifies the `--no-default-features` build path
on every PR so the feature combination doesn't bit-rot.

## Environment Variables

Defined in `.env` (see `.env.sample`):

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | *required* | PostgreSQL connection string |
| `SERVER_HOST` | `0.0.0.0` | Bind address |
| `SERVER_PORT` | `8000` | Bind port |
| `QML_DATABASE_URL` | falls back to `DATABASE_URL` | Job queue DB URL |
| `QML_WORKER_COUNT` | `2` | Background worker threads |
| `QML_BATCH_SIZE` | `5` | Jobs per batch |
| `QML_RETRY_MAX_ATTEMPTS` | `1` | Max retry attempts for failed jobs |
| `QML_RETRY_BASE_SECONDS` | `1` | Initial backoff delay (seconds) |
| `QML_RETRY_MULTIPLIER` | `2.0` | Exponential multiplier between retries |
| `QML_RETRY_MAX_SECONDS` | `60` | Cap on backoff delay (seconds) |
| `AWS_REGION` | `ap-southeast-1` | AWS SDK region |
| `RUST_LOG` | `info` | Tracing log level |

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/healthz` | Health check |
| `GET` | `/swagger` | OpenAPI / Swagger UI |
| `GET` | `/users` | List users |
| `POST` | `/users` | Create user |
| `GET` | `/users/{id}` | Get user by ID |
| `PUT` | `/users/{id}` | Update user |
| `DELETE` | `/users/{id}` | Delete user |

Responses are wrapped as `ApiResponse { success, data, message }`. Errors follow RFC 7807 ProblemDetails.

## Database Migrations

```sh
sqlx migrate add <MIGRATION_NAME>   # Create new migration in migrations/
sqlx migrate run                    # Run all pending migrations
sqlx migrate revert                 # Revert last migration
sqlx migrate info                   # Check migration status
```

Migrations also run automatically on application startup.

## Database Access

Repositories build their SQL at runtime with SQLx's [`QueryBuilder`](https://docs.rs/sqlx/latest/sqlx/struct.QueryBuilder.html) and map rows into domain entities via `FromRow` structs (see `crates/infrastructure/src/users/pg_repository.rs`). There is no compile-time query cache, so builds and CI never need a live database.

## Testing

```sh
make test                          # Run all tests with .env loaded
cargo test unit_tests::            # Unit tests only (filter)
cargo test integration::           # Integration tests only (filter)
```

Test patterns, mock repositories, and coverage tooling are documented in [`tests/README.md`](./tests/README.md).

## Adding a New Entity

Code is grouped per-feature within each crate (e.g. `users/`). Use that folder as the reference; the same shape applies for any new entity (`orders/`, `invoices/`, …).

1. **Domain** — add `crates/domain/src/entities/<entity>.rs` and `crates/domain/src/repositories/<entity>_repository.rs` (the trait). Re-export from each `mod.rs`.
2. **Application** — create `crates/application/src/<entity>/` with `mod.rs`, `dtos.rs`, `use_cases.rs`. Re-export public types from `mod.rs` and add `pub mod <entity>;` to `lib.rs`.
3. **Infrastructure** — create `crates/infrastructure/src/<entity>/` with `mod.rs` and `pg_repository.rs`. Add `pub mod <entity>;` to `lib.rs`.
4. **API** — create `crates/api/src/<entity>/` with `mod.rs` and `controller.rs` (defining `routes(state) -> Router`). Add `pub mod <entity>;` to `lib.rs` and register OpenAPI paths/schemas in `crates/api/src/docs.rs`.
5. **Migration** — `sqlx migrate add create_<entity>_table`, then write the SQL.
6. **Wire it up** — add a field to `application::Repositories` and `application::UseCases`, construct it in `infrastructure::AppFactoryState::new`, and merge `<entity>::routes(state)` in `crates/api/src/routers.rs`.
7. **Tests** — entity tests in `tests/unit_tests/domain/`, use-case tests with a mock repository in `tests/unit_tests/applications/`, controller tests in `tests/integration/controllers/`.

## Troubleshooting

- **Database connection issues:** confirm Postgres is running (`docker ps`), `DATABASE_URL` is set, and migrations are applied (`sqlx migrate info`).
- **Build cache acting up:** `cargo clean && cargo build`.

## Contributing

1. Follow the existing code structure and patterns
2. Add unit tests for domain/use cases and integration tests for new endpoints
3. Run `make fmt` before submitting; `make fmt-check` must pass in CI
4. Ensure `make test` passes before submitting

## License

This project is licensed under the MIT License.
