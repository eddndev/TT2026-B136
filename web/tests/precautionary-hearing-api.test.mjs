import test from 'node:test';
import assert from 'node:assert/strict';
import { precautionaryHearingsApi } from '../src/lib/precautionary-hearing-api.mjs';
import { caseApi } from '../src/lib/case-api.mjs';
import {
  precautionaryCaseId,
  precautionaryHearingOperation,
  precautionaryHearingOverview,
  precautionaryHearingAlert,
  clone,
} from './fixtures/precautionary-hearing-unit.mjs';

const foreign = 'f0000000-0000-4000-8000-000000000099';
function client(value = precautionaryHearingOperation()) {
  const calls = [];
  const api = precautionaryHearingsApi(async (path, options) => {
    calls.push({ path, options });
    return clone(value);
  }, precautionaryCaseId);
  return { api, calls };
}
function exactPath(selected) {
  return `/cases/${selected.case_id}/precautionary-hearings/${selected.id}/revisions/${selected.revision}`;
}

test('exact precautionary reads preserve cancellation values and the selected historical prefix', async () => {
  for (const revision of [1, 2, 3]) {
    const value = precautionaryHearingOperation({ revision });
    const selected = precautionaryHearingOverview(value),
      { api, calls } = client(value);
    const result = await api.exact(selected);
    assert.deepEqual(result, value);
    assert.deepEqual(calls, [{ path: exactPath(selected), options: undefined }]);
    assert.equal(result.history.captures.length, revision);
    assert.deepEqual(result.history.captures.at(-1), result.capture);
    assert.equal(result.history.origin.capture_digest, result.history.captures[0].capture_digest);
    assert.equal(result.capture.review.sources.participants[0].display_name, 'Persona historica 1');
    if (revision === 3) {
      assert.equal(result.capture.review.status, 'cancelled');
      assert.equal(Object.hasOwn(result.capture.review.command.change, 'values'), false);
      assert.deepEqual(
        result.capture.review.resolved_values,
        result.history.captures[1].review.resolved_values,
      );
      assert.equal(result.capture.review.resolved_values.venue, 'Sala cautelar reprogramada');
    }
  }
});

test('case API exposes the same scoped exact precautionary reader', async () => {
  const value = precautionaryHearingOperation(),
    calls = [];
  const api = caseApi(async (path, options) => {
    calls.push({ path, options });
    return clone(value);
  });
  const selected = precautionaryHearingOverview(value);
  const scoped = api.casePrecautionaryHearings(selected.case_id);
  assert.deepEqual(await scoped.exact(selected), value);
  assert.equal(typeof scoped.fromAlert, 'function');
  assert.deepEqual(calls, [{ path: exactPath(selected), options: undefined }]);
});

test('invalid or foreign precautionary agenda selections fail before requesting', async () => {
  for (const change of [
    (v) => {
      v.case_id = foreign;
    },
    (v) => {
      v.id = 'not-an-id';
    },
    (v) => {
      v.revision = 0;
    },
    (v) => {
      v.capture_digest = 'bad';
    },
    (v) => {
      v.purpose = 'initial';
    },
    (v) => {
      v.status = 'withdrawn';
    },
    (v) => {
      v.modality = 'other';
    },
    (v) => {
      v.participant_count = 33;
    },
    (v) => {
      v.scheduled_at = '2026-02-30T10:00:00Z';
    },
    (v) => {
      v.resource_id = foreign;
    },
  ]) {
    const selected = precautionaryHearingOverview(),
      { api, calls } = client();
    change(selected);
    await assert.rejects(api.exact(selected), change.toString());
    assert.equal(calls.length, 0);
  }
});

test('selected precautionary detail must match every agenda overview field', async () => {
  const value = precautionaryHearingOperation();
  for (const [field, changed] of Object.entries({
    id: foreign,
    revision: 2,
    capture_digest: '1'.repeat(64),
    purpose: 'review',
    scheduled_at: '2026-01-04T09:00:01-06:00',
    modality: 'videoconference',
    status: 'scheduled',
    participant_count: 1,
  })) {
    const selected = precautionaryHearingOverview(value),
      { api, calls } = client(value);
    selected[field] = changed;
    await assert.rejects(api.exact(selected), field);
    assert.equal(calls.length, 1);
  }
});

test('exact reads reject a substituted case, origin, selected capture or broken history prefix', async () => {
  for (const change of [
    (v) => {
      v.capture.review.case_id = foreign;
    },
    (v) => {
      v.capture.review.command.case_id = foreign;
    },
    (v) => {
      v.capture.review.command.hearing_id = foreign;
    },
    (v) => {
      v.history.origin.case_id = foreign;
    },
    (v) => {
      v.history.origin.hearing_id = foreign;
    },
    (v) => {
      v.history.origin.operation_id = foreign;
    },
    (v) => {
      v.history.origin.capture_digest = 'a'.repeat(64);
    },
    (v) => {
      v.history.captures.shift();
    },
    (v) => {
      v.history.captures.splice(1, 1);
    },
    (v) => {
      v.history.captures.reverse();
    },
    (v) => {
      v.history.captures.push(clone(v.capture));
    },
    (v) => {
      v.history.captures[1].review.result_revision = 1;
    },
    (v) => {
      v.history.captures[1].review.command.hearing_id = foreign;
    },
    (v) => {
      v.history.captures[1].review.command.change.expected_revision = 0;
    },
    (v) => {
      v.history.captures[1].review.command.change.expected_capture_digest = 'a'.repeat(64);
    },
    (v) => {
      v.history.captures[1].review.command.operation_id = v.history.origin.operation_id;
    },
    (v) => {
      v.history.captures[2].review.resolved_values.venue = 'Other retained venue';
    },
    (v) => {
      v.capture.review.resolved_values.venue = 'Substituted venue';
    },
  ]) {
    const value = precautionaryHearingOperation(),
      selected = precautionaryHearingOverview(value);
    change(value);
    await assert.rejects(client(value).api.exact(selected), change.toString());
  }
});

