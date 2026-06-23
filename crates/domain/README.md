# Domain Layer

The domain layer contains the core business logic and rules of the application. It is independent of any external frameworks or technologies.

## Contents

- **Entities**: Core business objects with identity and behavior
- **Value Objects**: Immutable objects representing concepts with no identity
- **Repositories**: Abstractions for data access
- **Services**: Domain-specific operations that don't naturally fit within entities
- **Events**: Domain events for business processes

This layer should have no dependencies on other layers.
