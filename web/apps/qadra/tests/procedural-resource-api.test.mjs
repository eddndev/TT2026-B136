import { test } from 'node:test';
import assert from 'node:assert/strict';
import { proceduralResourcesApi } from '../src/lib/procedural-resources-api.mjs';
import {
  resourceMatches,
  readResourceSubmission,
} from '../src/lib/procedural-resource-submission.mjs';
import {
  resourceCaseId,
  resourceActor,
  resourceId,
  resourceActId,
  resourcePrepared,
  resourceRecord,
  resourceCommandFixture,
} from './fixtures/procedural-resource-unit.mjs';
const base = `/cases/${resourceCaseId}/procedural-resources`;
test('resource queries bind case, exact revision, filters and pagination', async () => {
  const row = resourceRecord(),
    calls = [];
  let response = { resources: [row], has_more: false, next_after_id: null };
  const api = proceduralResourcesApi(async (path) => {
    calls.push(path);
    return response;
  }, resourceCaseId);
  assert.equal((await api.list({ kind: 'appeal', status: 'active' })).resources[0], row);
  assert.match(calls[0], /kind=appeal/);
  response = { ...response, has_more: true, next_after_id: resourceId };
  await assert.rejects(() => api.list());
  response = row;
  assert.equal(await api.revision(resourceId, 1), row);
  await assert.rejects(() => api.revision(resourceId, 2));
  response = { ...row, case_id: resourceId };
  await assert.rejects(() => api.get(resourceId));
  response = { resources: [row], has_more: false, next_after_id: null };
  await assert.rejects(() => api.list({ kind: 'revocation' }));
  await assert.rejects(() => api.list({ afterId: resourceId }));
  response = { revisions: [row], has_more: false, next_before_revision: null };
  assert.equal((await api.history(resourceId)).revisions[0], row);
  await assert.rejects(() => api.history(resourceId, { beforeRevision: 1 }));
  assert.ok(calls.every((path) => path.startsWith(base)));
});
test('prepare rejects substituted commands, actors and exact source captures', async () => {
  const command = resourceCommandFixture(),
    prepared = resourcePrepared(command);
  let response = prepared;
  const api = proceduralResourcesApi(async () => response, resourceCaseId);
  assert.equal(await api.prepare(command, resourceActor), prepared);
  for (const mutate of [
    (v) => {
      v.command.change.values.title = 'Otra';
    },
    (v) => {
      v.recorded_by.email = 'other@example.test';
    },
    (v) => {
      v.sources.resolution.case_id = resourceId;
    },
    (v) => {
      v.sources.resolution.revision = 2;
    },
    (v) => {
      v.sources.supports[0].digest = 'b'.repeat(64);
    },
    (v) => {
      v.observed_stage.case_id = resourceId;
    },
  ]) {
    response = structuredClone(prepared);
    mutate(response);
    await assert.rejects(() => api.prepare(command, resourceActor));
  }
});
test('six commands use explicit routes and compare the complete prepared receipt', async () => {
  for (const [action, suffix, method] of [
    ['register', '', 'POST'],
    ['correct', `/${resourceId}`, 'PUT'],
    ['record_act', `/${resourceId}/acts`, 'POST'],
    ['correct_act', `/${resourceId}/acts/${resourceActId}`, 'PUT'],
    ['archive', `/${resourceId}/archive`, 'POST'],
    ['reactivate', `/${resourceId}/reactivation`, 'POST'],
  ]) {
    const prepared = resourcePrepared(resourceCommandFixture(action));
    const expected = resourceRecord(prepared);
    let result = expected;
    const api = proceduralResourcesApi(async (path, options) => {
      assert.equal(path, base + suffix);
      assert.equal(options.method, method);
      assert.deepEqual(options.data, {
        command: prepared.command,
        expected_submission_digest: prepared.submission_digest,
      });
      return result;
    }, resourceCaseId);
    assert.equal(await api.submit(prepared), expected);
    result = structuredClone(expected);
    result.receipt.submission_digest = 'a'.repeat(64);
    await assert.rejects(() => api.submit(prepared));
  }
});
test('uncertain submissions only reconcile an exact matching historical receipt', async () => {
  const prepared = resourcePrepared(resourceCommandFixture('correct_act'));
  const row = resourceRecord(prepared);
  assert.equal(resourceMatches(row, prepared), true);
  for (const mutate of [
    (v) => {
      v.recorded_by.email = 'other@example.test';
    },
    (v) => {
      v.receipt.previous.capture_digest = 'a'.repeat(64);
    },
    (v) => {
      v.act.previous.revision = 1;
    },
    (v) => {
      v.act.values.kind = 'admission';
    },
    (v) => {
      v.recorded_administration.title = 'Changed';
    },
  ]) {
    const bad = structuredClone(row);
    mutate(bad);
    assert.equal(resourceMatches(bad, prepared), false);
  }
  let reads = 0;
  const result = await readResourceSubmission(
    {
      revision: async (id, rev) => {
        reads++;
        assert.equal(id, resourceId);
        assert.equal(rev, 4);
        return row;
      },
    },
    prepared,
  );
  assert.equal(result.state, 'matched');
  assert.equal(reads, 1);
  assert.deepEqual(
    await readResourceSubmission(
      {
        revision: async () => {
          throw Object.assign(new Error('missing'), {
            status: 404,
            code: 'procedural_resource_not_found',
          });
        },
      },
      prepared,
    ),
    { state: 'absent' },
  );
});
test('a disposed case scope rejects late responses and never starts another request', async () => {
  let resolve,
    calls = 0;
  const api = proceduralResourcesApi(() => {
    calls++;
    return new Promise((done) => {
      resolve = done;
    });
  }, resourceCaseId);
  const pending = api.get(resourceId);
  api.dispose();
  resolve(resourceRecord());
  await assert.rejects(() => pending);
  await assert.rejects(() => api.get(resourceId));
  assert.equal(calls, 1);
});
