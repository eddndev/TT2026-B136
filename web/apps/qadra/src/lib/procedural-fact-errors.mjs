const messages = {
  procedural_fact_revision_conflict:
    'La declaraci\u00f3n cambi\u00f3. Conserva el borrador y consulta la base actual.',
  procedural_fact_already_withdrawn:
    'La declaraci\u00f3n ya fue retirada. Su historia permanece disponible.',
  procedural_fact_operation_conflict:
    'La operaci\u00f3n ya fue utilizada. Consulta el recibo de la revisi\u00f3n exacta.',
  procedural_fact_submission_mismatch:
    'La preparaci\u00f3n no coincide con el env\u00edo. Revisa el borrador y prepara de nuevo.',
  procedural_fact_support_changed:
    'El soporte cambi\u00f3. Consulta de nuevo su versi\u00f3n exacta antes de preparar.',
  procedural_fact_revision_exhausted:
    'Se agotaron las revisiones de esta declaraci\u00f3n. Su historia sigue disponible.',
  procedural_fact_invalid_reference:
    'Revisa las personas, antecedentes y soportes exactos seleccionados.',
  procedural_fact_not_found:
    'La declaraci\u00f3n o revisi\u00f3n exacta no est\u00e1 disponible. Esto no confirma el resultado de un env\u00edo incierto.',
  procedural_fact_reference_not_found:
    'Una fuente exacta no est\u00e1 disponible en este expediente. Revisa su selecci\u00f3n.',
  procedural_fact_support_too_large:
    'El lote directo supera el presupuesto de lectura: hasta dos versiones, 16 MiB cada una y 32 MiB en total.',
  procedural_fact_support_format_rejected: 'Un soporte no cumple el formato PDF o DOCX admitido.',
  procedural_fact_support_validation_limit:
    'El lote de soportes supera el presupuesto compartido de validaci\u00f3n.',
  procedural_fact_support_digest_mismatch:
    'La huella no coincide con el soporte. Consulta su versi\u00f3n exacta.',
  invalid_procedural_fact:
    'Revisa los datos declarados, las referencias exactas y los motivos requeridos.',
  invalid_declared_procedural_time:
    'Revisa la fecha, hora, precisi\u00f3n y desfase declarados, sin completar datos desconocidos.',
  case_closed:
    'El expediente est\u00e1 cerrado. Conserva el borrador; las consultas hist\u00f3ricas siguen disponibles.',
  case_not_found: 'El expediente no est\u00e1 disponible con tu acceso actual.',
  permission_denied: 'Tu acceso actual no permite esta operaci\u00f3n.',
};
export function factFailure(error = {}) {
  if (error.status === 413) return 'La declaraci\u00f3n supera el l\u00edmite de 512 KiB.';
  return (
    messages[error.code] ||
    error.message ||
    'No se pudo confirmar la operaci\u00f3n. Consulta su revisi\u00f3n exacta.'
  );
}
export const canFacts = (role, operation = 'read') =>
  operation === 'read'
    ? ['owner', 'litigator', 'paralegal'].includes(role)
    : operation === 'manage' && ['owner', 'litigator'].includes(role);
export const factDenied = (error) =>
  [401, 403].includes(error?.status) || (error?.status === 404 && error.code === 'case_not_found');
export const factUncertain = (error) => !error?.status || error.status >= 500;
