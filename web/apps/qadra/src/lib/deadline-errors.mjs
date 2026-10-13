const messages = {
  deadline_revision_conflict:
    'El plazo cambi\u00f3. Conserva el borrador y consulta la revisi\u00f3n actual.',
  deadline_operation_conflict:
    'Esta operaci\u00f3n ya fue utilizada. Consulta su revisi\u00f3n exacta.',
  deadline_revision_exhausted: 'El plazo agot\u00f3 sus revisiones. Su historia sigue disponible.',
  deadline_retired: 'El plazo fue retirado y conserva su historia.',
  deadline_profile_unavailable:
    'El perfil seleccionado ya no est\u00e1 disponible para este registro. Revisa su revisi\u00f3n.',
  deadline_responsible_unavailable:
    'La persona responsable ya no tiene el acceso requerido. Consulta el selector de nuevo.',
  deadline_submission_mismatch:
    'Cambiaron los datos revisados. Conserva el borrador y prepara de nuevo.',
  deadline_not_found:
    'El plazo o revisi\u00f3n exacta no est\u00e1 disponible. Consulta el resultado de cualquier env\u00edo pendiente.',
  invalid_deadline:
    'Revisa las declaraciones, fuentes, perfil, calendario y responsable seleccionados.',
  deadline_profile_not_found: 'El perfil o su revisi\u00f3n exacta no est\u00e1 disponible.',
  case_closed:
    'El expediente est\u00e1 cerrado. Conserva el borrador; la historia permanece disponible.',
  case_not_found: 'El expediente no est\u00e1 disponible con tu acceso actual.',
  permission_denied: 'Tu acceso actual no permite esta operaci\u00f3n.',
};
export function deadlineFailure(error = {}) {
  if (error.status === 413) return 'La declaraci\u00f3n supera el l\u00edmite de 1 MiB.';
  return (
    messages[error.code] ||
    error.message ||
    'No se pudo confirmar la operaci\u00f3n. Consulta su revisi\u00f3n exacta.'
  );
}
export const canDeadlines = (role, action = 'read') =>
  action === 'read'
    ? ['owner', 'litigator', 'paralegal'].includes(role)
    : action === 'manage' && ['owner', 'litigator'].includes(role);
export const deadlineDenied = (error) =>
  [401, 403].includes(error?.status) || (error?.status === 404 && error.code === 'case_not_found');
export const deadlineUncertain = (error) => !error?.status || error.status >= 500;
