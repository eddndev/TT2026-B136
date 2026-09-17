import {
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factText as text,
  factLabel as label,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { factDeclaration, factCatalog } from './procedural-fact-values.mjs';
import { factTime } from './procedural-fact-time.mjs';
const compare = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
const personKey = (v) => `${v.id}/${v.revision}`;
const hearingKey = (v) =>
  `${v.hearing_id}/${v.result_id}/${v.revision}/${v.agreement_id === null ? 'none' : v.agreement_id}`;
const supportKey = (v) => `${v.document_id}/${v.version}`;
const personOrder = (a, b) => compare(a.id, b.id) || a.revision - b.revision;
const hearingOrder = (a, b) =>
  compare(a.hearing_id, b.hearing_id) ||
  compare(a.result_id, b.result_id) ||
  a.revision - b.revision ||
  (a.agreement_id === null
    ? -1
    : b.agreement_id === null
      ? 1
      : compare(a.agreement_id, b.agreement_id));
const supportOrder = (a, b) => compare(a.document_id, b.document_id) || a.version - b.version;
function unique(values, key, order) {
  return [...new Map(values.map((v) => [key(v), v])).values()].sort(order);
}
export function factSelection(values) {
  const people = [values.intended_recipient, values.actual_receiver]
    .filter((v) => v?.kind === 'known')
    .map((v) => v.value);
  if (values.representation?.kind === 'declared')
    people.push(values.representation.represented, values.representation.representative);
  const provenance = [values.provenance, values.representation?.provenance].filter(Boolean);
  return {
    resolution: values.resolution || null,
    participants: unique(
      people.filter((v) => v.kind === 'participant'),
      personKey,
      personOrder,
    ),
    hearing_results: unique(
      provenance.filter((v) => v.kind === 'hearing_result').map((v) => v.reference),
      hearingKey,
      hearingOrder,
    ),
    direct_supports: unique(
      provenance.map((v) => v.support).filter(Boolean),
      supportKey,
      supportOrder,
    ),
  };
}
function state(value) {
  if (!['recorded', 'withdrawn'].includes(value))
    invalid('El estado hist\u00f3rico no es v\u00e1lido.');
}
function captured(row, caseId) {
  if (row?.case_id !== caseId) invalid('La fuente pertenece a otro expediente.');
  digest(row.values_digest);
}
function exact(actual, expected, key, maximum) {
  if (
    !Array.isArray(actual) ||
    actual.length > maximum ||
    actual.length !== expected.length ||
    actual.some((row, i) => !row || key(row) !== key(expected[i]))
  )
    invalid('Las fuentes no corresponden a la selecci\u00f3n exacta.');
}
export function factSources(sources, values, caseId) {
  if (!sources || typeof sources !== 'object' || Array.isArray(sources))
    invalid('Faltan las fuentes exactas.');
  const selection = factSelection(values),
    parent = sources.resolution;
  if (selection.resolution) {
    if (
      !parent ||
      parent.id !== selection.resolution.id ||
      parent.revision !== selection.resolution.revision
    )
      invalid('La resoluci\u00f3n seleccionada no coincide.');
    captured(parent, caseId);
    state(parent.status);
    digest(parent.submission_digest);
    factDeclaration(parent.class, (v) => factCatalog(v, ['order', 'judgment']));
    factDeclaration(parent.issuer, label);
    factTime(parent.issued_at);
    text(parent.summary);
  } else if (parent !== null) invalid('Hay una resoluci\u00f3n padre no seleccionada.');
  exact(sources.participants, selection.participants, personKey, 4);
  for (const row of sources.participants) {
    captured(row, caseId);
    uuid(row.id);
    revision(row.revision);
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
  }
  exact(sources.hearing_results, selection.hearing_results, hearingKey, 2);
  const prior = new Map();
  for (const row of sources.hearing_results) {
    captured(row, caseId);
    digest(row.submission_digest);
    state(row.status);
    text(row.summary);
    if (!['occurred', 'not_started'].includes(row.occurrence)) invalid();
    const time = row.event_time;
    if (!time || !['date', 'instant'].includes(time.precision) || time.offset_seconds == null)
      invalid();
    factTime({ ...time, precision: time.precision === 'instant' ? 'second' : 'date' });
    if (row.agreement_id === null) {
      if (row.agreement !== null) invalid('Hay un acuerdo no seleccionado.');
    } else {
      if (row.agreement?.id !== row.agreement_id)
        invalid('El acuerdo no corresponde a su revisi\u00f3n.');
      text(row.agreement.text);
    }
    const key = `${row.hearing_id}/${row.result_id}/${row.revision}`;
    const common = {
      case_id: row.case_id,
      values_digest: row.values_digest,
      submission_digest: row.submission_digest,
      status: row.status,
      occurrence: row.occurrence,
      event_time: row.event_time,
      summary: row.summary,
    };
    if (prior.has(key) && !same(prior.get(key), common))
      invalid('Dos selecciones contradicen el mismo resultado hist\u00f3rico.');
    prior.set(key, common);
  }
  exact(sources.direct_supports, selection.direct_supports, supportKey, 2);
  sources.direct_supports.forEach((row, i) => {
    if (row.digest !== selection.direct_supports[i].digest)
      invalid('La huella del soporte no coincide.');
    digest(row.digest);
    text(row.name, 'Nombre documental', 128, false);
    if (!['pdf', 'docx'].includes(row.format) || row.policy !== 'pdf_docx_v1')
      invalid('Falta la admisi\u00f3n capturada del soporte.');
  });
  return sources;
}
