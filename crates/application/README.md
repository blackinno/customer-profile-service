# Application Layer

The application layer orchestrates business processes and use cases. It coordinates the flow of data to and from the domain layer and defines application-specific logic.

## Contents

- **Use Cases**: Application-specific business processes
- **Commands/Queries**: CQRS pattern for separating reads and writes
- **DTOs**: Data transfer objects for communication between layers
- **Interfaces**: Abstractions for external dependencies
- **Validators**: Input validation logic
- **Errors**: Application-level error types

This layer depends on the domain layer but not on infrastructure or frameworks.
