import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlineAuthorV2, deadlineReceiptVersion } from '../src/lib/deadline-tracking.mjs';
import { v2Record, technicalRecord, ids } from './fixtures/deadline-v2-unit.mjs';
import { caseId, parseVersion, sourceEventVersion, rejects } from './deadline-tracking-support.mjs';

test('tagged authors preserve exact human or technical identity and zero UUIDs', () => {
  const user = { kind: 'user', id: ids(0), email: 'staff@example.test' };
  const technical = technicalRecord().recorded_by;
  assert.deepEqual(deadlineAuthorV2(user), user);
  assert.deepEqual(deadlineAuthorV2(v2Record().recorded_by), v2Record().recorded_by);
  assert.deepEqual(deadlineAuthorV2(technical), technical);
  assert.equal(technical.kind, 'technical');
  assert.equal(Object.hasOwn(technical, 'id'), false);
  const maximum = { ...user, email: '\u{e9}'.repeat(320) };
  assert.deepEqual(deadlineAuthorV2(maximum), maximum);
});

test('authors reject absent tags, unknown fields, malformed text and unsupported policy', () => {
  rejects(v2Record().recorded_by, deadlineAuthorV2, [
    [
      'missing tag',
      (v) => {
        delete v.kind;
      },
    ],
    [
      'unknown tag',
      (v) => {
        v.kind = 'owner';
      },
    ],
    [
      'bad UUID',
      (v) => {
        v.id = 'invalid';
      },
    ],
    [
      'extra field',
      (v) => {
        v.role = 'owner';
      },
    ],
    ...[
      '',
      ' staff@example.test',
      'staff@example.test ',
      'x\n@y',
      'x\0@y',
      '\u{e9}'.repeat(321),
      '\ud800',
    ].map((email) => [
      'noncanonical email',
      (v) => {
        v.email = email;
      },
    ]),
  ]);
  rejects(technicalRecord().recorded_by, deadlineAuthorV2, [
    [
      'fake user',
      (v) => {
        v.id = ids(4);
      },
    ],
    [
      'unknown service',
      (v) => {
        v.service = 'other';
      },
    ],
    ...[0, 2, '1', null].map((policy) => [
      'bad policy',
      (v) => {
        v.policy_version = policy;
      },
    ]),
  ]);
  for (const raw of [null, [], 'user', 1]) assert.throws(() => deadlineAuthorV2(raw));
});

test('version objects preserve V1, human V2 and both technical causes', () => {
  assert.deepEqual(parseVersion({ kind: 'v1' }), { kind: 'v1' });
  for (const value of [v2Record().receipt.version, sourceEventVersion()])
    assert.deepEqual(parseVersion(value), value);
  const bootstrap = sourceEventVersion();
  bootstrap.cause = { kind: 'legacy_bootstrap', job_id: ids(0), policy_version: 1 };
  assert.deepEqual(parseVersion(bootstrap), bootstrap);
  const human = v2Record().receipt.version;
  assert.equal(human.predecessor, null);
  assert.equal(human.cause, null);
});

test('versions reject incomplete V2 rather than falling back to V1', () => {
  for (const raw of [null, [], {}, 'v2', 2]) assert.throws(() => parseVersion(raw));
  rejects(sourceEventVersion(), parseVersion, [
    [
      'unknown version',
      (v) => {
        v.kind = 'v3';
      },
    ],
    [
      'missing digest',
      (v) => {
        delete v.observations_digest;
      },
    ],
    [
      'bad digest',
      (v) => {
        v.observations_digest = 'a';
      },
    ],
    [
      'missing predecessor',
      (v) => {
        delete v.predecessor;
      },
    ],
    [
      'bad predecessor',
      (v) => {
        v.predecessor = [];
      },
    ],
    [
      'incomplete predecessor',
      (v) => {
        delete v.predecessor.capture_digest;
      },
    ],
    [
      'extra predecessor',
      (v) => {
        v.predecessor.revision = 1;
      },
    ],
    [
      'missing cause',
      (v) => {
        delete v.cause;
      },
    ],
    [
      'unknown cause',
      (v) => {
        v.cause.kind = 'manual';
      },
    ],
    [
      'bad job',
      (v) => {
        v.cause.job_id = null;
      },
    ],
    [
      'extra cause',
      (v) => {
        v.cause.author = 'worker';
      },
    ],
    [
      'extra version',
      (v) => {
        v.accepted = true;
      },
    ],
  ]);
  assert.throws(() => parseVersion({ kind: 'v1', cause: null }));
  for (const policy_version of [0, 2, '1', null]) {
    const value = sourceEventVersion();
    value.cause = { kind: 'legacy_bootstrap', job_id: ids(0), policy_version };
    assert.throws(() => parseVersion(value));
  }
});

test('event sequences remain canonical decimal strings above JavaScript safe integers', () => {
  for (const sequence of ['1', '9007199254740993', '9223372036854775807']) {
    const value = sourceEventVersion();
    value.cause.event.sequence = sequence;
    value.cause.event.revision = 4294967295;
    assert.deepEqual(parseVersion(value), value);
    assert.equal(parseVersion(value).cause.event.sequence, sequence);
  }
  rejects(sourceEventVersion(), parseVersion, [
    ...[
      0,
      1,
      9007199254740992,
      '0',
      '-1',
      '+1',
      '01',
      ' 1',
      '1.0',
      '1e3',
      '9223372036854775808',
      '',
      null,
    ].map((sequence) => [
      'bad sequence',
      (v) => {
        v.cause.event.sequence = sequence;
      },
    ]),
    ...[0, -1, 1.5, 4294967296, '1', null].map((revision) => [
      'bad revision',
      (v) => {
        v.cause.event.revision = revision;
      },
    ]),
    [
      'missing nullable field',
      (v) => {
        delete v.cause.event.hearing_id;
      },
    ],
    [
      'extra event',
      (v) => {
        v.cause.event.accepted = true;
      },
    ],
    [
      'unknown family',
      (v) => {
        v.cause.event.family = 'document';
      },
    ],
  ]);
});

test('source events preserve valid family scopes and reject cross-case substitutions', () => {
  for (const [family, scope, hearing] of [
    ['profile', null, null],
    ['profile', caseId, null],
    ['calendar', null, null],
    ['resolution', caseId, null],
    ['notification', caseId, null],
    ['hearing_result', caseId, ids(0)],
  ]) {
    const value = sourceEventVersion();
    Object.assign(value.cause.event, {
      family,
      case_id: scope,
      hearing_id: hearing,
      source_id: ids(0),
      operation_id: ids(0),
    });
    assert.deepEqual(parseVersion(value), value);
  }
  for (const [family, scope, hearing] of [
    ['profile', ids(9), null],
    ['profile', null, ids(0)],
    ['calendar', caseId, null],
    ['calendar', null, ids(0)],
    ['resolution', null, null],
    ['resolution', ids(9), null],
    ['resolution', caseId, ids(0)],
    ['notification', null, null],
    ['notification', ids(9), null],
    ['notification', caseId, ids(0)],
    ['hearing_result', caseId, null],
    ['hearing_result', ids(9), ids(0)],
  ]) {
    const value = sourceEventVersion();
    Object.assign(value.cause.event, { family, case_id: scope, hearing_id: hearing });
    assert.throws(() => parseVersion(value));
  }
  const zeroCase = sourceEventVersion();
  Object.assign(zeroCase.cause.event, { case_id: ids(0), source_id: ids(0), operation_id: ids(0) });
  assert.deepEqual(deadlineReceiptVersion(zeroCase, ids(0)), zeroCase);
});
