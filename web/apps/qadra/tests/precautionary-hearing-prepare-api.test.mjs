import test from 'node:test';
import assert from 'node:assert/strict';
import { precautionaryHearingsApi } from '../src/lib/precautionary-hearing-api.mjs';
import {
  precautionaryHearingOperation,
  preparedHearing,
  hearingPrincipal,
  hearingBase,
  workflowClient,
  foreign,
  clone,
} from './fixtures/precautionary-hearing-workflow.mjs';

const client = (reply) => workflowClient(precautionaryHearingsApi, reply);

test('prepare returns the complete reviewed schedule replace or cancellation without capturing it', async () => {
  for (const revision of [1, 2, 3]) {
    const draft = preparedHearing(revision),
      { api, calls } = client(draft);
    assert.deepEqual(await api.prepare(clone(draft.command), hearingPrincipal()), draft);
    assert.deepEqual(calls, [
      { path: `${hearingBase}/prepare`, options: { method: 'POST', data: draft.command } },
    ]);
    assert.equal(Object.hasOwn(draft, 'capture_digest'), false);
    assert.equal(Object.hasOwn(draft, 'recorded_at'), false);
    if (revision === 3) {
      assert.equal(Object.hasOwn(draft.command.change, 'values'), false);
      assert.equal(Object.hasOwn(draft.command.change, 'context'), false);
      assert.equal(draft.resolved_values.venue, 'Sala cautelar reprogramada');
    }
  }
});

test('prepare normalizes declared text and exact selection order without mutating the caller', async () => {
  const draft = preparedHearing();
  draft.resolved_values.note = 'Primera linea\nSegunda linea';
  draft.command.change.values = clone(draft.resolved_values);
  const command = clone(draft.command);
  command.change.values.note = ' Primera linea\r\nSegunda linea ';
  command.change.values.venue = ` ${draft.resolved_values.venue} `;
  command.change.values.participants.reverse();
  const original = clone(command),
    { api, calls } = client(draft);
  assert.deepEqual(await api.prepare(command, hearingPrincipal()), draft);
  assert.deepEqual(command, original);
  assert.deepEqual(calls[0].options.data, draft.command);
});

test('prepare rejects invalid command shape scope revision and selections before transport', async () => {
  for (const change of [
    (v) => {
      v.case_id = foreign;
    },
    (v) => {
      v.operation_id = v.operation_id.toUpperCase();
    },
    (v) => {
      v.hearing_id = 'invalid';
    },
    (v) => {
      v.actor = hearingPrincipal();
    },
    (v) => {
      v.change.expected_revision = 0;
    },
    (v) => {
      v.change.context.context_digest = 'bad';
    },
    (v) => {
      v.change.context.stage_revision = 0;
    },
    (v) => {
      v.change.values.purpose = 'initial';
    },
    (v) => {
      delete v.change.values.note;
    },
    (v) => {
      v.change.values.participants.push(clone(v.change.values.participants[0]));
    },
    (v) => {
      v.change.values.scheduled_at = '2026-01-01T09:00:00';
    },
    (v) => {
      v.change.values.scheduling_basis.support.digest = 'bad';
    },
    (v) => {
      v.change.values.note = 'x'.repeat(1001);
    },
    (v) => {
      v.change.values.venue = 'x'.repeat(501);
    },
  ]) {
    const draft = preparedHearing(),
      command = clone(draft.command),
      { api, calls } = client(draft);
    change(command);
    await assert.rejects(api.prepare(command, hearingPrincipal()), change.toString());
    assert.equal(calls.length, 0);
  }
  for (const revision of [2, 3]) {
    for (const change of [
      (v) => {
        v.expected_revision = 0;
      },
      (v) => {
        v.expected_revision = 4294967295;
      },
      (v) => {
        v.expected_capture_digest = 'bad';
      },
      (v) => {
        delete v.reason;
      },
    ]) {
      const draft = preparedHearing(revision),
        command = clone(draft.command),
        { api, calls } = client(draft);
      change(command.change);
      await assert.rejects(api.prepare(command, hearingPrincipal()));
      assert.equal(calls.length, 0);
    }
  }
  for (const field of ['context', 'values']) {
    const draft = preparedHearing(3),
      command = clone(draft.command),
      { api, calls } = client(draft);
    command.change[field] = clone(preparedHearing().command.change[field]);
    await assert.rejects(api.prepare(command, hearingPrincipal()));
    assert.equal(calls.length, 0);
  }
});

