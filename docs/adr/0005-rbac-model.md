# ADR 0005: RBAC for Workspace Permissions

## Status
Accepted (2026-07-05)

## Context
Multi-tenant SaaS requires strict access control. We need to enforce:
- Workspace isolation
- Role-based permissions

## Decision
Implement **Role-Based Access Control (RBAC)** with four role levels:

| Role | Level | Capabilities |
|------|-------|--------------|
| Owner | 4 | All operations including delete workspace |
| Admin | 3 | Manage members, all CRUD except delete workspace |
| Member | 2 | Standard CRUD on workspace resources |
| Guest | 1 | Read-only access |

## Implementation
- `WorkspaceMemberRole` enum in `momentum_core::db::models::workspace_member`
- `has_at_least()` method for hierarchical checks
- Convenience functions: `require_owner`, `require_admin`, `require_member`

## Example
```rust
pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    user: AuthUserInfo,
    Path(workspace_id): Path<Uuid>,
) -> impl IntoResponse {
    // Only Owner can delete workspace
    if let Err(e) = require_owner(&state, workspace_id, &user).await {
        // Return 403 Forbidden
    }
    // ... business logic
}
```

## Security Implications
- All sensitive endpoints MUST call a `require_*` function
- Default-deny: missing check = no access
- Tests verify cross-role authorization

## Future Considerations
- Resource-level permissions (e.g., per-project roles)
- Time-bounded access (e.g., temporary admin)
- Audit log for permission changes

## References
- `momentum_api/src/middleware/permission.rs`
- `momentum_core/src/db/models/workspace_member.rs`
- `docs/architecture/REFACTOR_PLAN.md` - P0.4