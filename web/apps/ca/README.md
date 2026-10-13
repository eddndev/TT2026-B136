# CA operator portal

This application provides the separate CA operator login: email and password,
TOTP or a recovery code, an authenticated operator view and logout. Session
deadlines come from the server. Credentials, challenges and bearer tokens stay
in memory; reloading the page requires a fresh login.

It consumes the shared `@qadra/ui` brand, icons and base styles and builds
separately from the Qadra office application. Authenticated operators review
external CSRs, issue Partner or document-signing certificates, download public
certificates, revoke locally, publish CRLs with explicit maintenance confirmation
and resume uncertain operations from their durable history. Existing TSA
certificates are read-only inventory. Holder private keys never enter the portal.

From `web/`, use `npm run dev:ca`, `npm run build:ca`, `npm run preview:ca`
and `npm run test:ca`. Browser acceptance runs with
`npm run test:e2e --workspace @qadra/ca -- --workers=1`. Development and preview use port 18088. Their `/api`
proxy targets the independent CA service at `http://127.0.0.1:18089`; override
it with `CA_API_PROXY_TARGET` when needed. Production must route `/api` to the
CA service on the portal's own origin.

The client reuses the pure session clock in
`web/apps/qadra/src/lib/session-clock.mjs`. It does not load the office API,
roles, registration, certificate login or email recovery flows. The operator
must be enrolled separately before access can be exercised. A frontend build
does not establish enrollment or operational acceptance of the CA.

The finite implementation and acceptance scope is in
[docs/ca/README.md](../../../docs/ca/README.md).
