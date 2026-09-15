export const participantFields = [
  { key: 'display_name', label: 'Nombre del participante', limit: 200, required: true },
  { key: 'procedural_role', label: 'Rol en el expediente', limit: 80, required: true },
  { key: 'organization', label: 'Organizaci\u00f3n (opcional)', limit: 200 },
  { key: 'legal_status', label: 'Situaci\u00f3n jur\u00eddica registrada (opcional)', limit: 160 },
];
export function canParticipants(role, action) {
  if (action === 'read') return ['owner', 'litigator', 'paralegal'].includes(role);
  return action === 'manage' && ['owner', 'litigator'].includes(role);
}
function textValue(input, field) {
  const value = String(input || '');
  let reason = '';
  if (/[\u0000-\u001f\u007f-\u009f]/.test(value)) reason = 'no se permiten caracteres de control';
  const normalized = value.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');
  if (!reason && field.required && !normalized) reason = 'completa este campo';
  if (!reason && [...normalized].length > field.limit)
    reason = `usa hasta ${field.limit} caracteres`;
  if (reason) {
    const error = new Error(`${field.label}: ${reason}.`);
    error.field = field.key;
    throw error;
  }
  return normalized || null;
}
export function participantDraft(record = {}) {
  return Object.fromEntries([
    ...participantFields.map(({ key }) => [key, record[key] || '']),
    ['directory_status', record.directory_status || 'active'],
  ]);
}
export function participantValues(draft) {
  return Object.fromEntries(
    participantFields.map((field) => [field.key, textValue(draft[field.key], field)]),
  );
}
export function participantFilters(input) {
  if (!['active', 'archived', 'all'].includes(input.status))
    throw new Error('Estado del directorio no v\u00e1lido.');
  const name = textValue(input.name, { ...participantFields[0], required: false });
  const role = textValue(input.procedural_role, { ...participantFields[1], required: false });
  return {
    ...(name ? { name } : {}),
    ...(role ? { procedural_role: role } : {}),
    status: input.status,
  };
}
export function participantFailure(failure) {
  if (!failure.status)
    return 'No se pudo confirmar el resultado. Consulta los datos guardados antes de volver a enviar.';
  if (failure.status === 413) return 'Los datos del participante superan el l\u00edmite permitido.';
  return failure.message;
}
