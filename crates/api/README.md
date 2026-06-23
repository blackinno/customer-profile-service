# API Layer

The API layer exposes the application's functionality to external clients via HTTP or other protocols. It acts as the entry point for requests and responses.

## Contents

- **Controllers**: Handle incoming requests and coordinate with the application layer
- **Routes**: Define API endpoints and routing logic
- **Middleware**: Cross-cutting concerns such as logging, authentication, etc.
- **Extractors**: Request data extraction and validation
- **Responses**: Standardized API responses

This layer depends on the application layer.
