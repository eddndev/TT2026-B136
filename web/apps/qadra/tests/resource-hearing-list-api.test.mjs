import test from 'node:test';
import assert from 'node:assert/strict';
import { resourceHearingsApi } from '../src/lib/resource-hearing-api.mjs';
import {
  clone,
  resourceHearingPrepared,
  resourceHearingResult,
} from './fixtures/resource-hearing-unit.mjs';

const foreign = 'f0000000-0000-4000-8000-000000000099';
function fixture(count = 1) {
  const draft = resourceHearingPrepared();
  return {
    case_id: draft.command.case_id,
    resource_id: draft.command.resource_id,
    items: Array.from({ length: count }, (_, index) => {
      const d = clone(draft);
      d.command.hearing_id = `f0000000-0000-4000-8000-${(index + 1).toString(16).padStart(12, '0')}`;
      return resourceHearingResult(d);
    }),
    has_more: false,
    next_after_id: null,
  };
}
function client(value = fixture()) {
  const calls = [];
  const scope = fixture();
  const api = resourceHearingsApi(
    async (path, options) => {
      calls.push({ path, options });
      return clone(value);
    },
    scope.case_id,
    scope.resource_id,
  );
  return { api, calls };
}

test('own resource hearing list reads original pairs with bounded ascending cursor', async () => {
  const page = fixture(2);
  page.has_more = true;
  page.next_after_id = page.items[1].hearing.id;
  const { api, calls } = client(page);
  assert.deepEqual(await api.list({ limit: 2 }), page);
  assert.equal(
    calls[0].path,
    `/cases/${page.case_id}/procedural-resources/${page.resource_id}/activities/resource-hearings?limit=2`,
  );
  assert.equal(calls[0].options?.method || 'GET', 'GET');
  const next = fixture();
  next.items[0] = page.items[1];
  const second = client(next);
  assert.deepEqual(await second.api.list({ afterId: page.items[0].hearing.id }), next);
  assert.match(second.calls[0].path, /\?limit=10&after_id=f0000000-0000-4000-8000-000000000001$/);
  assert.deepEqual(await client(fixture(0)).api.list(), fixture(0));
});

test('own list refuses invalid limits, noncanonical cursors and unrelated filters', async () => {
  for (const query of [
    { limit: 0 },
    { limit: 21 },
    { limit: 1.5 },
    { afterId: foreign.toUpperCase() },
    { afterId: null },
    { status: 'linked' },
  ]) {
    const { api, calls } = client();
    await assert.rejects(api.list(query));
    assert.equal(calls.length, 0);
  }
});

test('own list rejects foreign, duplicated, unordered, partial and corrupt page captures', async () => {
  for (const alter of [
    (p) => {
      p.case_id = foreign;
    },
    (p) => {
      p.resource_id = foreign;
    },
    (p) => {
      p.items.reverse();
    },
    (p) => {
      p.items[1] = clone(p.items[0]);
    },
    (p) => {
      p.items[0].origin.operation_id = foreign;
    },
    (p) => {
      p.items[0].association.status = 'unlinked';
    },
    (p) => {
      p.next_after_id = foreign;
    },
    (p) => {
      p.has_more = true;
    },
    (p) => {
      p.has_more = true;
      p.next_after_id = p.items[0].hearing.id;
    },
    (p) => {
      p.has_more = true;
      p.next_after_id = p.items[1].hearing.id;
      p.items.pop();
    },
    (p) => {
      p.has_more = 'false';
    },
    (p) => {
      p.extra = true;
    },
  ]) {
    const page = fixture(2);
    alter(page);
    await assert.rejects(client(page).api.list({ limit: 2 }));
  }
  await assert.rejects(client(fixture(2)).api.list({ limit: 1 }));
  await assert.rejects(client(fixture()).api.list({ afterId: fixture().items[0].hearing.id }));
});

test('disposed list refuses late pages and further requests', async () => {
  const page = fixture();
  let resolve;
  const api = resourceHearingsApi(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
    page.case_id,
    page.resource_id,
  );
  const pending = api.list();
  api.dispose();
  resolve(page);
  await assert.rejects(pending);
  await assert.rejects(api.list());
});
