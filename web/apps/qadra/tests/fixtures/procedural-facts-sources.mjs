import {
  factCaseId,
  factKey,
  factPrepared,
  emptyFactSources,
  factResolutionSource,
} from './procedural-facts.mjs';
export function prepareFact(state, command) {
  const results = state.results;
  const base = state.records.get(factKey(command))?.at(-1);
  const sources = emptyFactSources(),
    values = command.change.values || base?.values;
  if (command.family === 'notification') {
    const parent = state.records
      .get(factKey({ family: 'resolution', id: command.resolution_id }))
      ?.find((row) => row.revision === values.resolution.revision);
    if (!parent) throw new Error('The fixture requires the exact parent resolution');
    sources.resolution = factResolutionSource(parent);
  }
  const declarations = [values.intended_recipient, values.actual_receiver];
  const representation = values.representation;
  if (representation?.kind === 'declared')
    declarations.push(
      { kind: 'known', value: representation.represented },
      { kind: 'known', value: representation.representative },
    );
  for (const declaration of declarations) {
    const ref = declaration?.kind === 'known' && declaration.value;
    if (!ref || ref.kind !== 'participant') continue;
    if (sources.participants.some((row) => row.id === ref.id && row.revision === ref.revision))
      continue;
    const row = results.directory.get(ref.id)?.find((item) => item.revision === ref.revision);
    if (!row) throw new Error('The fixture requires the exact participant');
    sources.participants.push({
      case_id: factCaseId,
      id: row.id,
      revision: row.revision,
      values_digest: row.values_digest,
      directory_status: row.directory_status,
      subject: row.subject
        ? {
            id: row.subject.id,
            revision: row.subject.revision,
            values_digest: row.subject.values_digest,
          }
        : null,
      display_name: row.display_name,
      procedural_role: row.procedural_role,
      organization: row.organization || null,
      kind: row.profile?.kind || null,
    });
  }
  const provenances = [
    values.provenance,
    representation?.kind === 'declared' && representation.provenance,
  ].filter(Boolean);
  for (const provenance of provenances) {
    if (provenance.kind === 'hearing_result') {
      const ref = provenance.reference;
      const row = results.records
        .get(ref.result_id)
        ?.find((item) => item.revision === ref.revision && item.hearing_id === ref.hearing_id);
      if (!row) throw new Error('The fixture requires the exact hearing result');
      const key = (item) =>
        `${item.hearing_id}:${item.result_id}:${item.revision}:${item.agreement_id}`;
      if (!sources.hearing_results.some((item) => key(item) === key(ref))) {
        const time = row.values.event_time,
          [year, month, day] = time.date.split('-').map(Number);
        sources.hearing_results.push({
          case_id: factCaseId,
          ...ref,
          values_digest: row.values_digest,
          submission_digest: row.receipt.submission_digest,
          status: row.status,
          occurrence: row.values.occurrence,
          event_time: { precision: 'date', year, month, day, offset_seconds: -21600 },
          summary: row.values.summary,
          agreement:
            ref.agreement_id === null
              ? null
              : row.values.agreements.find((item) => item.id === ref.agreement_id),
        });
      }
    }
    const support = provenance.support;
    if (
      support &&
      !sources.direct_supports.some(
        (row) => row.document_id === support.document_id && row.version === support.version,
      )
    )
      sources.direct_supports.push({
        document_id: support.document_id,
        version: support.version,
        digest: support.digest,
        name: 'contrato.pdf',
        format: 'pdf',
        policy: 'pdf_docx_v1',
      });
  }
  sources.participants.sort((a, b) => a.id.localeCompare(b.id) || a.revision - b.revision);
  sources.direct_supports.sort(
    (a, b) => a.document_id.localeCompare(b.document_id) || a.version - b.version,
  );
  return factPrepared(command, base, sources);
}
