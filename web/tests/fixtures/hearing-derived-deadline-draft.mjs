import assert from 'node:assert/strict';
import { createDraftRegistry } from '../../src/lib/draft-registry.mjs';
import { hearingResultDraft } from '../../src/lib/hearing-result-values.mjs';
import { initialDeadlinePolicies } from '../../src/lib/deadline-editor-policies.mjs';
import {
  captureHearingDerivedDeadlineDraft,
  createHearingDerivedDeadlineDraft,
} from '../../src/lib/hearing-derived-deadline-draft.mjs';
import { derivedReady, principal, clone } from './hearing-derived-deadline-unit.mjs';

export function draftState(mode = 'uncertain') {
  const last = derivedReady();
  return {
    resultId: last.command.result.result_id,
    deadlineId: last.command.deadline.deadline_id,
    draft: hearingResultDraft(last.result),
    anchor: clone(last.result.anchor),
    continuation: clone(last.result.continuation),
    definition: clone(last.deadline.definition),
    policies: initialDeadlinePolicies(last.deadline.definition, last.deadline.tracking),
    mode,
    last,
    inputs: { attendees: { name: 'Pending name', applied: 'Exact selection' } },
  };
}

export function setup(value = draftState()) {
  const registry = createDraftRegistry();
  let actor = principal(),
    allowed = true,
    status = 'active';
  const caseId = value.definition.input.selection.case_id;
  const hearingId = value.anchor.hearing_id;
  const calls = [];
  const caseRecord = () => ({
    id: caseId,
    administration: { case_id: caseId, administrative_status: status },
  });
  const session = {
    registry,
    principal: () => actor,
    canAdmit: () => allowed,
    authorizeCase: async () => {
      calls.push('case');
      return caseRecord();
    },
  };
  registry.activate(actor.id);
  const controller = (
    read = async () => {
      calls.push('read');
      return { hearing: { id: hearingId }, closed: false };
    },
  ) =>
    createHearingDerivedDeadlineDraft({
      session,
      caseId,
      hearingId,
      read,
      capture: () => captureHearingDerivedDeadlineDraft(value),
    });
  const first = controller();
  first.register(value.resultId, value.anchor.revision);
  const descriptor = () => ({
    principalId: actor.id,
    contextId: caseId,
    editorKind: 'hearing-derived-deadline',
    resourceId: hearingId,
    action: 'register',
    instanceId: value.resultId,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: value.anchor.revision,
  });
  function suspend() {
    const report = registry.suspend();
    first.dispose();
    registry.activate(actor.id);
    assert.equal(report.failed.length, 0);
    return registry.pending()[0];
  }
  return {
    value,
    session,
    registry,
    calls,
    caseId,
    hearingId,
    first,
    controller,
    descriptor,
    suspend,
    caseRecord,
    setPrincipal: (change) => {
      actor = { ...actor, ...change };
    },
    setAllowed: (next) => {
      allowed = next;
    },
    setStatus: (next) => {
      status = next;
    },
    unsafeSnapshot(changeValue, changeDescriptor = () => {}) {
      first.dispose();
      const stored = clone(value),
        entry = descriptor();
      changeValue(stored);
      changeDescriptor(entry);
      registry.register(entry, { fields: Object.keys(stored), capture: () => stored });
      return suspend();
    },
  };
}