test('precautionary reads preserve both empty and maximum participant captures', async () => {
  for (const participantCount of [0, 32]) {
    const value = precautionaryHearingOperation({ revision: 1, participantCount });
    assert.deepEqual(await client(value).api.exact(precautionaryHearingOverview(value)), value);
  }
});

test('precautionary alerts open their historical revision and original capture through GET', async () => {
  const value = precautionaryHearingOperation({ revision: 2 });
  const row = precautionaryHearingAlert(value),
    { api, calls } = client(value);
  row.state = { kind: 'resolved', at: clone(row.created_at), reason: 'cancelled_hearing' };
  assert.deepEqual(await api.fromAlert(row), value);
  assert.deepEqual(calls, [
    { path: exactPath(precautionaryHearingOverview(value)), options: undefined },
  ]);
  assert.equal(value.capture.review.status, 'scheduled');
  assert.equal(value.capture.capture_digest, row.origin.evidence_digest);
  assert.notEqual(value.capture.review.submission_digest, row.origin.evidence_digest);
});

test('foreign or malformed precautionary alert selectors fail before requesting', async () => {
  for (const change of [
    (v) => {
      v.subject.kind = 'hearing';
    },
    (v) => {
      v.subject.kind = 'resource_hearing';
    },
    (v) => {
      v.subject.case_id = foreign;
    },
    (v) => {
      v.subject.resource_id = foreign;
    },
    (v) => {
      v.subject.id = 'invalid';
    },
    (v) => {
      v.origin.revision = 0;
    },
    (v) => {
      v.origin.evidence_digest = 'bad';
    },
    (v) => {
      v.kind = { kind: 'review_required' };
    },
    (v) => {
      v.kind.lead_hours = 0;
    },
  ]) {
    const value = precautionaryHearingOperation({ revision: 2 });
    const row = precautionaryHearingAlert(value),
      { api, calls } = client(value);
    change(row);
    await assert.rejects(api.fromAlert(row), change.toString());
    assert.equal(calls.length, 0);
  }
});

test('precautionary alert detail rejects mismatched revision, evidence or activity time without fallback', async () => {
  for (const change of [
    (v) => {
      v.subject.id = foreign;
    },
    (v) => {
      v.origin.revision = 1;
    },
    (v) => {
      v.origin.evidence_digest = '1'.repeat(64);
    },
    (v) => {
      v.kind.activity_at.unix_seconds++;
    },
    (v) => {
      v.kind.activity_at.nanosecond++;
    },
  ]) {
    const value = precautionaryHearingOperation({ revision: 2 });
    const row = precautionaryHearingAlert(value),
      { api, calls } = client(value);
    change(row);
    await assert.rejects(api.fromAlert(row), change.toString());
    assert.equal(calls.length, 1);
  }
});

test('disposing either precautionary reader rejects delayed responses and subsequent requests', async () => {
  for (const method of ['exact', 'fromAlert']) {
    const value = precautionaryHearingOperation({ revision: 2 });
    const selected =
      method === 'exact' ? precautionaryHearingOverview(value) : precautionaryHearingAlert(value);
    let release,
      calls = 0;
    const api = precautionaryHearingsApi(() => {
      calls++;
      return new Promise((resolve) => {
        release = resolve;
      });
    }, precautionaryCaseId);
    const pending = api[method](selected);
    api.dispose();
    release(value);
    await assert.rejects(pending);
    await assert.rejects(api[method](selected));
    assert.equal(calls, 1);
  }
});

test('pending precautionary reads retain the original selection across caller changes', async () => {
  for (const method of ['exact', 'fromAlert']) {
    const value = precautionaryHearingOperation({ revision: 2 });
    const selected =
      method === 'exact' ? precautionaryHearingOverview(value) : precautionaryHearingAlert(value);
    let release;
    const api = precautionaryHearingsApi(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
      precautionaryCaseId,
    );
    const pending = api[method](selected);
    if (method === 'exact') selected.capture_digest = '1'.repeat(64);
    else selected.origin.evidence_digest = '1'.repeat(64);
    release(clone(value));
    assert.deepEqual(await pending, value);
  }
});

test('precautionary reads preserve transport failures without retries or alternate routes', async () => {
  for (const method of ['exact', 'fromAlert']) {
    for (const status of [401, 403, 404, 409, 500, 503]) {
      const value = precautionaryHearingOperation({ revision: 2 });
      const selected =
        method === 'exact' ? precautionaryHearingOverview(value) : precautionaryHearingAlert(value);
      const failure = Object.assign(new Error('Read failed'), { status, code: 'read_failed' });
      let calls = 0;
      const api = precautionaryHearingsApi(async () => {
        calls++;
        throw failure;
      }, precautionaryCaseId);
      await assert.rejects(api[method](selected), (error) => error === failure);
      assert.equal(calls, 1);
    }
  }
});
