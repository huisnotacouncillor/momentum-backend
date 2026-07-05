# Architecture Decision Records (ADR)

This directory contains Architecture Decision Records (ADRs) for the
Momentum backend project.

## What is an ADR?
An ADR is a short document capturing an important architectural decision,
its context, and consequences. ADRs are immutable once accepted.

## Index

| Number | Title | Status |
|--------|-------|--------|
| [0001](0001-use-axum.md) | Use Axum as the Web Framework | Accepted |
| [0002](0002-diesel-and-r2d2.md) | Use Diesel + r2d2 for Database Access | Accepted |
| [0003](0003-repository-pattern.md) | Repository Pattern with Trait Abstraction | Accepted |
| [0004](0004-api-versioning.md) | API Versioning via URL Path Prefix | Accepted |
| [0005](0005-rbac-model.md) | RBAC for Workspace Permissions | Accepted |

## Writing a New ADR

1. Copy `template.md` to `NNNN-short-title.md`
2. Use the next sequential number
3. Fill in: Status, Context, Decision, Consequences
4. Submit as PR
5. After merge, update this index

## ADR Status Lifecycle

- **Proposed**: Under discussion
- **Accepted**: Decision made and active
- **Deprecated**: No longer relevant, kept for history
- **Superseded**: Replaced by another ADR (link to replacement)