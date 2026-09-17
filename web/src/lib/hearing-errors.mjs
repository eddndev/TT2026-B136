const messages = {
  hearing_revision_conflict:
    'La audiencia cambi\u00f3. Conserva tu borrador y consulta el registro actual.',
  hearing_context_conflict:
    'La administraci\u00f3n o etapa cambi\u00f3. Consulta el contexto y revisa tu borrador.',
  hearing_participant_changed:
    'Una selecci\u00f3n de participante cambi\u00f3. Revisa las fichas elegidas y selecciona sus revisiones admitidas.',
  hearing_already_cancelled:
    'La audiencia ya est\u00e1 cancelada. Consulta su registro e historial.',
  hearing_operation_conflict:
    'La operaci\u00f3n no se pudo registrar. Consulta la revisi\u00f3n del env\u00edo antes de decidir otro intento.',
  hearing_submission_mismatch:
    'La preparaci\u00f3n no coincide con el env\u00edo. Consulta el contexto y prepara de nuevo el borrador.',
  hearing_support_changed:
    'El soporte cambi\u00f3. Selecciona y consulta de nuevo su versi\u00f3n exacta.',
  hearing_revision_exhausted:
    'Se agotaron las revisiones de esta audiencia. Puedes consultar su historial.',
  hearing_context_required: 'Completa la ficha penal y registra la etapa antes de programar.',
  hearing_stage_incompatible: 'El tipo de audiencia no corresponde a la etapa consultada.',
  hearing_immutable_kind: 'El tipo de una audiencia no se puede cambiar.',
  invalid_hearing_value: 'Revisa los datos, participantes y soporte de la audiencia.',
  invalid_hearing_revision: 'La revisi\u00f3n de audiencia no es v\u00e1lida.',
  hearing_not_found: 'No se encontr\u00f3 la audiencia o revisi\u00f3n consultada.',
  hearing_support_digest_mismatch:
    'El digest no coincide con el soporte. Vuelve a elegir su versi\u00f3n exacta.',
  hearing_support_too_large: 'El soporte supera el l\u00edmite admitido de 16 MiB.',
  hearing_support_format_rejected: 'El soporte no cumple el formato PDF o DOCX admitido.',
  hearing_support_validation_limit:
    'El soporte supera los l\u00edmites de validaci\u00f3n del formato.',
};
export function hearingFailure(error) {
  if (messages[error.code]) return messages[error.code];
  if (error.status === 413) return 'El registro supera el l\u00edmite admitido de 64 KiB.';
  return error.message || 'No se pudo completar la consulta de audiencias.';
}
