# ADR 0001: Use Axum as the Web Framework

## Status
Accepted (2026-07-05)

## Context
We needed a Rust web framework for our HTTP/WebSocket backend. The main
candidates were:
- **Axum**: Built on Tower, type-safe extractors, good async support
- **Actix-web**: Mature, fast, actor-based
- **Rocket**: Easy to use, but historically less async-friendly

## Decision
We chose **Axum 0.6** because:
1. First-class Tokio integration (matches our `tokio-tungstenite` choice)
2. Tower middleware ecosystem (cors, tracing, timeout)
3. Type-safe extractors via `FromRequestParts`
4. Composable routers
5. Strong community and momentum

## Consequences

### Positive
- Native async/await support, no runtime mismatch
- Excellent middleware composition via `tower::Layer`
- WebSocket upgrade built-in via `WebSocketUpgrade`
- Type-safe handlers reduce runtime errors

### Negative
- Axum 0.6 → 0.7 migration needed in future (some API changes)
- Extractors can be confusing for newcomers
- Less out-of-the-box than Rocket

### Mitigation
- Pin to Axum 0.6 LTS
- Document extractor patterns in team wiki