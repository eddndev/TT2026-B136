import { factFailure, factDenied, factUncertain, canFacts } from './procedural-fact-errors.mjs';
export const resourceDenied = factDenied;
export const resourceUncertain = factUncertain;
export const canResources = canFacts;
const messages = {
  procedural_resource_not_found:
    'El recurso, acto o revisi\u00f3n no est\u00e1 disponible. Consulta de nuevo su historia.',
  procedural_resource_revision_conflict:
    'El recurso cambi\u00f3. Conserva tu borrador y consulta la revisi\u00f3n actual.',
  procedural_resource_operation_conflict:
    'El env\u00edo ya tiene un resultado. Consulta su revisi\u00f3n exacta.',
  procedural_resource_archived: 'El recurso est\u00e1 archivado. Su historia permanece disponible.',
  procedural_resource_state_unchanged: 'El recurso ya tiene el estado solicitado.',
  procedural_resource_submission_mismatch:
    'La preparaci\u00f3n cambi\u00f3. Revisa los datos y prepara de nuevo.',
  invalid_procedural_resource: 'Revisa los valores, personas y soportes exactos del recurso.',
};
export const resourceFailure = (error = {}) => messages[error.code] || factFailure(error);
