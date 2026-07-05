# ADR 0002: Use Diesel + r2d2 for Database Access

## Status
Accepted (2026-07-05)

## Context
We need an ORM/database layer for PostgreSQL with the following requirements:
- Type-safe queries
- Compile-time SQL validation
- Connection pooling
- Transaction support

Candidates:
- **Diesel 2.0**: Mature Rust ORM, synchronous, compile-time checked
- **sqlx**: Async-native, runtime SQL validation
- **SeaORM**: Built on sqlx, async

## Decision
We chose **Diesel 2.0 + r2d2** because:
1. Compile-time query validation catches errors early
2. Mature ecosystem with extensive Postgres support
3. Synchronous queries are simpler to reason about
4. r2d2 provides battle-tested connection pooling

## Consequences

### Positive
- Errors caught at compile time
- Strong typing through to SQL
- Predictable performance
- Good documentation

### Negative
- Synchronous API blocks async runtimes
- Migration to async sqlx would be costly
- Schema must be managed via Diesel CLI

### Mitigation for Sync-in-Async Issue
- Wrap all DB calls in `tokio::task::spawn_blocking`
- See `momentum_core::db::run_db` helper
- Long-term: consider sqlx for new services

## References
- `momentum_core/src/db/mod.rs` - DB pool and run_db helper
- `docs/architecture/REFACTOR_PLAN.md` - P1.3 spawn_blocking fix