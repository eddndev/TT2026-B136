# 0062 - Scoped PostgreSQL validation during server construction

## Status

Accepted. Focused PostgreSQL and composition checks pass. The populated HTTP
acceptance preserves restart history and completes both signal restarts within
the existing budget; measurements are recorded in docs/verification-report.md.

## Context

The composition root constructs independent PostgreSQL adapters. Every ordinary
adapter opening checks the same catalog, runtime privileges and immutable
inventories. Repeating that complete scan for each adapter increases startup
cost as the persisted inventory grows. Skipping validation globally or caching
a successful URL would let later independent openings accept changed state.

## Decision

Provide an explicit, sealed connection source usable only inside a construction
callback. Validate all existing catalog, role and inventory guards once on its
retained connection, with the schema migration lock followed by the audit
mutation lock. This ordering matches migrations. Retain both locks until the
callback finishes; release audit then schema before returning owned adapters.

Additional adapters connect independently with the same protected URL, require
UTF8 and compare the resolved database and role identities, schema/search path,
server endpoint and postmaster start time. The retained validation connection
must still respond. The capability has private fields and cannot be cloned,
sent to workers or escape the callback. No arbitrary-client constructor is
exposed. No static, thread-local or process-wide validation cache exists.

Construction does not run database work through the new adapters. Bind the HTTP
listener and start consumers only after the callback has returned and released
its locks. Error and panic paths also release locks. PostgreSQL releases locks
on a lost session. Ordinary string-based openings and consumer reconnections
continue to run the complete validation boundary independently.

## Consequences

Startup performs one complete validation during ordinary composition, plus any
independent legacy cutover check required by local imported files. Adapter SQL,
permissions, immutable evidence checks and runtime reconnection behavior stay
unchanged. The scoped lock temporarily excludes cooperating mutations while
construction finishes. Arbitrary concurrent administrator DDL remains outside
this construction contract, as it does for existing runtime adapters.

Regression checks must prove separate adapter connections, full rejection of
unrelated corruption and unsafe roles, changed target rejection, migration-lock
ordering and lock release after success, error and panic. Independent openings
and reconnecting consumers must continue to detect new corruption. A populated
HTTP restart remains the acceptance check for the practical startup budget.
