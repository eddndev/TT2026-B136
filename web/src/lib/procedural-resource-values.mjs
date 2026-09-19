import { factDeclaration } from './procedural-fact-values.mjs';
import { factTime } from './procedural-fact-time.mjs';
import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factLabel as label,
  factText as text,
  factOptional as optional,
  factMaxRevision,
} from './procedural-fact-primitives.mjs';

export const resourceKinds = { revocation: 'Revocaci\u00f3n', appeal: 'Apelaci\u00f3n' };
export const resourceActKinds = {
  interposition: 'Interposici\u00f3n',
  admission: 'Admisi\u00f3n',
  inadmissibility: 'Inadmisibilidad',
  withdrawal: 'Desistimiento',
  resolution: 'Resoluci\u00f3n',
};
export const resourceActions = {
  register: 'Registrar recurso',
  correct: 'Corregir recurso',
  record_act: 'Registrar acto',
  correct_act: 'Corregir acto',
  archive: 'Archivar recurso',
  reactivate: 'Reactivar recurso',
};
function catalog(value, keys) {
  if (typeof value !== 'string' || !keys.includes(value)) invalid('Selecciona el tipo declarado.');
  return value;
}
export const resourceMode = (value) =>
  factDeclaration(value, (v) => catalog(v, ['oral', 'written']));
export function resourceReference(raw) {
  object(raw, ['id', 'revision']);
  return { id: uuid(raw.id), revision: revision(raw.revision) };
}
export function resourceEvidence(raw) {
  object(raw, ['document_id', 'version', 'digest', 'locator']);
  return {
    document_id: uuid(raw.document_id),
    version: revision(raw.version),
    digest: digest(raw.digest),
    locator: label(raw.locator),
  };
}
export function resourceValues(raw) {
  object(raw, [
    'kind',
    'mode',
    'title',
    'resolution',
    'resolution_evidence',
    'resolution_reference',
    'issuing_authority',
    'receiving_authority',
    'resolution_at',
    'notification_at',
    'challenged_part',
    'grounds',
    'appellants',
  ]);
  if (!Array.isArray(raw.appellants) || !raw.appellants.length || raw.appellants.length > 32)
    invalid('Indica entre una y 32 personas recurrentes.');
  const selected = new Set();
  const appellants = raw.appellants.map((row) => {
    object(row, ['name', 'role', 'participant']);
    const participant = optional(row.participant, resourceReference);
    if (participant) {
      if (selected.has(participant.id)) invalid('La misma ficha aparece m\u00e1s de una vez.');
      selected.add(participant.id);
    }
    return { name: label(row.name), role: factDeclaration(row.role, label), participant };
  });
  return {
    kind: catalog(raw.kind, Object.keys(resourceKinds)),
    mode: resourceMode(raw.mode),
    title: label(raw.title),
    resolution: resourceReference(raw.resolution),
    resolution_evidence: resourceEvidence(raw.resolution_evidence),
    resolution_reference: factDeclaration(raw.resolution_reference, label),
    issuing_authority: factDeclaration(raw.issuing_authority, label),
    receiving_authority: optional(raw.receiving_authority, (v) => factDeclaration(v, label)),
    resolution_at: factTime(raw.resolution_at),
    notification_at: optional(raw.notification_at, factTime),
    challenged_part: text(raw.challenged_part),
    grounds: text(raw.grounds),
    appellants,
  };
}
export function resourceActValues(raw) {
  object(raw, ['kind', 'mode', 'occurred_at', 'authority', 'statement', 'evidence']);
  if (!Array.isArray(raw.evidence) || !raw.evidence.length || raw.evidence.length > 2)
    invalid('Cada acto requiere uno o dos soportes documentales.');
  const evidence = raw.evidence.map(resourceEvidence),
    seen = new Map();
  for (const row of evidence) {
    const key = `${row.document_id}/${row.version}`;
    if (seen.has(key) && seen.get(key) !== row.digest)
      invalid('Las huellas del mismo soporte difieren.');
    seen.set(key, row.digest);
  }
  return {
    kind: catalog(raw.kind, Object.keys(resourceActKinds)),
    mode: resourceMode(raw.mode),
    occurred_at: factTime(raw.occurred_at),
    authority: factDeclaration(raw.authority, label),
    statement: text(raw.statement),
    evidence,
  };
}
export function resourceCommand(raw) {
  object(raw, ['operation_id', 'resource_id', 'change']);
  const change = raw.change,
    action = catalog(change?.action, Object.keys(resourceActions));
  const hasValues = ['register', 'correct', 'record_act', 'correct_act'].includes(action);
  const hasReason = ['correct', 'correct_act', 'archive', 'reactivate'].includes(action);
  const act = ['record_act', 'correct_act'].includes(action);
  object(change, [
    'action',
    'expected_revision',
    ...(hasValues ? ['values'] : []),
    ...(hasReason ? ['reason'] : []),
    ...(act ? ['act_id'] : []),
    ...(action === 'correct_act' ? ['expected_act_revision'] : []),
  ]);
  if (action === 'register') {
    if (change.expected_revision !== 0) invalid();
  } else revision(change.expected_revision, factMaxRevision - 1);
  return {
    operation_id: uuid(raw.operation_id),
    resource_id: uuid(raw.resource_id),
    change: {
      action,
      expected_revision: change.expected_revision,
      ...(hasValues ? { values: (act ? resourceActValues : resourceValues)(change.values) } : {}),
      ...(hasReason ? { reason: text(change.reason, 'Motivo') } : {}),
      ...(act ? { act_id: uuid(change.act_id) } : {}),
      ...(action === 'correct_act'
        ? { expected_act_revision: revision(change.expected_act_revision, factMaxRevision - 1) }
        : {}),
    },
  };
}
export function resourceDraft(record = null, act = false) {
  if (record)
    return { values: structuredClone(act ? record.act.values : record.values), reason: '' };
  const blank = () => ({ kind: '' }),
    time = () => ({ precision: '' });
  return {
    reason: '',
    values: act
      ? {
          kind: '',
          mode: blank(),
          occurred_at: time(),
          authority: blank(),
          statement: '',
          evidence: [null],
        }
      : {
          kind: '',
          mode: blank(),
          title: '',
          resolution: null,
          resolution_evidence: null,
          resolution_reference: blank(),
          issuing_authority: blank(),
          receiving_authority: null,
          resolution_at: time(),
          notification_at: null,
          challenged_part: '',
          grounds: '',
          appellants: [{ name: '', role: blank(), participant: null }],
        },
  };
}
