default:
	@echo "Makefile commands:"
	@echo "  make              - Show this help"
	@echo "  make run          - Build and run the project (incremental)"
	@echo "  make build        - Build the project (incremental)"
	@echo "  make watch        - Watch for changes and re-run (cargo-watch)"
	@echo "  make check        - cargo check"
	@echo "  make check-fast   - cargo check --no-default-features (skip AWS SDK)"
	@echo "  make fmt          - cargo fmt --all (apply formatting)"
	@echo "  make fmt-check    - cargo fmt --all --check (verify only; CI-friendly)"
	@echo "  make test         - Run tests"
	@echo "  make clean        - cargo clean (force a full rebuild)"

clean:
	@echo "Cleaning the project..."
	cargo clean

run:
	@echo "Running the project..."
	source ./.env && cargo run

build:
	@echo "Building the project..."
	cargo build

watch:
	@echo "Starting file watcher..."
	source ./.env && cargo watch -x run

check:
	@echo "Checking the project..."
	cargo check

# Skip the AWS SDK on the dev check loop. Use this when iterating on code that
# doesn't touch SNS — saves several minutes on a cold cache and ~30s on warm.
check-fast:
	@echo "Checking the project (no AWS SDK)..."
	cargo check --no-default-features

fmt:
	@echo "Formatting the project..."
	cargo fmt --all

fmt-check:
	@echo "Checking formatting (no writes)..."
	cargo fmt --all -- --check

test:
	@echo "Running tests..."
	source ./.env && cargo test
