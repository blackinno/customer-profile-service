# Test Suite

This directory contains comprehensive unit and integration tests for the Clean Architecture Rust application.

## Test Structure

The test suite is organized into two main categories:

### 📁 Folder Structure

```
tests/
├── unit_tests/          # Unit tests (isolated component testing)
│   ├── domain/          # Domain layer tests
│   │   ├── mod.rs
│   │   └── user_entity_tests.rs
│   ├── applications/    # Application layer tests
│   │   ├── mod.rs
│   │   └── user_use_cases_test.rs
│   └── mod.rs
├── integration/         # Integration tests (full stack testing)
│   ├── controllers/     # Controller integration tests
│   │   ├── mod.rs
│   │   └── user_controller_test.rs
│   └── mod.rs
├── mod.rs
└── README.md
```

### 🔬 Unit Tests (`unit_tests/`)

#### 📦 Domain Tests (`unit_tests/domain/`)

- **User Entity Tests** (`user_entity_tests.rs`): Tests for user creation, validation, updates, and serialization
- **Business Logic Tests**: Tests for domain-specific business rules and constraints
- **Value Object Tests**: Tests for email validation and other value objects

#### 🎯 Application Tests (`unit_tests/applications/`)

- **Use Case Tests** (`user_use_cases_test.rs`): Tests for all user-related use cases (create, read, update, delete)
- **DTO Validation Tests**: Tests for request/response data transfer objects
- **Business Rule Tests**: Tests for application-level business rules
- **Error Handling Tests**: Tests for proper error propagation and handling
- **Mock Repository Tests**: Tests using in-memory mock repositories

### 🌐 Integration Tests (`integration/`)

#### Controller Integration Tests (`integration/controllers/`)

- **User Controller Tests** (`user_controller_test.rs`): Tests for user-related HTTP endpoints
- **Request/Response Tests**: Tests for proper JSON serialization/deserialization
- **Error Response Tests**: Tests for proper HTTP error codes and messages
- **Validation Tests**: Tests for request validation and error handling
- **Content Type Tests**: Tests for proper content-type validation
- **Full Lifecycle Tests**: End-to-end tests covering create, read, update, delete operations

## Running Tests

### Basic Test Commands

```bash
# Run all tests
cargo test

# Run tests with verbose output
cargo test --verbose

# Run tests in a single thread (useful for debugging)
cargo test -- --test-threads=1

# Run tests with output capture disabled
cargo test -- --nocapture
```

### Run Tests by Category

```bash
# Run all unit tests
cargo test unit_tests

# Run all integration tests
cargo test integration

# Run specific test modules
cargo test unit_tests::domain
cargo test unit_tests::applications
cargo test integration::controllers
```

### Run Tests by Name Pattern

```bash
# Run tests containing "user" in the name
cargo test user

# Run tests containing "create" in the name
cargo test create

# Run specific test function
cargo test test_user_creation
```

### Advanced Test Options

```bash
# Run tests and show test execution times
cargo test -- --report-time

# Run tests with JSON output (useful for CI/CD)
cargo test -- --format=json

# Run tests with JUnit XML output
cargo test -- --format=junit

# Run ignored tests
cargo test -- --ignored

# Run tests in random order
cargo test -- --shuffle

# List all available tests without running them
cargo test -- --list
```

## Code Coverage

**Important**: `cargo test` itself does **not** have built-in coverage flags. You need to use external tools for coverage reporting.

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Run tests with coverage
cargo tarpaulin --verbose --all-features --workspace --timeout 120

# Generate HTML coverage report
cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out Html

# Generate XML coverage report (for CI/CD)
cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out Xml

# Run coverage with specific output directory
cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out Html --output-dir coverage-report
```

### Coverage Report Types

- **HTML**: Visual coverage report with line-by-line highlighting
- **XML**: Machine-readable format for CI/CD integration
- **LCOV**: Standard format compatible with many tools
- **JSON**: Structured data format for custom processing

## Test Categories

### Unit Tests

Unit tests focus on testing individual components in isolation:

- **Domain Layer**: Tests individual entities and value objects without external dependencies
- **Application Layer**: Tests use cases and DTOs with mocked repositories and services
- **Fast Execution**: Use in-memory implementations for quick feedback
- **Isolated**: No external services or databases required

### Integration Tests

Integration tests verify that multiple components work together correctly:

- **API Integration**: Tests HTTP endpoints with full request/response cycles
- **Cross-Layer**: Tests interaction between API, Application, and Domain layers
- **Realistic Scenarios**: Uses actual HTTP requests and responses
- **End-to-End**: Covers complete user workflows

## Test Utilities

### Mock Repositories

The test suite includes comprehensive mock implementations of repositories:

- `MockUserRepository`: In-memory repository for testing user operations
- Supports all CRUD operations
- Includes failure simulation for error testing
- Thread-safe using Arc<Mutex<>>

### Test Data

Tests use consistent test data:

- Valid emails: `test@example.com`, `user@example.com`
- Invalid emails: `invalid-email`, `test@`, `@example.com`
- Valid names: `John Doe`, `Jane Smith`
- Invalid names: `""` (empty), `"a".repeat(101)` (too long)

## Test Patterns

### Unit Test Patterns

```rust
// Simple unit test
#[test]
fn test_user_creation() {
    let user = User::new("test@example.com".to_string(), "John Doe".to_string());
    assert_eq!(user.email, "test@example.com");
}

