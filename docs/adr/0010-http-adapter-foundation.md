# ADR-0010: HTTP adapter foundation

- Status: Accepted
- Date: 2026-08-17

## Context

The cryptographic and timestamping core is usable without the external PSC.
The web crate existed only as an empty workspace member, while the next
implementation slice needs a stable inbound boundary before adding sessions,
persistence, and the user interface.

## Decision

Use Axum for the inbound HTTP adapter and expose a small, tested `router()`
entry point. The first route is `/healthz`, returning `200 OK` and `ok`. The
router remains independent of Cincel and of any external timestamp authority;
application services and persistence ports will be injected in later routes.

## Consequences

- The workspace now has a compile-time and testable HTTP boundary.
- Deployment checks can verify process readiness without invoking business
  services or paid external integrations.
- The next increments can add application handlers, session management, and
  persistence without changing the router construction contract.
- The route is not a substitute for authentication or authorization and does
  not claim that the web surface is complete.
