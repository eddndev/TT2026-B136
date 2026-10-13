import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';

const subject = { case_id: 'case', id: 'subject', revision: 2, values_digest: 'a'.repeat(64) };
const detail = {
  case_id: 'case',
  id: 'participant',
  revision: 3,
  canonical_format: 'part2',
  subject,
};
const proposal = { participant_id: detail.id, expected_participant_revision: 2 };
const review = { directory_stamp: 'b'.repeat(64), selection_reason: 'Reviewed', different: [] };
const preparation = { proposal, review, certificate_base64: null };

test('typed facade sends review/prepare/commit with explicit bodies and validates current case', async () => {
  const calls = [];
  const api = createApi(async (path, options) => {
    calls.push({ path, method: options.method, data: JSON.parse(options.body) });
    return new Response(
      JSON.stringify(
        path.endsWith('/commit') ? detail : { case_id: 'case', proposal, review, candidates: [] },
      ),
    );
  }).caseTypedParticipants('case');
  const input = {
    subject: { operation: 'keep', reference: subject },
    participant: { operation: 'existing', id: detail.id, expected_revision: 2 },
    role: {},
    certificate_base64: null,
  };
  await api.review(input);
  await api.prepare(preparation);
  await api.commit({ prepared: preparation, signature_base64: null });
  assert.deepEqual(
    calls.map(({ path, method }) => [path, method]),
    ['review', 'prepare', 'commit'].map((name) => [
      `/api/v1/cases/case/participants/proposals/${name}`,
      'POST',
    ]),
  );
  assert.deepEqual(
    calls.map((call) => call.data),
    [input, preparation, { prepared: preparation, signature_base64: null }],
  );
});

test('subject and participant historical reads use exact revisions, not latest substitutions', async () => {
  const paths = [];
  const api = createApi(async (path) => {
    paths.push(path);
    const payload = path.includes('/subjects')
      ? subject
      : path.endsWith('/credential')
        ? { case_id: 'case', reference: { participant_id: 'participant', participant_revision: 3 } }
        : detail;
    return new Response(JSON.stringify(payload));
  }).caseTypedParticipants('case');
  await api.subject('subject');
  await api.subjectRevision('subject', 2);
  await api.participantRevision('participant', 3);
  await api.credential('participant', 3);
  assert.deepEqual(paths, [
    '/api/v1/cases/case/subjects/subject',
    '/api/v1/cases/case/subjects/subject/revisions/2',
    '/api/v1/cases/case/participants/participant/revisions/3',
    '/api/v1/cases/case/participants/participant/revisions/3/credential',
  ]);
});

test('subject index and history use server cursors and reject foreign rows', async () => {
  const paths = [];
  let foreign = false;
  const api = createApi(async (path) => {
    paths.push(path);
    const row = { ...subject, case_id: foreign ? 'other' : 'case' };
    return new Response(
      JSON.stringify(
        path.includes('/history')
          ? { revisions: [row], has_more: false }
          : { subjects: [row], has_more: false },
      ),
    );
  }).caseTypedParticipants('case');
  await api.subjects({ limit: 20, afterId: 'cursor', name: 'Ana', kind: 'natural_person' });
  await api.subjectHistory('subject', { beforeRevision: 3 });
  assert.equal(
    paths[0],
    '/api/v1/cases/case/subjects?limit=20&after_id=cursor&name=Ana&kind=natural_person',
  );
  assert.match(paths[1], /before_revision=3/);
  foreign = true;
  await assert.rejects(api.subjects(), /expediente/);
});

test('subject edits separate candidate review from explicit replacement', async () => {
  const calls = [];
  const api = createApi(async (path, options) => {
    calls.push([path, options.method, JSON.parse(options.body)]);
    return new Response(
      JSON.stringify(
        path.endsWith('/review')
          ? { case_id: 'case', id: 'subject', expected_revision: 2, candidates: [] }
          : subject,
      ),
    );
  }).caseTypedParticipants('case');
  await api.reviewSubject('subject', 2, { kind: 'natural_person' });
  await api.replaceSubject('subject', 2, { kind: 'natural_person' }, review);
  assert.equal(calls[0][1], 'POST');
  assert.equal(calls[1][1], 'PUT');
  assert.deepEqual(calls[1][2], {
    expected_revision: 2,
    values: { kind: 'natural_person' },
    review,
  });
});

test('wrong exact detail and stale disposed responses cannot enter a selection', async () => {
  let release;
  let delayed = false;
  const api = createApi(async () => {
    if (delayed) await new Promise((resolve) => (release = resolve));
    return new Response(JSON.stringify({ ...detail, revision: 4 }));
  }).caseTypedParticipants('case');
  await assert.rejects(api.participantRevision('participant', 3), /revisi/);
  delayed = true;
  const pending = api.participantRevision('participant', 4);
  api.dispose();
  release();
  await assert.rejects(pending, /expediente/);
});

test('passes typed kind and profile filters through the compact participant index', async () => {
  const { participantsApi } = await import('../src/lib/participants-api.mjs');
  let path;
  const api = participantsApi(async (value) => {
    path = value;
    return { participants: [], has_more: false, next_after_id: null };
  }, 'case');
  await api.list({ kind: 'control_judge', profile: 'typed', status: 'all' });
  assert.match(path, /kind=control_judge/);
  assert.match(path, /profile=typed/);
});
