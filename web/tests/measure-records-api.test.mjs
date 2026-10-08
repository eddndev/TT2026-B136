import test from 'node:test';
import assert from 'node:assert/strict';
import { caseApi } from '../src/lib/case-api.mjs';
import {
  measureCaseId,
  measureId,
  otherMeasureId,
  foreignMeasureId,
  measureRecord,
  measurePage,
  clone,
} from './fixtures/measure-records.mjs';

const base = `/cases/${measureCaseId}/measures`;
function client(reply) {
  const calls = [];
  const api = caseApi(async (path, options) => {
    calls.push({ path, options: clone(options) });
    return typeof reply === 'function' ? reply(path, options) : clone(reply);
  }).caseMeasures(measureCaseId);
  return { api, calls };
}

test('measure current reads preserve actual m1 m2 and c1 records and their owner histories', async () => {
  for (const family of ['m1', 'm2', 'c1']) {
    const detail = measureRecord({ family }),
      { api, calls } = client(detail);
    assert.deepEqual(await api.get(measureId), detail);
    assert.deepEqual(calls, [{ path: `${base}/${measureId}`, options: undefined }]);
    assert.equal(detail.record.capture.result.values.validity.start.offset_seconds, null);
    assert.equal(detail.record.capture.result.values.validity.end, null);
  }
});

test('exact read keeps an older m1 capture after a c1 head marked entered in error', async () => {
  const old = measureRecord(),
    current = measureRecord({ family: 'c1', validity: 'entered_in_error' });
  const { api, calls } = client((path) => clone(path.includes('/revisions/') ? old : current));
  assert.deepEqual(await api.get(measureId), current);
  assert.deepEqual(await api.exact(old.reference), old);
  assert.deepEqual(calls[1], {
    path: `${base}/${measureId}/revisions/1?capture_digest=${old.reference.capture_digest}`,
    options: undefined,
  });
  assert.equal(calls.length, 2);
});

test('historical reads need only measure authority and do not demand an active case context', async () => {
  const detail = measureRecord({ family: 'c1' });
  const { api, calls } = client((path, options) => {
    assert.equal(path.startsWith(`${base}/`), true);
    assert.equal(options, undefined);
    return clone(detail);
  });
  assert.deepEqual(await api.get(measureId), detail);
  assert.deepEqual(await api.exact(detail.reference), detail);
  assert.equal(calls.length, 2);
});

test('exact selectors reject unknown properties and noncanonical identities before transport', async () => {
  for (const change of [
    (v) => {
      v.id = 'invalid';
    },
    (v) => {
      v.id = v.id.toUpperCase();
    },
    (v) => {
      v.revision = 0;
    },
    (v) => {
      v.revision = 4294967296;
    },
    (v) => {
      v.capture_digest = 'bad';
    },
    (v) => {
      delete v.capture_digest;
    },
    (v) => {
      v.current = true;
    },
  ]) {
    const detail = measureRecord(),
      selected = clone(detail.reference),
      { api, calls } = client(detail);
    change(selected);
    await assert.rejects(api.exact(selected));
    assert.equal(calls.length, 0);
  }
  const { api, calls } = client(measureRecord());
  await assert.rejects(api.get('invalid'));
  assert.equal(calls.length, 0);
});

test('returned measure identity revision digest and family bind the selected record', async () => {
  for (const change of [
    (v) => {
      v.case_id = foreignMeasureId;
    },
    (v) => {
      v.reference.id = otherMeasureId;
    },
    (v) => {
      v.reference.revision++;
    },
    (v) => {
      v.reference.capture_digest = 'f'.repeat(64);
    },
    (v) => {
      v.family = 'unknown';
    },
    (v) => {
      v.record.family = 'c1';
    },
    (v) => {
      v.record.capture.family = 'm2';
    },
    (v) => {
      v.record.capture.case_id = foreignMeasureId;
    },
    (v) => {
      v.record.capture.capture_digest = 'f'.repeat(64);
    },
    (v) => {
      v.record.capture.result.id = otherMeasureId;
    },
    (v) => {
      v.record.capture.result.revision++;
    },
    (v) => {
      v.record.owner.operation_id = foreignMeasureId;
    },
    (v) => {
      v.last_action = 'cease';
    },
    (v) => {
      v.validity = 'entered_in_error';
    },
  ]) {
    const detail = measureRecord(),
      selected = clone(detail.reference);
    change(detail);
    await assert.rejects(client(detail).api.exact(selected), change.toString());
  }
  await assert.rejects(client(measureRecord({ id: otherMeasureId })).api.get(measureId));
});

