import test from 'node:test';
import assert from 'node:assert/strict';
import {
  decisionFormCommand,
  newMeasureEffect,
  proposalFromRecord,
} from '../src/components/measure-decision-editor-values.mjs';
import { measureRecord, measureCaseId, otherMeasureId } from './fixtures/measure-records.mjs';
import { hearingRecord } from './fixtures/hearings.mjs';
import { preparedDecision } from './fixtures/measure-decision-workflow.mjs';

function form() {
  const prepared = preparedDecision(),
    command = prepared.review.command;
  return {
    caseId: measureCaseId,
    decisionId: command.decision_id,
    operationId: command.operation_id,
    fields: {
      authority: command.values.authority,
      justification: command.values.justification,
      locator: command.values.locator,
      declaredAt: command.values.declared_at,
      outcome: 'changes',
      statement: '',
    },
    support: prepared.review.material.support,
    anchor: null,
    effects: [],
  };
}
const context = () => ({ expectation: preparedDecision().review.command.context });

test('decision form preserves grouped exact references and declared values without inventing time', () => {
  const state = form(),
    prior = measureRecord(),
    proposal = proposalFromRecord(prior);
  proposal.id = otherMeasureId;
  state.effects = [
    { ...newMeasureEffect(), action: 'confirm', previous: prior },
    { ...newMeasureEffect(), action: 'impose', proposal },
  ];
  const command = decisionFormCommand(state, context());
  assert.deepEqual(command.outcome.effects, [
    { action: 'confirm', previous: prior.reference },
    {
      action: 'impose',
      proposal: { id: otherMeasureId, values: prior.record.capture.result.values },
    },
  ]);
  assert.deepEqual(command.values.declared_at, state.fields.declaredAt);
  assert.equal(command.anchor, null);
  assert.equal(command.operation_id, state.operationId);
  assert.equal(command.decision_id, state.decisionId);
});

test('substitution retains all predecessors and successors and no-change excludes unfinished effects', () => {
  const state = form(),
    prior = measureRecord(),
    successor = proposalFromRecord(prior);
  successor.id = otherMeasureId;
  state.effects = [
    { ...newMeasureEffect(), action: 'substitute', predecessors: [prior], successors: [successor] },
  ];
  assert.deepEqual(decisionFormCommand(state, context()).outcome.effects, [
    {
      action: 'substitute',
      predecessors: [prior.reference],
      successors: [{ id: otherMeasureId, values: prior.record.capture.result.values }],
    },
  ]);
  state.fields.outcome = 'no_measure_change';
  state.fields.statement = 'No se modificaron las medidas';
  state.effects = [newMeasureEffect()];
  assert.deepEqual(decisionFormCommand(state, context()).outcome, {
    kind: 'no_measure_change',
    statement: state.fields.statement,
  });
});

test('preparation rejects incomplete source selections and keeps modification identity fixed', () => {
  const state = form(),
    prior = measureRecord();
  state.effects = [newMeasureEffect()];
  assert.throws(() => decisionFormCommand(state, context()));
  state.effects = [
    {
      ...newMeasureEffect(),
      action: 'modify',
      previous: prior,
      proposal: proposalFromRecord(prior),
    },
  ];
  state.effects[0].proposal.values.conditions = 'Condiciones modificadas';
  const effect = decisionFormCommand(state, context()).outcome.effects[0];
  assert.deepEqual(effect.previous, prior.reference);
  assert.deepEqual(effect.values.subject, prior.record.capture.result.values.subject);
  assert.equal(effect.values.kind, prior.record.capture.result.values.kind);
  assert.equal(effect.values.conditions, 'Condiciones modificadas');
  state.support = null;
  assert.throws(() => decisionFormCommand(state, context()));
});

test('initial hearing anchor retains the exact receipt without scheduling another appointment', () => {
  const state = form(),
    record = hearingRecord();
  record.case_id = measureCaseId;
  state.fields.outcome = 'no_measure_change';
  state.fields.statement = 'Sin cambio';
  state.anchor = { kind: 'initial', record };
  assert.deepEqual(decisionFormCommand(state, context()).anchor, {
    kind: 'initial',
    hearing_id: record.id,
    revision: record.revision,
    values_digest: record.values_digest,
    submission_digest: record.receipt.submission_digest,
  });
});

test('choosing an origin without its exact revision cannot silently become an independent decision', () => {
  const state = form();
  state.fields.outcome = 'no_measure_change';
  state.fields.statement = 'Sin cambio';
  state.inputs = { anchor: { kind: 'initial', choosing: true } };
  assert.throws(() => decisionFormCommand(state, context()));
});
