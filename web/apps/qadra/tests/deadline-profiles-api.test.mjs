import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlineProfilesApi } from '../src/lib/deadline-profiles-api.mjs';
import { profile, id } from './fixtures/deadline-unit.mjs';

test('profile exact reads preserve corpus, conditions and immutable scope', async () => {
  const row = profile(),
    calls = [],
    api = deadlineProfilesApi(async (path) => {
      calls.push(path);
      return structuredClone(row);
    }, id(1));
  assert.deepEqual(await api.revision(id(2), 1), row);
  assert.equal(calls[0], `/cases/${id(1)}/deadline-profiles/${id(2)}/revisions/1`);
  for (const mutate of [
    (p) => {
      p.collection.case_id = id(9);
    },
    (p) => {
      p.definition.scope.case_id = id(9);
    },
    (p) => {
      p.definition.conditions[0].reference_ids = [id(99)];
    },
    (p) => {
      p.definition.examples[0].expected.outcome.instant.nanosecond = 1000000000;
    },
    (p) => {
      p.definition.trigger.field = 'invented';
    },
    (p) => {
      p.definition.examples = [];
    },
    (p) => {
      p.receipt.action = 'retire';
    },
  ]) {
    const value = structuredClone(row);
    mutate(value);
    await assert.rejects(() => deadlineProfilesApi(async () => value, id(1)).revision(id(2), 1));
  }
});
test('global collection cannot expose a private profile and pages check continuation', async () => {
  const row = profile();
  row.collection = { kind: 'global' };
  await assert.rejects(() => deadlineProfilesApi(async () => row).get(id(2)));
  const summary = Object.fromEntries(
    ['id', 'revision', 'status', 'algorithm', 'definition_digest', 'scope'].map((k) => [k, row[k]]),
  );
  summary.title = row.definition.title;
  const page = {
    collection: { kind: 'case', case_id: id(1) },
    profiles: [summary],
    has_more: false,
    next_after_id: null,
  };
  assert.deepEqual(await deadlineProfilesApi(async () => page, id(1)).list(), page);
  page.has_more = true;
  await assert.rejects(() => deadlineProfilesApi(async () => page, id(1)).list({ limit: 1 }));
});

test('global profiles remain available through both authorized collections with exact history', async () => {
  const row = profile();
  row.scope = {
    kind: 'global',
    value: {
      title: 'Ambito declarado',
      jurisdiction: 'federal',
      entity_codes: ['09'],
      authority: 'Autoridad',
      organ: 'Organo',
      territory: 'Territorio',
      use_description: 'Uso declarado',
    },
  };
  row.definition.scope = structuredClone(row.scope);
  for (const caseId of [null, id(1)]) {
    row.collection = caseId === null ? { kind: 'global' } : { kind: 'case', case_id: caseId };
    let requested;
    const api = deadlineProfilesApi(async (path) => {
      requested = path;
      return structuredClone(row);
    }, caseId);
    assert.deepEqual(await api.get(id(2)), row);
    assert.equal(
      requested,
      `${caseId === null ? '' : `/cases/${caseId}`}/deadline-profiles/${id(2)}`,
    );
    const { collection, definition, ...header } = structuredClone(row);
    const page = { collection, revisions: [header], has_more: false, next_before_revision: null };
    assert.deepEqual(await deadlineProfilesApi(async () => page, caseId).history(id(2)), page);
  }
});
