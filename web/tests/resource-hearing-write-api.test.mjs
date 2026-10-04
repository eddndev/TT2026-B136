import test from 'node:test';
import assert from 'node:assert/strict';
import { resourceHearingsApi } from '../src/lib/resource-hearing-api.mjs';
import {
  clone,
  resourceHearingCreation,
  resourceHearingCommand,
  resourceHearingPrepared,
  resourceHearingResult,
} from './fixtures/resource-hearing-unit.mjs';

const foreign = 'f0000000-0000-4000-8000-000000000099';
const creation = () => resourceHearingCreation({ withAct: true });
const prepared = () => resourceHearingPrepared(creation());
const principal = () => ({ ...prepared().recorded_by, role: 'owner' });
function setup(reply, draft = prepared()) {
  const calls = [];
  const api = resourceHearingsApi(
    async (path, options) => {
      calls.push({ path, options: clone(options) });
      return typeof reply === 'function' ? reply(path, options) : clone(reply);
    },
    draft.command.case_id,
    draft.command.resource_id,
  );
  return { api, calls };
}
const base = (d) =>
  `/cases/${d.command.case_id}/procedural-resources/${d.command.resource_id}/activities/resource-hearings`;

test('prepare keeps historical resource and act distinct from observed head', async () => {
  const draft = prepared(),
    { api, calls } = setup(draft);
  assert.deepEqual(await api.prepare(resourceHearingCommand(creation()), principal()), draft);
  assert.deepEqual(calls, [
    { path: `${base(draft)}/prepare`, options: { method: 'POST', data: draft.command } },
  ]);
  assert.equal(draft.command.resource.revision, 1);
  assert.equal(draft.command.act.resource_revision, 4);
  assert.equal(draft.observed_resource_head.revision, 5);
  assert.equal(Object.hasOwn(draft, 'capture_digest'), false);
});

test('prepare rejects malformed scoped commands before transport', async () => {
  for (const alter of [
    (c) => {
      c.case_id = foreign;
    },
    (c) => {
      c.resource_id = foreign;
    },
    (c) => {
      c.resource.id = foreign;
    },
    (c) => {
      delete c.act;
    },
    (c) => {
      c.operation_id = c.operation_id.toUpperCase();
    },
    (c) => {
      c.expected_resource_revision = 0;
    },
    (c) => {
      c.expected_resource_revision = 3;
    },
    (c) => {
      c.values.participants.push(clone(c.values.participants[0]));
    },
    (c) => {
      c.values.scheduling_basis.support.digest = 'bad';
    },
    (c) => {
      c.values.scheduled_at = '2026-01-01T10:00:00';
    },
    (c) => {
      c.values.stage = 'trial';
    },
    (c) => {
      c.extra = true;
    },
  ]) {
    const { api, calls } = setup(prepared()),
      command = resourceHearingCommand(creation());
    alter(command);
    await assert.rejects(api.prepare(command, principal()));
    assert.equal(calls.length, 0);
  }
  for (const role of ['paralegal', 'client']) {
    const { api, calls } = setup(prepared());
    await assert.rejects(api.prepare(prepared().command, { ...principal(), role }));
    assert.equal(calls.length, 0);
  }
});

test('prepare binds full actor, exact material, administration and observed head', async () => {
  for (const alter of [
    (d) => {
      d.command.hearing_id = foreign;
    },
    (d) => {
      d.recorded_by.id = foreign;
    },
    (d) => {
      d.recorded_by.email = 'different@example.test';
    },
    (d) => {
      d.resource.receipt.capture_digest = '0'.repeat(64);
    },
    (d) => {
      d.act.act.id = foreign;
    },
    (d) => {
      d.act = null;
    },
    (d) => {
      d.observed_resource_head.revision = 4;
    },
    (d) => {
      d.observed_resource_head.id = foreign;
    },
    (d) => {
      d.observed_administration.case_id = foreign;
    },
    (d) => {
      d.sources.support.name = 'unadmitted.pdf';
    },
    (d) => {
      d.sources.participants[0].case_id = foreign;
    },
    (d) => {
      d.sources.participants[0].revision += 1;
    },
    (d) => {
      d.sources.participants[0].directory_status = 'inactive';
    },
    (d) => {
      d.resource.values.kind = 'revocation';
    },
    (d) => {
      d.submission_digest = 'bad';
    },
    (d) => {
      d.recorded_at = '2026-01-01T00:00:00Z';
    },
  ]) {
    const draft = prepared();
    alter(draft);
    const { api, calls } = setup(draft);
    await assert.rejects(api.prepare(prepared().command, principal()));
    assert.equal(calls.length, 1);
  }
});

