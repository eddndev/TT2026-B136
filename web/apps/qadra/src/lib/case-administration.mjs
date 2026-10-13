export const basicFields = [
  { key: 'title', label: 'T\u00edtulo del expediente', limit: 200, required: true },
  { key: 'reference', label: 'Referencia interna', limit: 100, required: true },
];
export const profileFields = [
  { key: 'nuc', label: 'NUC', limit: 100, required: true },
  { key: 'nuc_authority', label: 'Autoridad emisora del NUC', limit: 200, required: true },
  { key: 'judicial_case_number', label: 'Carpeta judicial', limit: 100, required: true },
  {
    key: 'judicial_authority',
    label: '\u00d3rgano emisor de la carpeta',
    limit: 200,
    required: true,
  },
  {
    key: 'general_information',
    label: 'Informaci\u00f3n general (opcional)',
    limit: 1000,
    multiline: true,
  },
  {
    key: 'complementary_identifiers',
    label: 'Identificadores complementarios (opcional)',
    limit: 300,
  },
];
function invalid(field, message) {
  const error = new Error(message);
  error.field = field;
  throw error;
}
export function caseText(input, field) {
  let raw = String(input || '');
  if (field.multiline) raw = raw.replace(/\r\n/g, '\n');
  const controls = field.multiline
    ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/
    : /[\u0000-\u001f\u007f-\u009f]/;
  if (controls.test(raw))
    invalid(field.key, `${field.label}: no se permiten caracteres de control.`);
  const value = raw.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');
  if (field.required && !value) invalid(field.key, `Completa ${field.label.toLowerCase()}.`);
  if ([...value].length > field.limit)
    invalid(field.key, `${field.label}: usa hasta ${field.limit} caracteres.`);
  return value || null;
}
export function offenseValue(value) {
  return caseText(value, {
    key: 'offenses',
    label: 'Descripci\u00f3n de delito',
    limit: 120,
    required: true,
  });
}
export function caseDraft(record = {}) {
  return {
    title: record.title || '',
    reference: record.reference || '',
    profile: {
      ...Object.fromEntries(profileFields.map(({ key }) => [key, record.profile?.[key] || ''])),
      offenses: [...(record.profile?.offenses || [])],
    },
  };
}
export function caseValues(draft, complete) {
  const result = Object.fromEntries(
    basicFields.map((field) => [field.key, caseText(draft[field.key], field)]),
  );
  result.profile = null;
  if (complete) {
    const raw = draft.profile.offenses;
    if (raw.length < 1 || raw.length > 8)
      invalid('offenses', 'Agrega de 1 a 8 descripciones de delito.');
    const offenses = raw.map(offenseValue);
    if (new Set(offenses).size !== offenses.length)
      invalid('offenses', 'Hay descripciones de delito repetidas. Revisa la lista.');
    result.profile = {
      ...Object.fromEntries(
        profileFields.map((field) => [field.key, caseText(draft.profile[field.key], field)]),
      ),
      offenses,
    };
  }
  return result;
}
export function caseFilters(input) {
  if (
    !['active', 'closed', 'all'].includes(input.status) ||
    !['all', 'complete', 'pending'].includes(input.profile)
  )
    throw new Error('Revisa el estado y la ficha seleccionados.');
  const result = { status: input.status, profile: input.profile };
  for (const field of [basicFields[0], profileFields[0], profileFields[2]]) {
    const value = caseText(input[field.key], { ...field, required: false });
    if (value) result[field.key] = value;
  }
  return result;
}
export const staffCase = (role) => ['owner', 'litigator', 'paralegal'].includes(role);
export const manageCase = (role) => ['owner', 'litigator'].includes(role);
export const basicCase = (record) =>
  record.administration
    ? { ...record, title: record.administration.title, reference: record.administration.reference }
    : record;
export function caseFailure(error) {
  if (!error.status)
    return 'No se pudo confirmar el resultado. Consulta los datos guardados antes de volver a enviar.';
  return error.message;
}
