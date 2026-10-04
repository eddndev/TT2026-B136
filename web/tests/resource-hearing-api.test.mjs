import test from 'node:test';
import assert from 'node:assert/strict';
import { resourceHearingsApi } from '../src/lib/resource-hearing-api.mjs';
import { caseApi } from '../src/lib/case-api.mjs';
import {
  resourceHearingCreation,
  resourceHearingOverview,
  clone,
} from './fixtures/resource-hearing-unit.mjs';

const foreign = 'f0000000-0000-4000-8000-000000000099';
function client(value = resourceHearingCreation()) {
  const calls = [];
  const api = resourceHearingsApi(
    async (path, options) => {
      calls.push({ path, options });
      return clone(value);
    },
    value.hearing.case_id,
    value.hearing.resource_id,
  );
  return { api, calls };
}
function synchronizeTarget(value) {
  value.association.sources.target.record = clone(value.hearing);
}

test('exact resource hearing reads preserve original creation through their own scoped GET', async () => {
  const value = resourceHearingCreation({ withAct: true }),
    { api, calls } = client(value);
  const overview = resourceHearingOverview(value);
  assert.deepEqual(await api.exact(overview), value);
  assert.equal(calls.length, 1);
  assert.equal(
    calls[0].path,
    `/cases/${overview.case_id}/procedural-resources/${overview.resource_id}/activities/resource-hearings/${overview.id}/revisions/1`,
  );
  assert.equal(calls[0].options?.method || 'GET', 'GET');
  assert.equal(calls[0].options?.data, undefined);
  assert.equal(value.hearing.resource.revision, 1);
  assert.equal(value.hearing.act.resource_revision, 4);
  assert.equal(value.hearing.recorded_resource_head.revision, 5);
  assert.equal(value.association.revision, 1);
  assert.equal(value.hearing.sources.support.name, 'senalamiento-del-acto.pdf');
  assert.equal(Object.hasOwn(value.hearing, 'status'), false);
  assert.equal(Object.hasOwn(value.hearing, 'scheduling_context'), false);
});

test('case API keeps exact reads separate from explicit mutation methods', async () => {
  const value = resourceHearingCreation(),
    calls = [];
  const api = caseApi(async (path, options) => {
    calls.push({ path, options });
    return clone(value);
  });
  const scoped = api.caseResourceHearings(value.hearing.case_id, value.hearing.resource_id);
  assert.deepEqual(await scoped.exact(resourceHearingOverview(value)), value);
  assert.equal(typeof scoped.prepare, 'function');
  assert.equal(typeof scoped.submit, 'function');
  assert.equal(calls.length, 1);
  assert.equal(calls[0].options?.method || 'GET', 'GET');
});

test('invalid or foreign agenda selections are rejected before requesting', async () => {
  for (const alter of [
    (v) => {
      v.case_id = foreign;
    },
    (v) => {
      v.resource_id = foreign;
    },
    (v) => {
      v.id = v.id.toUpperCase();
    },
    (v) => {
      v.revision = 2;
    },
    (v) => {
      v.association_id = 'invalid';
    },
    (v) => {
      v.capture_digest = 'bad';
    },
    (v) => {
      v.kind = 'initial';
    },
    (v) => {
      v.participant_count = 33;
    },
    (v) => {
      v.scheduled_at = '2026-02-30T10:00:00Z';
    },
    (v) => {
      v.status = 'scheduled';
    },
    (v) => {
      v.resource_revision = 1;
    },
  ]) {
    const { api, calls } = client(),
      overview = resourceHearingOverview();
    alter(overview);
    await assert.rejects(api.exact(overview));
    assert.equal(calls.length, 0);
  }
});

test('exact response must match every captured overview field', async () => {
  const value = resourceHearingCreation();
  for (const [key, changed] of Object.entries({
    id: foreign,
    association_id: foreign,
    capture_digest: '1'.repeat(64),
    kind: 'written_revocation',
    scheduled_at: '2026-01-02T00:00:01Z',
    modality: 'videoconference',
    participant_count: 1,
  })) {
    const overview = resourceHearingOverview(value);
    overview[key] = changed;
    await assert.rejects(client(value).api.exact(overview), key);
  }
});