test('submit posts only retained command and submission digest once', async () => {
  const draft = prepared(),
    result = resourceHearingResult(draft);
  const { api, calls } = setup(result);
  assert.deepEqual(await api.submit(draft), result);
  assert.deepEqual(calls, [
    {
      path: `${base(draft)}/submit`,
      options: {
        method: 'POST',
        data: { command: draft.command, expected_submission_digest: draft.submission_digest },
      },
    },
  ]);
});

test('submit and reconciliation reject internally valid but different retained material', async () => {
  for (const alter of [
    (d) => {
      d.command.operation_id = foreign;
    },
    (d) => {
      d.command.hearing_id = foreign;
    },
    (d) => {
      d.command.association_id = foreign;
    },
    (d) => {
      d.command.values.venue = 'Otra sala';
    },
    (d) => {
      d.submission_digest = '0'.repeat(64);
    },
    (d) => {
      d.recorded_by.email = 'other@example.test';
    },
    (d) => {
      d.sources.participants[0].display_name = 'Otro nombre';
    },
    (d) => {
      d.observed_resource_head.capture_digest = '0'.repeat(64);
    },
  ]) {
    const different = prepared();
    alter(different);
    for (const method of ['submit', 'readSubmission']) {
      const { api, calls } = setup(resourceHearingResult(different));
      await assert.rejects(api[method](prepared()));
      assert.equal(calls.length, 1);
    }
  }
});

test('uncertain submit can be read exactly without retrying or inventing final capture', async () => {
  const draft = prepared(),
    result = resourceHearingResult(draft);
  const failure = Object.assign(new Error('Connection interrupted'), { status: 503 });
  const { api, calls } = setup((path) => {
    if (path.endsWith('/submit')) throw failure;
    return clone(result);
  });
  await assert.rejects(api.submit(draft), (error) => error === failure);
  assert.equal(calls.length, 1);
  assert.deepEqual(await api.readSubmission(draft), { state: 'confirmed', creation: result });
  assert.equal(calls.length, 2);
  assert.equal(calls[1].path, `${base(draft)}/${draft.command.hearing_id}/revisions/1`);
  assert.equal(calls[1].options?.method || 'GET', 'GET');
  assert.equal(calls[1].options?.data, undefined);
});

test('only exact missing activity remains unconfirmed and explicit retry preserves bytes', async () => {
  const draft = prepared(),
    result = resourceHearingResult(draft);
  const missing = Object.assign(new Error('Missing'), {
    status: 404,
    code: 'resource_activity_not_found',
  });
  const { api, calls } = setup((path) => {
    if (path.includes('/revisions/')) throw missing;
    return clone(result);
  });
  assert.deepEqual(await api.readSubmission(draft), { state: 'unconfirmed' });
  assert.equal(calls.length, 1);
  assert.deepEqual(await api.submit(draft), result);
  assert.deepEqual(calls[1].options.data, {
    command: draft.command,
    expected_submission_digest: draft.submission_digest,
  });
});

test('authorization, foreign 404 and integrity failures are never absence or retried', async () => {
  for (const [status, code] of [
    [401, 'unauthenticated'],
    [403, 'forbidden'],
    [404, 'case_not_found'],
    [409, 'resource_conflict'],
    [500, 'internal_error'],
  ]) {
    for (const method of ['submit', 'readSubmission']) {
      const failure = Object.assign(new Error(code), { status, code });
      const { api, calls } = setup(() => {
        throw failure;
      });
      await assert.rejects(api[method](prepared()), (error) => error === failure);
      assert.equal(calls.length, 1);
    }
  }
});

test('pending prepare snapshots command and full principal against caller mutation', async () => {
  const draft = prepared(),
    command = clone(draft.command),
    actor = principal();
  let resolve;
  const { api } = setup(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = api.prepare(command, actor);
  command.values.venue = 'Changed later';
  actor.email = 'changed@example.test';
  resolve(clone(draft));
  assert.deepEqual(await pending, draft);
});

test('pending submit snapshots retained draft and refuses late results after dispose', async () => {
  for (const disposed of [false, true]) {
    const draft = prepared(),
      result = resourceHearingResult(draft);
    let resolve;
    const { api } = setup(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const pending = api.submit(draft);
    draft.command.values.venue = 'Changed later';
    draft.submission_digest = '0'.repeat(64);
    if (disposed) api.dispose();
    resolve(clone(result));
    if (disposed) await assert.rejects(pending);
    else assert.deepEqual(await pending, result);
  }
});

test('disposed recovery cannot turn a late missing result into unconfirmed state', async () => {
  let reject;
  const { api, calls } = setup(
    () =>
      new Promise((_, fail) => {
        reject = fail;
      }),
  );
  const pending = api.readSubmission(prepared());
  api.dispose();
  reject(Object.assign(new Error('Missing'), { status: 404, code: 'resource_activity_not_found' }));
  await assert.rejects(pending);
  await assert.rejects(api.prepare(prepared().command, principal()));
  await assert.rejects(api.submit(prepared()));
  assert.equal(calls.length, 1);
});
