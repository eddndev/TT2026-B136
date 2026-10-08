import test from 'node:test';
import assert from 'node:assert/strict';
import { refreshDecisionSources } from '../src/components/measure-decision-editor-sources.mjs';
import { proposalFromRecord } from '../src/components/measure-decision-editor-values.mjs';
import { measureRecord, measureCaseId, otherMeasureId } from './fixtures/measure-records.mjs';

test('restoring an imposition ignores a previous measure discarded by changing the action', async () => {
  const previous = measureRecord(),
    proposal = proposalFromRecord(previous),
    calls = [];
  proposal.id = otherMeasureId;
  const client = () => ({ dispose() {} });
  const api = {
    caseTypedParticipants: () => ({
      ...client(),
      async subjectRevision(id, revision) {
        calls.push({ kind: 'subject', id, revision });
        return structuredClone(proposal.subject);
      },
    }),
    caseDocuments: client,
    caseHearings: client,
    casePrecautionaryHearings: client,
    caseMeasures: () => ({
      ...client(),
      async exact(reference) {
        calls.push({ kind: 'discarded_measure', reference });
        throw Object.assign(new Error('permission_denied'), { status: 403 });
      },
    }),
  };
  const state = {
    support: null,
    anchor: null,
    fields: { outcome: 'changes' },
    effects: [{ action: 'impose', previous, proposal }],
  };
  assert.equal(await refreshDecisionSources(api, measureCaseId, state, () => true), true);
  assert.deepEqual(calls, [
    { kind: 'subject', id: proposal.subject.id, revision: proposal.subject.revision },
  ]);
});