test('visible subject and declared result cannot disagree with their exact retained sources', async () => {
  for (const change of [
    (v) => {
      v.values.subject.id = foreignMeasureId;
    },
    (v) => {
      v.values.subject.revision++;
    },
    (v) => {
      v.values.subject.values_digest = 'f'.repeat(64);
    },
    (v) => {
      v.sources.subject.case_id = foreignMeasureId;
    },
    (v) => {
      v.projection.subject.id = foreignMeasureId;
    },
    (v) => {
      v.projection.subject.revision++;
    },
    (v) => {
      v.projection.subject.case_id = foreignMeasureId;
    },
    (v) => {
      v.projection.subject.display_name = 'Another person';
    },
    (v) => {
      v.projection.subject.kind = 'institutional_body';
    },
    (v) => {
      v.values.kind = 'unknown';
    },
    (v) => {
      v.values.conditions = '';
    },
    (v) => {
      delete v.values.validity.end;
    },
    (v) => {
      v.sources.subject = null;
    },
  ]) {
    const detail = measureRecord();
    change(detail.record.capture.result);
    await assert.rejects(client(detail).api.get(measureId), change.toString());
  }
});

test('record history uses bounded array containers without replacing the selected capture', async () => {
  for (const change of [
    (v) => {
      v.record_history.records.judicial.groups = null;
    },
    (v) => {
      v.record_history.records.administrative = {};
    },
    (v) => {
      v.record_history.decisions = null;
    },
    (v) => {
      v.record_history.extra = [];
    },
  ]) {
    const detail = measureRecord({ family: 'c1' });
    change(detail);
    await assert.rejects(client(detail).api.get(measureId));
  }
});

test('measure list uses ascending exclusive pagination and preserves marked heads', async () => {
  const page = measurePage(),
    { api, calls } = client(page);
  assert.deepEqual(await api.list(), page);
  assert.deepEqual(calls, [{ path: `${base}?limit=10`, options: undefined }]);
  const first = measurePage();
  first.has_more = true;
  first.next_after_id = otherMeasureId;
  assert.deepEqual(await client(first).api.list({ limit: 2 }), first);
  const next = { case_id: measureCaseId, items: [], has_more: false, next_after_id: null };
  const follow = client(next);
  assert.deepEqual(await follow.api.list({ limit: 20, afterId: otherMeasureId }), next);
  assert.equal(follow.calls[0].path, `${base}?limit=20&after_id=${otherMeasureId}`);
  assert.equal(page.items[1].validity, 'entered_in_error');
});

test('measure list rejects bad queries and inconsistent page scope order or cursor', async () => {
  for (const query of [
    { limit: 0 },
    { limit: 21 },
    { limit: 1.5 },
    { afterId: null },
    { afterId: foreignMeasureId.toUpperCase() },
    { validity: 'valid' },
  ]) {
    const { api, calls } = client(measurePage());
    await assert.rejects(api.list(query));
    assert.equal(calls.length, 0);
  }
  for (const change of [
    (v) => {
      v.case_id = foreignMeasureId;
    },
    (v) => {
      v.items.reverse();
    },
    (v) => {
      v.items[1] = clone(v.items[0]);
    },
    (v) => {
      v.items[0].case_id = foreignMeasureId;
    },
    (v) => {
      v.has_more = 'false';
    },
    (v) => {
      v.next_after_id = otherMeasureId;
    },
    (v) => {
      v.has_more = true;
    },
    (v) => {
      v.has_more = true;
      v.next_after_id = measureId;
    },
  ]) {
    const page = measurePage();
    change(page);
    await assert.rejects(client(page).api.list({ limit: 2 }), change.toString());
  }
  await assert.rejects(client(measurePage()).api.list({ limit: 1 }));
  await assert.rejects(client(measurePage()).api.list({ afterId: measureId }));
});

test('authorization absence and transport errors remain failures without fallback or retries', async () => {
  for (const status of [401, 403, 404, 500]) {
    const failure = Object.assign(new Error('Read rejected'), {
      status,
      code: 'measure_record_not_found',
    });
    for (const method of ['list', 'get', 'exact']) {
      const { api, calls } = client(() => {
        throw failure;
      });
      const argument =
        method === 'get' ? measureId : method === 'exact' ? measureRecord().reference : undefined;
      await assert.rejects(api[method](argument), (error) => error === failure);
      assert.equal(calls.length, 1);
    }
  }
});

test('pending exact reads retain their selector and disposal rejects late success or failure', async () => {
  const detail = measureRecord(),
    selected = clone(detail.reference);
  let release;
  const { api, calls } = client(
    () =>
      new Promise((resolve) => {
        release = resolve;
      }),
  );
  const pending = api.exact(selected);
  selected.revision = 2;
  selected.capture_digest = 'f'.repeat(64);
  release(clone(detail));
  assert.deepEqual(await pending, detail);
  assert.equal(calls.length, 1);
  for (const method of ['list', 'get', 'exact']) {
    for (const failed of [false, true]) {
      let resolve, reject;
      const next = client(
        () =>
          new Promise((yes, no) => {
            resolve = yes;
            reject = no;
          }),
      );
      const argument =
        method === 'get' ? measureId : method === 'exact' ? detail.reference : undefined;
      const result = next.api[method](argument);
      next.api.dispose();
      if (failed) reject(Object.assign(new Error('Missing'), { status: 404 }));
      else resolve(clone(method === 'list' ? measurePage() : detail));
      await assert.rejects(result);
      await assert.rejects(next.api[method](argument));
      assert.equal(next.calls.length, 1);
    }
  }
});
