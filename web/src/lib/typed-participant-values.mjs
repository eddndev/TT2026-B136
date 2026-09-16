import { profileKinds, fieldDraft } from './typed-participant-fields.mjs';
export { profileKinds } from './typed-participant-fields.mjs';
export function inputError(field, message) {
  const error = new Error(message);
  error.field = field;
  return error;
}
export function participantText(value, label, limit = 200, optional = false) {
  const raw = String(value ?? '');
  if (/[\u0000-\u001f\u007f-\u009f]/.test(raw))
    throw inputError(label, `${label}: no se permiten caracteres de control.`);
  const result = raw.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');
  if ((!optional && !result) || [...result].length > limit)
    throw inputError(
      label,
      `${label}: ${!result ? 'completa este campo' : `usa hasta ${limit} caracteres`}.`,
    );
  return result || null;
}
export function locatorValue(value, label = 'Soporte') {
  if (
    !value ||
    !(value.document_id || value.id) ||
    !Number.isInteger(value.version) ||
    value.version < 1 ||
    !/^[0-9a-f]{64}$/.test(value.digest || '')
  )
    throw inputError(label, `${label}: selecciona una versi\u00f3n exacta del soporte.`);
  return {
    document_id: value.document_id || value.id,
    version: value.version,
    digest: value.digest,
    locator: participantText(value.locator, `${label}: p\u00e1gina o secci\u00f3n`),
  };
}
function declaredValue(value, label, normalize) {
  if (value?.state === 'unknown')
    return { state: 'unknown', reason: participantText(value.reason, `Motivo de ${label}`, 500) };
  if (value?.state !== 'known') throw inputError(label, `${label}: indica si conoces el dato.`);
  return { state: 'known', value: normalize(value.value) };
}
function licenseValue(value, label) {
  const number = participantText(value?.number, label, 32);
  if (!/^[a-zA-Z0-9]+$/.test(number))
    throw inputError(
      label,
      `${label}: usa letras y n\u00fameros, conservando los ceros iniciales.`,
    );
  return { number, issuer: participantText(value?.issuer, `Emisor de ${label}`) };
}
function fieldValue(value, field) {
  const plain = { ...field, declared: false };
  if (field.declared) return declaredValue(value, field.label, (entry) => fieldValue(entry, plain));
  if (field.type === 'license') return licenseValue(value, field.label);
  if (field.type === 'choice') {
    if (!Object.hasOwn(field.options, value))
      throw inputError(field.label, `${field.label}: selecciona una opci\u00f3n.`);
    return value;
  }
  if (['contact', 'protection'].includes(field.type)) {
    if (value?.state === 'documented')
      return { state: 'documented', support: locatorValue(value.support, field.label) };
    const absent = field.type === 'contact' ? 'not_recorded' : 'none_declared';
    if (!['unknown', absent].includes(value?.state))
      throw inputError(field.label, `${field.label}: selecciona un estado.`);
    return {
      state: value.state,
      reason: participantText(value.reason, `Motivo de ${field.label}`, 500),
    };
  }
  return participantText(value, field.label, field.limit);
}
export function subjectDraft(values) {
  return values
    ? structuredClone(values)
    : {
        kind: 'natural_person',
        name: { state: 'known', value: '' },
        curp: { state: 'known', value: '' },
        identity_support: null,
      };
}
export function subjectValues(draft) {
  const identity_support = locatorValue(draft.identity_support, 'Soporte de identidad');
  if (draft.kind === 'institutional_body')
    return {
      kind: draft.kind,
      name: participantText(draft.name, 'Nombre del \u00f3rgano'),
      institutional_identifier: declaredValue(
        draft.institutional_identifier,
        'identificador institucional',
        (value) => participantText(value, 'Identificador institucional', 80),
      ),
      identity_support,
    };
  if (draft.kind !== 'natural_person')
    throw inputError('Identidad', 'Selecciona persona u \u00f3rgano.');
  let name;
  if (draft.name?.state === 'known')
    name = { state: 'known', value: participantText(draft.name.value, 'Nombre de la persona') };
  else if (draft.name?.state === 'unidentified')
    name = {
      state: 'unidentified',
      label: participantText(draft.name.label, 'Etiqueta de la persona'),
      reason: participantText(draft.name.reason, 'Motivo de identidad desconocida', 500),
    };
  else throw inputError('Nombre', 'Indica si el nombre de la persona es conocido.');
  const curp = declaredValue(draft.curp, 'CURP', (raw) => {
    const value = participantText(raw, 'CURP', 18).replace(/[a-z]/g, (letter) =>
      letter.toUpperCase(),
    );
    if (!/^[A-Z0-9]{18}$/.test(value))
      throw inputError('CURP', 'CURP: usa exactamente 18 letras o n\u00fameros.');
    return value;
  });
  return { kind: draft.kind, name, curp, identity_support };
}
export function roleDraft(kind = '', record) {
  if (record)
    return {
      organization: record.organization || '',
      legal_status: record.legal_status || '',
      profile: structuredClone(record.profile),
      role_support: structuredClone(record.role_support),
    };
  return {
    organization: '',
    legal_status: '',
    profile: {
      kind,
      ...Object.fromEntries(
        (profileKinds.find((item) => item.key === kind)?.fields || []).map((field) => [
          field.key,
          fieldDraft(field),
        ]),
      ),
    },
    role_support: null,
  };
}
export function roleValues(draft) {
  const schema = profileKinds.find((item) => item.key === draft.profile?.kind);
  if (!schema) throw inputError('Tipo de participante', 'Selecciona el tipo de participante.');
  return {
    organization: participantText(draft.organization, 'Organizaci\u00f3n', 200, true),
    legal_status: participantText(
      draft.legal_status,
      'Situaci\u00f3n jur\u00eddica declarada',
      160,
      true,
    ),
    profile: {
      kind: schema.key,
      ...Object.fromEntries(
        schema.fields.map((field) => [field.key, fieldValue(draft.profile[field.key], field)]),
      ),
    },
    role_support: locatorValue(draft.role_support, 'Soporte del rol'),
  };
}
export const candidateKey = (ref) => `${ref.kind}:${ref.id}:${ref.revision}`;
export function reviewValues(result, reason, decisions) {
  const different = result.candidates.map(({ reference }) => {
    const value = decisions[candidateKey(reference)];
    if (!value)
      throw inputError('Candidatos', 'Revisa y decide expl\u00edcitamente sobre cada candidato.');
    return {
      candidate: reference,
      reason: participantText(value.reason, 'Motivo de candidato distinto', 200),
      support: locatorValue(value.support, 'Soporte de comparaci\u00f3n'),
    };
  });
  return {
    directory_stamp: result.directory_stamp,
    selection_reason: participantText(reason, 'Motivo de selecci\u00f3n de identidad', 500),
    different,
  };
}
export function validateSupportSet(...values) {
  const records = new Map();
  function visit(value) {
    if (!value || typeof value !== 'object') return;
    if (value.document_id && value.version && value.digest) {
      const key = `${value.document_id}:${value.version}`;
      if (records.has(key) && records.get(key) !== value.digest)
        throw new Error('Una misma versi\u00f3n tiene un digest diferente. Vuelve a consultarla.');
      records.set(key, value.digest);
    } else for (const entry of Object.values(value)) visit(entry);
  }
  values.forEach(visit);
  if (records.size > 2)
    throw new Error(
      'Esta operaci\u00f3n admite como m\u00e1ximo dos versiones documentales distintas.',
    );
}
