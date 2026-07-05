# ADR 0004: API Versioning via URL Path Prefix

## Status
Accepted (2026-07-05)

## Context
As the API evolves, breaking changes are inevitable. We need a versioning strategy.

Options:
1. **URL path versioning**: `/v1/users`, `/v2/users`
2. **Header versioning**: `Accept: application/vnd.momentum.v2+json`
3. **Query parameter versioning**: `/users?version=2`

## Decision
Use **URL path versioning** with `/v1/` prefix.

## Rationale
- Discoverable and explicit
- Easy to route in middleware/load balancers
- Compatible with all HTTP clients
- Clear separation in logs and metrics

## Implementation
```rust
let app = Router::new()
    .nest("/v1", protected_routes)
    .merge(auth_routes)
    .merge(websocket_routes);
```

## Migration Path

When introducing breaking changes:
1. Create `/v2/` router with new schema
2. Keep `/v1/` for backwards compatibility
3. Document deprecation timeline
4. Communicate to clients at least 6 months before sunset

## When to Bump Version
- Removing a field
- Changing field type
- Renaming endpoint
- Changing authentication scheme
- Changing error response shape

## NOT a Breaking Change
- Adding new fields to response (clients should ignore unknowns)
- Adding new endpoints
- Adding new optional query parameters

## References
- `momentum_api/src/routes/mod.rs` - `create_v1_router`
- `momentum_api/src/main.rs` - `/v1` mount