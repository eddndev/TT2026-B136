import { factTime } from './procedural-fact-time.mjs';
import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factText as text,
  factLabel as label,
  factDigest as digest,
  factOptional as optional,
  factMaxRevision,
} from './procedural-fact-primitives.mjs';
export function factDeclaration(raw, parse) {
  if (raw?.kind === 'unknown') {
    object(raw, ['kind', 'reason']);
    return { kind: 'unknown', reason: text(raw.reason) };
  }
  object(raw, ['kind', 'value']);
  if (raw.kind !== 'known') invalid('Indica qu\u00e9 dato consta o por qu\u00e9 se desconoce.');
  return { kind: 'known', value: parse(raw.value) };
}
export function factCatalog(raw, known, other = true) {
  if (raw?.kind === 'other' && other) {
    object(raw, ['kind', 'label']);
    return { kind: 'other', label: label(raw.label) };
  }
  object(raw, ['kind']);
  if (!known.includes(raw.kind)) invalid('Selecciona una categor\u00eda declarada.');
  return { kind: raw.kind };
}
function person(raw) {
  if (raw?.kind === 'participant') {
    object(raw, ['kind', 'id', 'revision']);
    return { kind: 'participant', id: uuid(raw.id), revision: revision(raw.revision) };
  }
  object(raw, ['kind', 'label', 'description']);
  if (raw.kind !== 'unlinked') invalid();
  return { kind: 'unlinked', label: label(raw.label), description: text(raw.description) };
}
function support(raw) {
  object(raw, ['document_id', 'version', 'digest', 'locator']);
  return {
    document_id: uuid(raw.document_id),
    version: revision(raw.version),
    digest: digest(raw.digest),
    locator: label(raw.locator),
  };
}
function hearing(raw) {
  object(
    raw,
    ['hearing_id', 'result_id', 'revision', 'agreement_id'],
    ['hearing_id', 'result_id', 'revision'],
  );
  return {
    hearing_id: uuid(raw.hearing_id),
    result_id: uuid(raw.result_id),
    revision: revision(raw.revision),
    agreement_id: optional(raw.agreement_id, uuid),
  };
}
function provenance(raw) {
  if (raw?.kind === 'operator_note') {
    object(raw, ['kind', 'note']);
    return { kind: 'operator_note', note: text(raw.note) };
  }
  if (raw?.kind === 'external_reference') {
    object(raw, ['kind', 'reference', 'support'], ['kind', 'reference']);
    return {
      kind: raw.kind,
      reference: text(raw.reference),
      support: optional(raw.support, support),
    };
  }
  object(raw, ['kind', 'reference', 'locator', 'support'], ['kind', 'reference', 'locator']);
  if (raw.kind !== 'hearing_result') invalid('Selecciona la procedencia de la declaraci\u00f3n.');
  return {
    kind: raw.kind,
    reference: hearing(raw.reference),
    locator: label(raw.locator),
    support: optional(raw.support, support),
  };
}
function representation(raw) {
  if (raw?.kind === 'not_recorded') {
    object(raw, ['kind', 'reason']);
    return { kind: raw.kind, reason: text(raw.reason) };
  }
  object(raw, ['kind', 'represented', 'representative', 'scope', 'provenance']);
  if (raw.kind !== 'declared') invalid();
  return {
    kind: raw.kind,
    represented: person(raw.represented),
    representative: person(raw.representative),
    scope: text(raw.scope),
    provenance: provenance(raw.provenance),
  };
}
function effect(raw) {
  object(raw, ['at', 'statement', 'locator']);
  return { at: factTime(raw.at), statement: text(raw.statement), locator: label(raw.locator) };
}
const catalogs = {
  class: ['order', 'judgment'],
  character: ['personal', 'publication'],
  medium: ['in_person', 'electronic'],
  context: ['in_hearing', 'outside_hearing'],
  outcome: ['practiced', 'attempted'],
};
export function factValues(family, raw) {
  const common = ['subtype', 'summary', 'provenance'];
  if (family === 'resolution') {
    object(
      raw,
      [...common, 'class', 'issuer', 'issued_at'],
      ['class', 'issuer', 'issued_at', 'summary', 'provenance'],
    );
    return {
      class: factDeclaration(raw.class, (v) => factCatalog(v, catalogs.class)),
      subtype: optional(raw.subtype, label),
      issuer: factDeclaration(raw.issuer, label),
      issued_at: factTime(raw.issued_at),
      summary: text(raw.summary),
      provenance: provenance(raw.provenance),
    };
  }
  if (family !== 'notification') invalid('Selecciona resoluci\u00f3n o notificaci\u00f3n.');
  const fields = [
    ...common,
    'resolution',
    'character',
    'medium',
    'context',
    'outcome',
    'practiced_at',
    'received_at',
    'stated_effect',
    'intended_recipient',
    'actual_receiver',
    'representation',
  ];
  object(
    raw,
    fields,
    fields.filter((k) => !['subtype', 'received_at', 'stated_effect'].includes(k)),
  );
  object(raw.resolution, ['id', 'revision']);
  const value = {
    resolution: { id: uuid(raw.resolution.id), revision: revision(raw.resolution.revision) },
    ...Object.fromEntries(
      ['character', 'medium', 'context', 'outcome'].map((k) => [
        k,
        factDeclaration(raw[k], (v) => factCatalog(v, catalogs[k], k !== 'outcome')),
      ]),
    ),
    subtype: optional(raw.subtype, label),
    practiced_at: factTime(raw.practiced_at),
    received_at: optional(raw.received_at, factTime),
    stated_effect: optional(raw.stated_effect, effect),
    intended_recipient: factDeclaration(raw.intended_recipient, person),
    actual_receiver: factDeclaration(raw.actual_receiver, person),
    representation: representation(raw.representation),
    summary: text(raw.summary),
    provenance: provenance(raw.provenance),
  };
  const direct = [value.provenance.support, value.representation.provenance?.support].filter(
    Boolean,
  );
  if (
    direct.length === 2 &&
    direct[0].document_id === direct[1].document_id &&
    direct[0].version === direct[1].version &&
    direct[0].digest !== direct[1].digest
  )
    invalid('Las funciones del mismo soporte deben conservar la misma huella exacta.');
  return value;
}
export function factDraft(family, record = null, parent = null) {
  if (!['resolution', 'notification'].includes(family)) invalid();
  if (record) {
    if (record.family !== family) invalid();
    return { values: structuredClone(record.values), reason: '' };
  }
  const blank = () => ({ kind: '' }),
    time = () => ({ precision: '' });
  const values = { subtype: null, summary: '', provenance: blank() };
  if (family === 'resolution')
    Object.assign(values, { class: blank(), issuer: blank(), issued_at: time() });
  else
    Object.assign(values, {
      resolution: { id: parent?.id || '', revision: parent?.revision },
      character: blank(),
      medium: blank(),
      context: blank(),
      outcome: blank(),
      practiced_at: time(),
      received_at: null,
      stated_effect: null,
      intended_recipient: blank(),
      actual_receiver: blank(),
      representation: blank(),
    });
  return { values, reason: '' };
}
export function factCommand(draft, { family, action, base, operationId, id, resolutionId }) {
  if (
    !['resolution', 'notification'].includes(family) ||
    !['record', 'correct', 'withdraw'].includes(action)
  )
    invalid();
  const result = { family, operation_id: uuid(operationId), id: uuid(id) };
  if (family === 'notification') result.resolution_id = uuid(resolutionId);
  if (action === 'record') {
    if (base) invalid('El alta requiere una declaraci\u00f3n nueva.');
    result.change = { action, expected_revision: 0, values: factValues(family, draft.values) };
  } else {
    if (
      !base ||
      base.family !== family ||
      uuid(base.id) !== result.id ||
      base.status !== 'recorded' ||
      (family === 'notification' && uuid(base.resolution_id) !== result.resolution_id)
    )
      invalid('Consulta la base vigente de esta declaraci\u00f3n.');
    const expected_revision = revision(base.revision, factMaxRevision - 1);
    result.change = {
      action,
      expected_revision,
      ...(action === 'correct' ? { values: factValues(family, draft.values) } : {}),
      reason: text(draft.reason, 'Motivo'),
    };
  }
  if (
    family === 'notification' &&
    result.change.values &&
    result.change.values.resolution.id !== result.resolution_id
  )
    invalid('La resoluci\u00f3n padre es inmutable.');
  return result;
}
