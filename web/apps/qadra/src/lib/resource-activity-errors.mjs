import { resourceFailure } from './procedural-resource-errors.mjs';
export { resourceDenied, resourceUncertain } from './procedural-resource-errors.mjs';
const messages = {
  resource_activity_not_found: 'El vinculo o su revision no esta disponible.',
  resource_activity_revision_conflict:
    'El vinculo cambio. Conserva tu borrador y compara el registro actual.',
  resource_activity_resource_revision_conflict:
    'El recurso cambio. Conserva la seleccion exacta y compara su cabeza actual.',
  resource_activity_operation_conflict:
    'El envio ya tiene un resultado. Consulta su recibo exacto.',
  resource_activity_resource_archived:
    'El recurso esta archivado. Puedes consultar o desvincular actividades existentes.',
  resource_activity_state_unchanged:
    'El vinculo ya esta desvinculado. Su historia permanece disponible.',
  resource_activity_source_mismatch:
    'Una fuente exacta no coincide. Consulta y selecciona de nuevo esa fuente.',
  resource_activity_submission_mismatch:
    'La preparacion cambio. Conserva la seleccion y revisa de nuevo.',
  invalid_resource_activity: 'Revisa la actividad, sus revisiones exactas y el motivo requerido.',
};
export const resourceActivityFailure = (error = {}) =>
  error.status === 413
    ? 'La solicitud supera el limite de 16 KiB.'
    : messages[error.code] || resourceFailure(error);
