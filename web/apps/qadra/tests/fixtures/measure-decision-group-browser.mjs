import { measureDecisionOperation } from './measure-decision-workflow.mjs';
import { measureRecord } from './measure-records.mjs';
import { inMeasureCase, clone } from './measure-decision-browser.mjs';

export function seedDecisionGroup(state, count = 4) {
  const operation = inMeasureCase(measureDecisionOperation(), state.caseId);
  const rows = Array.from({ length: count }, (_, index) =>
    inMeasureCase(
      measureRecord({
        id: `a1000000-0000-4000-8000-${String(index + 1).padStart(12, '0')}`,
      }),
      state.caseId,
    ),
  );
  const group = operation.group;
  group.measures = rows.map((row) => clone(row.record.capture));
  group.review.results = group.measures.map((row) => clone(row.result));
  group.review.material.result_sources = group.measures.map((row) => ({
    id: row.result.id,
    sources: clone(row.result.sources),
  }));
  group.review.command.outcome.effects = group.measures.map((row) => ({
    action: 'impose',
    proposal: { id: row.result.id, values: clone(row.result.values) },
  }));
  group.review.command.values.support = {
    document_id: state.support.id,
    version: state.support.version,
    digest: state.support.digest,
  };
  group.review.material.support = {
    ...clone(group.review.command.values.support),
    name: state.support.name,
    format: 'pdf',
    policy: 'pdf_docx_v1',
  };
  group.decision.values = clone(group.review.command.values);
  group.decision.support = clone(group.review.material.support);
  for (const row of rows) {
    row.record_history.records.judicial.groups = [
      {
        origin: clone(operation.origin),
        capture: clone(group),
      },
    ];
    state.records.set(row.reference.id, [clone(row)]);
  }
  state.operations.set(operation.origin.operation_id, clone(operation));
  return rows;
}
