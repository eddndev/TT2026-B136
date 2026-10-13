import { factSources } from './procedural-fact-sources.mjs';
import {
  factInvalid as invalid,
  factObject as object,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factLabel as label,
  factText as text,
} from './procedural-fact-primitives.mjs';
const compare = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
export function resourceSupports(rows, references) {
  const selected = [
    ...new Map(references.map((v) => [`${v.document_id}/${v.version}`, v])).values(),
  ].sort((a, b) => compare(a.document_id, b.document_id) || a.version - b.version);
  if (!Array.isArray(rows) || rows.length !== selected.length) invalid('Faltan soportes exactos.');
  rows.forEach((row, i) => {
    const ref = selected[i];
    if (
      row.document_id !== ref.document_id ||
      row.version !== ref.version ||
      row.digest !== ref.digest
    )
      invalid('El soporte recibido no corresponde a la versi\u00f3n seleccionada.');
    digest(row.digest);
    text(row.name, 'Nombre documental', 128, false);
    if (!['pdf', 'docx'].includes(row.format) || row.policy !== 'pdf_docx_v1')
      invalid('Falta la validaci\u00f3n capturada del soporte.');
  });
  return rows;
}
export function resourceSources(sources, values, caseId) {
  object(sources, ['resolution', 'appellants', 'supports']);
  factSources(
    { resolution: sources.resolution, participants: [], hearing_results: [], direct_supports: [] },
    { resolution: values.resolution },
    caseId,
  );
  const selected = values.appellants
    .map((v) => v.participant)
    .filter(Boolean)
    .sort((a, b) => compare(a.id, b.id) || a.revision - b.revision);
  if (!Array.isArray(sources.appellants) || sources.appellants.length !== selected.length)
    invalid('Las personas capturadas no corresponden a las fichas seleccionadas.');
  sources.appellants.forEach((row, i) => {
    if (
      row.case_id !== caseId ||
      row.id !== selected[i].id ||
      row.revision !== selected[i].revision
    )
      invalid('La ficha recibida no corresponde al expediente y revisi\u00f3n seleccionados.');
    digest(row.values_digest);
    if (!['active', 'archived'].includes(row.directory_status)) invalid();
    label(row.display_name);
    label(row.procedural_role);
    if (row.organization !== null) label(row.organization);
    if (row.subject !== null) {
      uuid(row.subject?.id);
      revision(row.subject?.revision);
      digest(row.subject?.values_digest);
      if (typeof row.kind !== 'string' || !row.kind) invalid();
    } else if (row.kind !== null) invalid();
  });
  resourceSupports(sources.supports, [values.resolution_evidence]);
  return sources;
}
