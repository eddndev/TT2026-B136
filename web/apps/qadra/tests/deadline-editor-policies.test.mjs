import test from 'node:test';
import assert from 'node:assert/strict';
import {
  initialDeadlinePolicies,
  reconcileDeadlinePolicies,
  deadlinePoliciesCommand,
} from '../src/lib/deadline-editor-policies.mjs';
import { definition, id } from './fixtures/deadline-unit.mjs';
import { v2Prepared } from './fixtures/deadline-v2-unit.mjs';

test('new and legacy edits do not choose an intention for a selected profile', () => {
  const value = definition();
  for (const policies of [
    undefined,
    {
      profile: 'undetermined',
      source: 'undetermined',
      calendar: 'undetermined',
    },
  ]) {
    const state = initialDeadlinePolicies(value, policies);
    assert.equal(state.profile.value, '');
    assert.equal(state.source.value, '');
    assert.equal(state.calendar.value, '');
    assert.throws(() => deadlinePoliciesCommand(state, value), /perfil/i);
  }
});

test('a saved explicit policy is retained only for an existing dependency', () => {
  const value = v2Prepared();
  const state = initialDeadlinePolicies(value.definition, value.tracking.policies);
  assert.equal(state.profile.value, 'follow');
  assert.deepEqual(deadlinePoliciesCommand(state, value.definition), {
    profile: 'follow',
    source: 'undetermined',
    calendar: 'undetermined',
  });
});

test('another revision of the same root keeps the selected intention', () => {
  const value = definition();
  const state = initialDeadlinePolicies(value);
  state.profile.value = 'fixed';
  value.profile.revision = 2;
  const next = reconcileDeadlinePolicies(state, value);
  assert.equal(next.profile.value, 'fixed');
  assert.equal(deadlinePoliciesCommand(next, value).profile, 'fixed');
});

test('choosing a different root clears only the affected policy', () => {
  const value = definition();
  value.input.calendar = { id: id(30), revision: 1 };
  const state = initialDeadlinePolicies(value, {
    profile: 'follow',
    source: 'undetermined',
    calendar: 'fixed',
  });
  value.profile.id = id(31);
  const next = reconcileDeadlinePolicies(state, value);
  assert.equal(next.profile.value, '');
  assert.equal(next.calendar.value, 'fixed');
  assert.throws(() => deadlinePoliciesCommand(state, value), /perfil/i);
});

test('removing and reselecting a dependency requires a fresh explicit choice', () => {
  const value = definition();
  value.input.selection.source = {
    kind: 'known',
    value: { family: 'resolution', id: id(40), revision: 1 },
  };
  let state = initialDeadlinePolicies(value, {
    profile: 'fixed',
    source: 'follow',
    calendar: 'undetermined',
  });
  value.input.selection.source = { kind: 'unknown', reason: 'No consta' };
  state = reconcileDeadlinePolicies(state, value);
  assert.equal(state.source.value, '');
  assert.equal(deadlinePoliciesCommand(state, value).source, 'undetermined');
  value.input.selection.source = {
    kind: 'known',
    value: { family: 'resolution', id: id(40), revision: 1 },
  };
  state = reconcileDeadlinePolicies(state, value);
  assert.equal(state.source.value, '');
  assert.throws(() => deadlinePoliciesCommand(state, value), /fuente/i);
});

test('notification parent revisions retain intention but a different parent root clears it', () => {
  const value = definition();
  value.input.selection.source = {
    kind: 'known',
    value: {
      family: 'notification',
      id: id(41),
      revision: 1,
      resolution: { id: id(42), revision: 1 },
    },
  };
  let state = initialDeadlinePolicies(value, {
    profile: 'follow',
    source: 'fixed',
    calendar: 'undetermined',
  });
  value.input.selection.source.value.resolution.revision = 2;
  state = reconcileDeadlinePolicies(state, value);
  assert.equal(state.source.value, 'fixed');
  value.input.selection.source.value.resolution.id = id(43);
  state = reconcileDeadlinePolicies(state, value);
  assert.equal(state.source.value, '');
});

test('a context change cannot reuse a policy even for the same global profile root', () => {
  const value = definition();
  const state = initialDeadlinePolicies(value, {
    profile: 'follow',
    source: 'undetermined',
    calendar: 'undetermined',
  });
  value.input.selection.case_id = id(50);
  assert.equal(reconcileDeadlinePolicies(state, value).profile.value, '');
  assert.throws(() => deadlinePoliciesCommand(state, value), /perfil/i);
});

test('incomplete known selections cannot be disguised as absent dependencies', () => {
  const value = definition();
  const state = initialDeadlinePolicies(value, {
    profile: 'fixed',
    source: 'undetermined',
    calendar: 'undetermined',
  });
  value.input.selection.source = { kind: 'known', value: { family: 'resolution' } };
  assert.throws(() => deadlinePoliciesCommand(state, value), /fuente/i);
});