test('creation rejects altered origin and independently substituted original association', async () => {
  for (const alter of [
    (v) => {
      v.extra = true;
    },
    (v) => {
      v.submission_digest = '0'.repeat(64);
    },
    (v) => {
      v.origin.hearing_id = foreign;
    },
    (v) => {
      v.origin.resource_id = foreign;
    },
    (v) => {
      v.origin.case_id = foreign;
    },
    (v) => {
      v.origin.association_id = foreign;
    },
    (v) => {
      v.origin.operation_id = foreign;
    },
    (v) => {
      v.origin.capture_digest = '1'.repeat(64);
    },
    (v) => {
      v.origin.submission_digest = '2'.repeat(64);
    },
    (v) => {
      v.origin.extra = true;
    },
    (v) => {
      v.association.id = foreign;
    },
    (v) => {
      v.association.revision = 2;
    },
    (v) => {
      v.association.status = 'unlinked';
    },
    (v) => {
      v.association.receipt.action = 'unlink';
    },
    (v) => {
      v.association.receipt.operation_id = foreign;
    },
    (v) => {
      v.association.receipt.expected_resource_revision = 4;
    },
    (v) => {
      v.association.receipt.previous = { revision: 1, capture_digest: '1'.repeat(64) };
    },
    (v) => {
      v.association.recorded_by.email = 'other@example.test';
    },
    (v) => {
      v.association.recorded_at = '2026-01-01T00:00:01.123456788Z';
    },
    (v) => {
      v.association.recorded_administration.title = 'Different capture';
    },
    (v) => {
      v.association.recorded_resource_head.capture_digest = '2'.repeat(64);
    },
    (v) => {
      v.association.sources.resource.recorded_by.email = 'other@example.test';
    },
    (v) => {
      v.association.sources.act = null;
    },
    (v) => {
      v.association.sources.target.record.values.venue = 'Other venue';
    },
    (v) => {
      v.association.selection.target.capture_digest = '1'.repeat(64);
    },
    (v) => {
      v.association.selection.target.kind = 'hearing';
    },
    (v) => {
      v.association.selection.resource.revision = 2;
    },
    (v) => {
      v.association.reason = 'Unexpected reason';
    },
  ]) {
    const value = resourceHearingCreation({ withAct: true }),
      expected = resourceHearingOverview(value);
    alter(value);
    await assert.rejects(client(value).api.exact(expected), alter.toString());
  }
});

test('historical detail rejects malformed scheduling sources and participant bindings', async () => {
  for (const alter of [
    (h) => {
      h.extra = true;
    },
    (h) => {
      h.values.status = 'scheduled';
    },
    (h) => {
      h.values.venue = ' padded';
    },
    (h) => {
      h.values.note = '';
    },
    (h) => {
      h.values.scheduling_basis.statement = '';
    },
    (h) => {
      h.sources.support.name = 'Changed support.pdf';
    },
    (h) => {
      h.sources.support.policy = 'unknown';
    },
    (h) => {
      h.values.scheduling_basis.support.digest = '2'.repeat(64);
    },
    (h) => {
      h.sources.act = null;
    },
    (h) => {
      h.act.resource_revision = 6;
    },
    (h) => {
      h.recorded_resource_head.revision = 4;
    },
    (h) => {
      h.sources.resource.values.mode = { kind: 'known', value: 'oral' };
    },
    (h) => {
      h.sources.participants[0].case_id = foreign;
    },
    (h) => {
      h.sources.participants[0].revision = 3;
    },
    (h) => {
      h.sources.participants[0].values_digest = 'bad';
    },
    (h) => {
      h.sources.participants[0].directory_status = 'invented';
    },
    (h) => {
      h.sources.participants[0].subject = { id: foreign, revision: 1, values_digest: 'bad' };
    },
    (h) => {
      h.values.participants[1].participant_id = h.values.participants[0].participant_id;
    },
    (h) => {
      h.sources.participants.reverse();
    },
    (h) => {
      h.recorded_at = '2026-01-01T00:00:00+00:00';
    },
    (h) => {
      h.recorded_at = '2025-12-31T23:59:59Z';
    },
  ]) {
    const value = resourceHearingCreation({ withAct: true }),
      expected = resourceHearingOverview(value);
    alter(value.hearing);
    synchronizeTarget(value);
    await assert.rejects(client(value).api.exact(expected), alter.toString());
  }
});

test('both kinds and the zero and thirty-two participant boundaries remain readable', async () => {
  for (const kind of ['appeal_arguments', 'written_revocation']) {
    for (const participantCount of [0, 32]) {
      const value = resourceHearingCreation({ kind, participantCount });
      assert.deepEqual(await client(value).api.exact(resourceHearingOverview(value)), value);
    }
  }
});

test('closing an exact read invalidates delayed results and prevents further requests', async () => {
  let release,
    calls = 0;
  const value = resourceHearingCreation(),
    overview = resourceHearingOverview(value);
  const api = resourceHearingsApi(
    () => {
      calls++;
      return new Promise((resolve) => {
        release = resolve;
      });
    },
    overview.case_id,
    overview.resource_id,
  );
  const pending = api.exact(overview);
  api.dispose();
  release(value);
  await assert.rejects(pending);
  await assert.rejects(api.exact(overview));
  assert.equal(calls, 1);
});

test('an in-flight exact read retains its selected immutable overview', async () => {
  let release;
  const value = resourceHearingCreation(),
    overview = resourceHearingOverview(value);
  const api = resourceHearingsApi(
    () =>
      new Promise((resolve) => {
        release = resolve;
      }),
    overview.case_id,
    overview.resource_id,
  );
  const pending = api.exact(overview);
  overview.capture_digest = '1'.repeat(64);
  overview.resource_id = foreign;
  release(clone(value));
  assert.deepEqual(await pending, value);
});

test('failed exact reads retain transport errors without retries or fallback routes', async () => {
  const overview = resourceHearingOverview();
  for (const status of [401, 403, 404, 409, 500, 503]) {
    let calls = 0;
    const failure = Object.assign(new Error('Read failed'), { status, code: 'read_failed' });
    const api = resourceHearingsApi(
      async () => {
        calls++;
        throw failure;
      },
      overview.case_id,
      overview.resource_id,
    );
    await assert.rejects(api.exact(overview), (error) => error === failure);
    assert.equal(calls, 1);
  }
});
