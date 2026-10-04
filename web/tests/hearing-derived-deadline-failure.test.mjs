import test from 'node:test';
import assert from 'node:assert/strict';
import { createHearingDerivedDeadlineFailure } from '../src/lib/hearing-derived-deadline-failure.mjs';
import { hearingResultDraft } from '../src/lib/hearing-result-values.mjs';
import { derivedReady, clone, ids, digest } from './fixtures/hearing-derived-deadline-unit.mjs';

function setup(mode = 'draft') {
  const ready = derivedReady();
  const support = { document_id: ids(88), version: 1, digest: digest('8') };
  ready.result.values.provenance.support = support;
  ready.result.command.change.values = clone(ready.result.values);
  ready.command.result.change.values = clone(ready.result.values);
  ready.result.support = { ...support, name: 'Evidence.pdf', format: 'pdf', policy: 'pdf_docx_v1' };
  const state = {
    draft: hearingResultDraft(ready.result),
    mode,
    last: ['uncertain', 'conflict'].includes(mode) ? clone(ready) : null,
    blocked: true,
    prepared: clone(ready),
    acknowledged: true,
    retryAvailable: false,
    error: '',
  };
  const discarded = [],
    denied = [];
  const failure = createHearingDerivedDeadlineFailure({
    read: () => state,
    update: (patch) => Object.assign(state, patch),
    discardSupport: () => discarded.push('support'),
    deny: (error) => denied.push(error),
  });
  return { state, failure, discarded, denied };
}
const unavailable = (status) =>
  Object.assign(new Error('Selected support is unavailable'), {
    status,
    code: 'document_not_found',
    draftReference: 'support',
  });

test('unavailable support is repaired while initialization remains blocked without losing declarations', () => {
  for (const status of [403, 404]) {
    const s = setup(),
      expected = clone(s.state.draft);
    expected.provenance.support = null;
    s.failure(unavailable(status));
    assert.deepEqual(s.state.draft, expected);
    assert.equal(s.state.mode, 'draft');
    assert.equal(s.state.prepared, null);
    assert.equal(s.state.acknowledged, false);
    assert.equal(s.state.last, null);
    assert.deepEqual(s.discarded, ['support']);
    assert.deepEqual(s.denied, []);
    assert.match(s.state.error, /support/i);
  }
});

test('case or owning-context denial is never treated as a removable support reference', () => {
  for (const error of [
    Object.assign(unavailable(404), { code: 'case_not_found' }),
    Object.assign(unavailable(403), { draftReference: 'owner' }),
    Object.assign(new Error('Denied'), { status: 403, code: 'permission_denied' }),
  ]) {
    const s = setup(),
      before = clone(s.state.draft);
    s.failure(error);
    assert.deepEqual(s.denied, [error]);
    assert.deepEqual(s.discarded, []);
    assert.deepEqual(s.state.draft, before);
  }
});

test('a retained uncertain or conflicting attempt keeps all original fields and evidence', () => {
  for (const mode of ['uncertain', 'conflict']) {
    for (const status of [403, 404]) {
      const s = setup(mode),
        before = clone(s.state);
      s.failure(unavailable(status), true);
      assert.deepEqual(s.state.draft, before.draft);
      assert.deepEqual(s.state.last, before.last);
      assert.equal(s.state.mode, mode);
      assert.deepEqual(s.discarded, []);
      assert.deepEqual(s.denied, []);
      assert.equal(s.state.retryAvailable, false);
    }
  }
});

test('compound request budget failures report one MiB and preserve raw inputs', () => {
  const s = setup(),
    before = clone(s.state.draft);
  s.failure(Object.assign(new Error('Payload rejected'), { status: 413 }));
  assert.match(s.state.error, /1 MiB/);
  assert.doesNotMatch(s.state.error, /512/);
  assert.deepEqual(s.state.draft, before);
  assert.deepEqual(s.discarded, []);
  assert.deepEqual(s.denied, []);
});

test('transport failure during submission keeps the original attempt available for explicit checking', () => {
  const s = setup('uncertain'),
    before = clone(s.state);
  s.failure(new Error('Lost response'), true);
  assert.equal(s.state.mode, 'uncertain');
  assert.deepEqual(s.state.last, before.last);
  assert.deepEqual(s.state.draft, before.draft);
  assert.equal(s.state.error, 'Lost response');
  assert.deepEqual(s.discarded, []);
  assert.deepEqual(s.denied, []);
});
