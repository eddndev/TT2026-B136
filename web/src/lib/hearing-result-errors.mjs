const messages = {
  hearing_result_revision_conflict:
    'El registro cambi\u00f3. Conserva el borrador y consulta la base actual.',
  hearing_result_already_withdrawn: 'El registro ya est\u00e1 retirado. Consulta su historia.',
  hearing_result_operation_conflict:
    'La operaci\u00f3n no se pudo registrar. Consulta la revisi\u00f3n exacta del env\u00edo.',
  hearing_result_submission_mismatch:
    'La preparaci\u00f3n no coincide con el env\u00edo. Revisa el borrador y prepara de nuevo.',
  hearing_result_support_changed:
    'El soporte cambi\u00f3. Selecciona y consulta de nuevo la versi\u00f3n exacta.',
  hearing_result_revision_exhausted:
    'Se agotaron las revisiones de este registro. Su historia sigue disponible.',
  hearing_result_future_time:
    'La fecha declarada est\u00e1 completamente en el futuro. Revisa fecha, hora y desfase.',
  hearing_result_invalid_reference:
    'Revisa la programaci\u00f3n y el antecedente exactos seleccionados.',
  hearing_result_not_found: 'No se encontr\u00f3 el registro o la revisi\u00f3n consultada.',
  hearing_result_reference_not_found:
    'Una referencia exacta no est\u00e1 disponible en este expediente. Revisa la selecci\u00f3n.',
  invalid_hearing_result_value:
    'Revisa el relato, tiempo declarado, comparecencias, acuerdos y procedencia.',
  invalid_hearing_result_revision: 'La revisi\u00f3n del registro no es v\u00e1lida.',
  hearing_result_support_too_large: 'El soporte supera el l\u00edmite de 16 MiB.',
  hearing_result_support_format_rejected: 'El soporte no cumple el formato PDF o DOCX admitido.',
  hearing_result_support_validation_limit:
    'El soporte supera los l\u00edmites de validaci\u00f3n del formato.',
  hearing_result_support_digest_mismatch:
    'La huella no coincide con el soporte. Vuelve a consultar su versi\u00f3n exacta.',
};
export function hearingResultFailure(error) {
  if (error.status === 413) return 'El registro supera el l\u00edmite admitido de 512 KiB.';
  return messages[error.code] || error.message || 'No se pudo completar la consulta de resultados.';
}
export const resultOccurrence = { occurred: 'Se inici\u00f3', not_started: 'No se inici\u00f3' };
export const resultExtent = {
  partial: 'Parcial',
  concluded: 'Concluido',
  unspecified: 'No consta',
};
export const resultStatus = { recorded: 'Registrado', withdrawn: 'Registro retirado' };
export const resultSource = {
  operator_note: 'Nota de trabajo',
  oral_reference: 'Referencia oral',
  written_record: 'Registro escrito',
};
