import { resourceFailure } from './procedural-resource-errors.mjs';
export { resourceDenied, resourceUncertain } from './procedural-resource-errors.mjs';
const messages = {
  resource_activity_not_found: 'El v\u00ednculo o su revisi\u00f3n no est\u00e1 disponible.',
  resource_activity_revision_conflict:
    'El v\u00ednculo cambi\u00f3. Conserva tu borrador y compara el registro actual.',
  resource_activity_resource_revision_conflict:
    'El recurso cambi\u00f3. Conserva la selecci\u00f3n exacta y compara su cabeza actual.',
  resource_activity_operation_conflict:
    'El env\u00edo ya tiene un resultado. Consulta su recibo exacto.',
  resource_activity_resource_archived:
    'El recurso est\u00e1 archivado. Puedes consultar o desvincular actividades existentes.',
  resource_activity_state_unchanged:
    'El v\u00ednculo ya est\u00e1 desvinculado. Su historia permanece disponible.',
  resource_activity_source_mismatch:
    'Una fuente exacta no coincide. Consulta y selecciona de nuevo esa fuente.',
  resource_activity_submission_mismatch:
    'La preparaci\u00f3n cambi\u00f3. Conserva la selecci\u00f3n y revisa de nuevo.',
  invalid_resource_activity: 'Revisa la actividad, sus revisiones exactas y el motivo requerido.',
};
export const resourceActivityFailure = (error = {}) =>
  error.status === 413
    ? 'La solicitud supera el l\u00edmite de 16 KiB.'
    : messages[error.code] || resourceFailure(error);
