# ADR 0089: Show display names in the dashboard workload

## Status

Accepted on 2026-11-08 at the author's request. Supersedes the statement of
ADR 0087 that workload keeps showing the email; the rest of ADR 0087 stands.

## Context

The redesigned dashboard lists the active cases of each litigator. A list of
email addresses read as a technical inventory, while every office account
already carries an immutable display name written by the Owner.

## Decision

Each row of the dashboard workload carries the member's display name:
`{user_id, email, name, active_cases}`. The query reads `users.display_name`
and the adapter admits it through `DisplayName::from_stored`, so a stored
value that the domain rejects is a port failure, not a rendered string. The
page shows the name with the member's initials and keeps the email as the
title of the row.

The dashboard heading shows the date as its title and the signed-in name with
the role and scope as a detail line; it no longer greets the person.

The name stays out of `Principal`, the audit log and the CA store, as in
ADR 0087. Contracts, the team directory, pickers and reports keep showing the
email.

## Consequences

The workload response gains one field; clients that validate it with a closed
key set must accept `name`. A wrong name is visible to every reader of the
dashboard until an administrator corrects it outside the application.
