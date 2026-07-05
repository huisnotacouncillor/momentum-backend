# ADR 0003: Repository Pattern with Trait Abstraction

## Status
Accepted (2026-07-05)

## Context
Services need data access. Options:
1. **Direct DB calls**: Simple but couples services to Diesel
2. **Repository structs**: Concrete repos (current state)
3. **Repository traits**: Abstract interface with concrete implementations

## Decision
Adopt **Repository Trait abstraction** with concrete adapter implementations.

## Rationale
- Enables unit testing with mock repositories (no DB needed)
- Decouples service layer from data layer
- Allows swapping implementations (e.g., cache vs DB)

## Implementation

```rust
// Trait definition
#[async_trait]
pub trait IssueRepositoryTrait: Send + Sync {
    async fn find_by_id_in_workspace(...) -> Result<Option<Issue>, AppError>;
    // ...
}

// Adapter for Diesel implementation
pub struct IssueRepoAdapter;

#[async_trait]
impl IssueRepositoryTrait for IssueRepoAdapter {
    async fn find_by_id_in_workspace(...) -> Result<Option<Issue>, AppError> {
        super::issues::IssueRepo::find_by_id_in_workspace(...)
            .map_err(AppError::Database)
    }
}
```

## Consequences

### Positive
- Services can be tested with mock repos
- Clear interface boundary
- Migration path to alternative storage backends

### Negative
- Boilerplate adapter code
- Indirect calls add slight complexity
- Async traits have minor overhead

## When to Add a New Trait
- When the repository is used in business logic with test coverage goals
- When multiple implementations might exist (DB + cache, etc.)

## References
- `momentum_core/src/db/repositories/traits.rs`
- `docs/architecture/REFACTOR_PLAN.md` - P2.6