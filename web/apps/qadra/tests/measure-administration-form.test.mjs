import test from 'node:test';
import assert from 'node:assert/strict';
import {
  administrationFormCommand,
  administrationFields,
  assertAdministrationBase,
} from '../src/components/measure-administration-editor-values.mjs';
import { measureRecord, measureCaseId, foreignMeasureId } from './fixtures/measure-records.mjs';
import { preparedAdministration } from './fixtures/measure-administrations.mjs';

function form(action = 'correct') {
  const base = measureRecord(),
    review = preparedAdministration({ action });
  const fields = administrationFields(base);
  fields.reason = review.command.reason;
  fields.conditions = 'Condiciones rectificadas expresamente';
  return {
    caseId: measureCaseId,
    operationId: review.command.operation_id,
    action,
    base,
    fields,
    replacementId:
      action === 'replace_entered_in_error' ? review.command.action.replacement_id : null,
    subject: action === 'replace_entered_in_error' ? review.replacement.sources.subject : null,
  };
}
const context = { expectation: preparedAdministration().command.context };

test('correction retains its exact target and only sends administrative text and declared times', () => {
  const state = form(),
    command = administrationFormCommand(state, context);
  assert.deepEqual(command.target, state.base.reference);
  assert.deepEqual(command.action, {
    kind: 'correct',
    values: {
      conditions: state.fields.conditions,
      validity: state.base.record.capture.result.values.validity,
      supervision_text: state.base.record.capture.result.values.supervision.reason,
    },
  });
  assert.equal(command.reason, state.fields.reason);
  assert.equal(command.operation_id, state.operationId);
  assert.equal(Object.hasOwn(command, 'support'), false);
  assert.equal(Object.hasOwn(command.action.values, 'subject'), false);
});

test('marking and replacement preserve distinct operations and the selected replacement identity', () => {
  assert.deepEqual(administrationFormCommand(form('entered_in_error'), context).action, {
    kind: 'entered_in_error',
  });
  const state = form('replace_entered_in_error'),
    subject = state.subject;
  assert.deepEqual(administrationFormCommand(state, context).action, {
    kind: state.action,
    replacement_id: state.replacementId,
    subject: { id: subject.id, revision: subject.revision, values_digest: subject.values_digest },
  });
  state.subject = null;
  assert.throws(() => administrationFormCommand(state, context));
});

test('a stale or invalid administrative target cannot silently become another current base', () => {
  const base = measureRecord();
  assert.doesNotThrow(() => assertAdministrationBase(base, structuredClone(base)));
  const later = measureRecord({ family: 'c1' });
  assert.throws(() => assertAdministrationBase(base, later));
  assert.throws(() => assertAdministrationBase(base, measureRecord({ id: foreignMeasureId })));
  assert.throws(() => assertAdministrationBase(base, { ...base, validity: 'entered_in_error' }));
});