// Async unit test with mocks
#[tokio::test]
async fn test_create_user_success() {
    let repository = Arc::new(MockUserRepository::new());
    let use_cases = UserUseCases::new(repository);
    // Test implementation
}
```

### Integration Test Patterns

```rust
// HTTP integration test
#[tokio::test]
async fn test_create_user_endpoint() {
    let app = create_test_app();
    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let (status, body) = send_request(app, request).await;
    assert_eq!(status, StatusCode::OK);
}
```

### Error Testing

Tests verify both success and failure scenarios:

```rust
// Test success case
assert!(result.is_ok());

// Test error case
assert!(result.is_err());
match result.unwrap_err() {
    ApplicationError::NotFound(msg) => {
        assert_eq!(msg, "User not found");
    }
    _ => panic!("Expected NotFound error"),
}
```

## Test Dependencies

The following dev-dependencies are required:

- `tokio-test`: For async test utilities
- `mockall`: For creating mock objects (optional)
- `async-trait`: For async trait implementations
- `validator`: For validation testing
- `tower`: For HTTP service testing

## Best Practices

### Unit Tests

1. **Test Isolation**: Each test should be independent and not rely on other tests
2. **Mock External Dependencies**: Use mock repositories instead of real implementations
3. **Test Both Success and Failure Cases**: Verify both happy path and error scenarios
4. **Fast Execution**: Keep unit tests fast with in-memory implementations
5. **Single Responsibility**: Each test should verify one specific behavior

### Integration Tests

1. **Realistic Scenarios**: Test actual user workflows and API contracts
2. **End-to-End Validation**: Verify complete request/response cycles
3. **Error Handling**: Test HTTP error codes and proper error responses
4. **Data Validation**: Ensure proper JSON serialization/deserialization
5. **Security Testing**: Validate authentication and authorization flows

### General Guidelines

1. **Use Descriptive Test Names**: Test names should clearly indicate what is being tested
2. **Test Edge Cases**: Include tests for boundary conditions and invalid inputs
3. **Maintain Test Data**: Use consistent, realistic test data across tests
4. **Document Complex Tests**: Add comments for complex test scenarios
5. **Regular Maintenance**: Keep tests up-to-date with code changes

## Adding New Tests

When adding new functionality:

### For Unit Tests

1. **Add Domain Tests**: Test new entities and value objects in `unit_tests/domain/` directory
2. **Add Application Tests**: Test new use cases and business logic in `unit_tests/applications/` directory
3. **Update Mock Repositories**: Add new methods to mock implementations
4. **Add Test Data**: Create new test scenarios and edge cases

### For Integration Tests

1. **Add Controller Tests**: Test new endpoints in `integration/controllers/` directory
2. **Test HTTP Interactions**: Verify request/response formats and status codes
3. **Add Workflow Tests**: Create end-to-end user workflow tests
4. **Update Test Utilities**: Enhance helper functions for new scenarios

## Test Suite Statistics

### Test Organization

- **Domain Tests**: Located in `unit_tests/domain/` directory
- **Application Tests**: Located in `unit_tests/applications/` directory
- **Integration Tests**: Located in `integration/controllers/` directory

### Test Categories Breakdown

- **Unit Tests**: Fast, isolated component testing
- **Integration Tests**: End-to-end system testing

## Continuous Integration

These tests are designed to run in CI/CD pipelines:

- **No External Dependencies**: Unit tests use in-memory implementations
- **Fast Execution**: Quick feedback loop for development
- **Comprehensive Coverage**: Tests all architectural layers
- **Clear Error Messages**: Detailed output for debugging failures
- **Parallel Execution**: Tests can run concurrently

## Coverage Goals

### Unit Tests

Aim for high coverage on isolated components:

- **Domain Layer**: 95%+ coverage
- **Application Layer**: 90%+ coverage
- **Focus**: Business logic and data validation

### Integration Tests

Aim for comprehensive API coverage:

- **API Endpoints**: 100% endpoint coverage
- **Error Scenarios**: All error conditions tested
- **Focus**: User workflows and system integration

### Overall Targets

- **Combined Coverage**: 90%+ overall coverage
- **Critical Paths**: 100% coverage for critical business flows
- **Error Handling**: Complete error scenario coverage

## Troubleshooting

### Common Issues

1. **Tests timing out**: Increase timeout with `--timeout` flag
2. **Flaky tests**: Use `--test-threads=1` for sequential execution
3. **Coverage not working**: Ensure proper tool installation and environment variables
4. **Memory issues**: Run tests with `--release` flag for large test suites

### Debug Commands

```bash
# Run tests with backtrace for panic debugging
RUST_BACKTRACE=1 cargo test

# Run tests with full backtrace
RUST_BACKTRACE=full cargo test

# Run tests with logging enabled
RUST_LOG=debug cargo test

# Run tests with no capture to see println! output
cargo test -- --nocapture
```

## Example Commands

### Daily Development

```bash
# Quick test run
cargo test

# Test with coverage
cargo tarpaulin --verbose --all-features --workspace

# Test specific functionality
cargo test user_creation
```

### CI/CD Pipeline

```bash
# Run all tests with coverage and XML output
cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out Xml

# Run tests with JSON output for parsing
cargo test -- --format=json
```

### Local Development

```bash
# Run tests with HTML coverage report
cargo tarpaulin --verbose --all-features --workspace --out Html --output-dir coverage-report

# Open coverage report
open coverage-report/index.html  # macOS
xdg-open coverage-report/index.html  # Linux
```