test('prepare admits both managing roles and binds the full returned actor', async () => {
  for (const role of ['owner', 'litigator']) {
    const draft = preparedHearing();
    draft.actor.role = role;
    assert.deepEqual(await client(draft).api.prepare(draft.command, clone(draft.actor)), draft);
  }
  for (const role of ['paralegal', 'client']) {
    const draft = preparedHearing(),
      { api, calls } = client(draft);
    await assert.rejects(api.prepare(draft.command, { ...hearingPrincipal(), role }));
    assert.equal(calls.length, 0);
  }
  for (const [field, changed] of Object.entries({
    id: foreign,
    email: 'other@example.test',
    role: 'litigator',
  })) {
    const draft = preparedHearing();
    draft.actor[field] = changed;
    await assert.rejects(client(draft).api.prepare(preparedHearing().command, hearingPrincipal()));
  }
});

test('prepare respects zero and thirty-two participants and finite review target bounds', async () => {
  for (const participantCount of [0, 32]) {
    const draft = precautionaryHearingOperation({ revision: 1, participantCount }).capture.review;
    assert.deepEqual(await client(draft).api.prepare(draft.command, hearingPrincipal()), draft);
  }
  for (const count of [1, 32]) {
    const draft = preparedHearing();
    draft.resolved_values.purpose = 'review';
    draft.resolved_values.review_targets = Array.from({ length: count }, (_, index) => ({
      id: `a0000000-0000-4000-8000-${String(index + 1).padStart(12, '0')}`,
      revision: 2,
      capture_digest: 'a'.repeat(64),
    }));
    draft.command.change.values = clone(draft.resolved_values);
    assert.deepEqual(await client(draft).api.prepare(draft.command, hearingPrincipal()), draft);
  }
  for (const change of [
    (v) => {
      v.participants = Array.from({ length: 33 }, () => clone(v.participants[0]));
    },
    (v) => {
      v.purpose = 'review';
    },
    (v) => {
      v.review_targets = [{ id: foreign, revision: 1, capture_digest: 'a'.repeat(64) }];
    },
  ]) {
    const draft = preparedHearing(),
      command = clone(draft.command),
      { api, calls } = client(draft);
    change(command.change.values);
    await assert.rejects(api.prepare(command, hearingPrincipal()));
    assert.equal(calls.length, 0);
  }
});

test('prepare rejects a different command result status or material returned by the server', async () => {
  for (const change of [
    (v) => {
      v.command.operation_id = foreign;
    },
    (v) => {
      v.command.hearing_id = foreign;
    },
    (v) => {
      v.result_revision = 2;
    },
    (v) => {
      v.status = 'cancelled';
    },
    (v) => {
      v.resolved_values.venue = 'Another venue';
    },
    (v) => {
      v.sources.support.digest = 'b'.repeat(64);
    },
    (v) => {
      v.sources.participants[0].case_id = foreign;
    },
    (v) => {
      v.observed_context.administration.administrative_status = 'closed';
    },
    (v) => {
      v.submission_digest = 'bad';
    },
    (v) => {
      v.review_digest = 'bad';
    },
  ]) {
    const draft = preparedHearing();
    change(draft);
    await assert.rejects(client(draft).api.prepare(preparedHearing().command, hearingPrincipal()));
  }
});

test('pending prepare retains the original command and principal and rejects disposed responses', async () => {
  for (const disposed of [false, true]) {
    const draft = preparedHearing(),
      command = clone(draft.command),
      actor = hearingPrincipal();
    let release;
    const { api, calls } = client(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    );
    const pending = api.prepare(command, actor);
    command.change.values.venue = 'Changed after request';
    actor.email = 'changed@example.test';
    if (disposed) api.dispose();
    release(clone(draft));
    if (disposed) await assert.rejects(pending);
    else assert.deepEqual(await pending, draft);
    assert.deepEqual(calls[0].options.data, draft.command);
  }
});
