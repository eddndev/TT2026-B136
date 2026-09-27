import { test } from 'node:test';
import assert from 'node:assert/strict';
import { activityResourcesApi } from '../src/lib/activity-resources-api.mjs';
import { caseApi } from '../src/lib/case-api.mjs';
import { activityPrepared, activityView, clone } from './fixtures/resource-activity-unit.mjs';

const id = (n) => `e0000000-0000-4000-8000-${String(n).padStart(12, '0')}`;
function example(kind = 'hearing', count = 1) {
  const draft = activityPrepared(kind);
  const view = activityView(draft);
  return {
    case_id: draft.case_id,
    target: { kind, id: draft.selection.target.id },
    checked_at: clone(view.checked_at),
    associations: Array.from({ length: count }, (_, n) => {
      const row = clone(view);
      row.association.id = id(n + 1);
      return row;
    }),
    has_more: false,
    next_after_id: null,
  };
}
const apiFor = (page, request = async () => clone(page)) =>
  activityResourcesApi(request, page.case_id, clone(page.target));

for (const kind of ['hearing', 'deadline'])
  test(`inverse ${kind} reads retain target identity, current observation and older linked capture`, async () => {
    const page = example(kind),
      calls = [];
    const api = apiFor(page, async (path, options) => {
      calls.push({ path, options });
      return clone(page);
    });
    assert.deepEqual(await api.list(), page);
    const url = new URL(calls[0].path, 'https://example.test');
    assert.equal(
      url.pathname,
      `/cases/${page.case_id}/${kind === 'hearing' ? 'hearings' : 'deadlines'}/${page.target.id}/resource-associations`,
    );
    assert.equal(url.searchParams.get('status'), 'linked');
    assert.equal(url.searchParams.get('limit'), '20');
    assert.ok(calls[0].options === undefined || calls[0].options.method === 'GET');
    assert.equal(page.associations[0].association.selection.target.revision, 1);
    assert.equal(page.associations[0].current_target.record.revision, 2);
  });

test('inverse pagination preserves explicit all status and exclusive cursor across resources', async () => {
  const page = example('deadline', 2),
    calls = [];
  const first = page.associations[0].association.resource_id;
  page.associations[1] = JSON.parse(JSON.stringify(page.associations[1]).replaceAll(first, id(99)));
  page.has_more = true;
  page.next_after_id = page.associations.at(-1).association.id;
  const api = apiFor(page, async (path) => {
    calls.push(path);
    return clone(page);
  });
  assert.deepEqual(await api.list({ status: 'all', limit: 2 }), page);
  page.associations = [];
  page.has_more = false;
  page.next_after_id = null;
  assert.deepEqual(await api.list({ status: 'all', limit: 2, afterId: id(2) }), page);
  const parameters = new URL(calls[1], 'https://example.test').searchParams;
  assert.equal(parameters.get('status'), 'all');
  assert.equal(parameters.get('limit'), '2');
  assert.equal(parameters.get('after_id'), id(2));
});

test('an empty inverse page still validates exact case, typed target and UTC observation', async () => {
  const page = example('hearing', 0);
  assert.deepEqual(await apiFor(page).list(), page);
  for (const alter of [
    (value) => {
      value.case_id = id(91);
    },
    (value) => {
      value.target.kind = 'deadline';
    },
    (value) => {
      value.target.id = id(92);
    },
    (value) => {
      value.checked_at = null;
    },
    (value) => {
      value.checked_at.offset_seconds = 3600;
    },
    (value) => {
      value.has_more = true;
    },
    (value) => {
      value.next_after_id = id(1);
    },
    (value) => {
      value.unexpected = true;
    },
  ]) {
    const changed = clone(page);
    alter(changed);
    await assert.rejects(apiFor(page, async () => changed).list());
  }
});

test('inverse rows reject foreign targets, substituted captures, status and observation mismatch', async () => {
  const page = example('deadline');
  for (const alter of [
    (value) => {
      value.associations[0].association.case_id = id(91);
    },
    (value) => {
      value.associations[0].association.selection.target.id = id(92);
    },
    (value) => {
      value.associations[0].association.sources.resource.receipt.capture_digest = '1'.repeat(64);
    },
    (value) => {
      value.associations[0].association.selection.target.capture_digest = '2'.repeat(64);
    },
    (value) => {
      value.associations[0].checked_at.nanosecond++;
    },
    (value) => {
      value.checked_at.nanosecond++;
    },
    (value) => {
      value.associations[0].current_target.record.id = id(93);
    },
    (value) => {
      value.associations[0].current_target.record.operational.checked_at.nanosecond++;
    },
  ]) {
    const changed = clone(page);
    alter(changed);
    await assert.rejects(apiFor(page, async () => changed).list());
  }
  await assert.rejects(apiFor(page).list({ status: 'unlinked' }));
});

test('inverse pages reject duplicate order, excess rows and unusable continuations', async () => {
  const page = example('hearing', 2);
  for (const alter of [
    (value) => {
      value.associations.reverse();
    },
    (value) => {
      value.associations[1] = clone(value.associations[0]);
    },
    (value) => {
      value.has_more = true;
      value.next_after_id = id(99);
    },
    (value) => {
      value.has_more = true;
      value.next_after_id = null;
    },
    (value) => {
      value.next_after_id = id(2);
    },
  ]) {
    const changed = clone(page);
    alter(changed);
    await assert.rejects(apiFor(page, async () => changed).list({ limit: 2 }));
  }
  await assert.rejects(apiFor(page).list({ limit: 1 }));
  await assert.rejects(apiFor(page).list({ afterId: id(1) }));
  page.has_more = true;
  page.next_after_id = id(2);
  await assert.rejects(apiFor(page).list({ limit: 3 }));
});

test('inverse inputs fail before transport and disposal rejects both late success and failure', async () => {
  const page = example();
  let calls = 0;
  const api = apiFor(page, async () => {
    calls++;
    return page;
  });
  for (const query of [
    { limit: 0 },
    { limit: 101 },
    { limit: 1.5 },
    { afterId: 'bad' },
    { status: 'active' },
  ])
    await assert.rejects(api.list(query));
  for (const target of [
    { kind: 'notification', id: id(1) },
    { kind: 'hearing', id: 'bad' },
  ])
    assert.throws(() =>
      activityResourcesApi(
        async () => {
          calls++;
        },
        page.case_id,
        target,
      ),
    );
  assert.equal(calls, 0);
  for (const failed of [false, true]) {
    let finish;
    const scoped = apiFor(
      page,
      () =>
        new Promise((resolve, reject) => {
          finish = failed ? reject : resolve;
        }),
    );
    const pending = scoped.list();
    scoped.dispose();
    finish(failed ? new Error('old request') : page);
    await assert.rejects(pending, /abierta|termin|cerrad/i);
    await assert.rejects(scoped.list());
  }
});

test('case API exposes the inverse read client through the existing scoped transport', async () => {
  const page = example();
  const api = caseApi(async () => clone(page));
  assert.deepEqual(await api.caseActivityResources(page.case_id, page.target).list(), page);
});
